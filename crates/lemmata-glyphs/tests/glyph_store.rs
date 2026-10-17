//! Compares the ported collection path against Iosevka's.
//!
//! `corpus/glyph-store.json` runs a sequence of saves through upstream's
//! `GlyphSaveSink` and `GlyphStore`: named glyphs, bare code points, an
//! anonymous one, a save the build filter skips, a forced save, and a duplicate
//! name. `golden/glyph-store.json` carries what upstream filed. The test replays
//! the same sequence through the port and compares the names, code points,
//! widths and outlines.

#![allow(clippy::cast_precision_loss)]

use std::fs;
use std::path::{Path, PathBuf};

use kurbo::{Affine, BezPath, ParamCurve, ParamCurveNearest, Point, Shape};
use lemmata_geom::{Geometry, KnotType, MonoKnot};
use lemmata_glyphs::glyph::{GlyphProc, Operation};
use lemmata_glyphs::store::{GlyphSaveSink, GlyphTarget};
use serde_json::Value;

/// Samples per segment.
const SAMPLES_PER_SEGMENT: usize = 16;
/// Boundary tolerance, in font units.
const TOLERANCE: f64 = 0.5;

fn workspace_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn number(value: &Value) -> f64 {
    value.as_f64().expect("number")
}

fn geometry(value: &Value) -> Geometry {
    match value["kind"].as_str().expect("kind") {
        "spiro" => Geometry::Spiro {
            gizmo: Affine::IDENTITY,
            closed: value["closed"].as_bool().unwrap(),
            knots: value["knots"]
                .as_array()
                .unwrap()
                .iter()
                .map(|k| {
                    MonoKnot::new(
                        KnotType::G4,
                        number(&k["x"]),
                        number(&k["y"]),
                        k["unimportant"].as_bool().unwrap_or(false),
                    )
                })
                .collect(),
        },
        "combine" => Geometry::Combine(
            value["parts"]
                .as_array()
                .unwrap()
                .iter()
                .map(geometry)
                .collect(),
        ),
        other => panic!("unknown geometry {other}"),
    }
}

fn contents(value: &Value) -> GlyphProc {
    GlyphProc::new(
        value["parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|part| Operation::IncludeGeometry(geometry(part)))
            .collect(),
    )
}

fn path_from_geometry(geometry: &Geometry) -> BezPath {
    let mut path = BezPath::new();
    for contour in geometry.to_bez_arcs() {
        let Some(first) = contour.first() else {
            continue;
        };
        path.move_to(Point::new(first.a.x, first.a.y));
        for arc in &contour {
            path.curve_to(
                Point::new(arc.b.x, arc.b.y),
                Point::new(arc.c.x, arc.c.y),
                Point::new(arc.d.x, arc.d.y),
            );
        }
        path.close_path();
    }
    path
}

fn path_from_json(contours: &Value) -> BezPath {
    let mut path = BezPath::new();
    let p = |v: &Value| Point::new(number(&v["x"]), number(&v["y"]));
    for contour in contours.as_array().unwrap() {
        let arcs = contour.as_array().unwrap();
        let Some(first) = arcs.first() else {
            continue;
        };
        path.move_to(p(&first["a"]));
        for arc in arcs {
            path.curve_to(p(&arc["b"]), p(&arc["c"]), p(&arc["d"]));
        }
        path.close_path();
    }
    path
}

fn sample(path: &BezPath) -> Vec<Point> {
    let mut out = Vec::new();
    for segment in path.segments() {
        for index in 0..=SAMPLES_PER_SEGMENT {
            out.push(segment.eval(index as f64 / SAMPLES_PER_SEGMENT as f64));
        }
    }
    out
}

fn directed(samples: &[Point], path: &BezPath) -> f64 {
    let segments: Vec<_> = path.segments().collect();
    let mut worst: f64 = 0.0;
    for p in samples {
        let mut nearest = f64::INFINITY;
        for segment in &segments {
            nearest = nearest.min(segment.nearest(*p, 1e-3).distance_sq.sqrt());
        }
        if nearest > worst && !path.contains(*p) {
            worst = nearest;
        }
    }
    worst
}

#[test]
fn store_matches_upstream() {
    let corpus: Value = serde_json::from_str(
        &fs::read_to_string(workspace_path("corpus/glyph-store.json")).unwrap(),
    )
    .unwrap();
    let golden: Value = serde_json::from_str(
        &fs::read_to_string(workspace_path("golden/glyph-store.json")).unwrap(),
    )
    .unwrap();

    let transform = &corpus["transform"];
    let mut sink = GlyphSaveSink::new(
        500.0,
        Affine::new([
            number(&transform["xx"]),
            number(&transform["yx"]),
            number(&transform["xy"]),
            number(&transform["yy"]),
            number(&transform["tx"]),
            number(&transform["ty"]),
        ]),
    );
    if let Some(needed) = corpus["needed"].as_array() {
        sink.restrict_to(
            needed
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect(),
        );
    }

    let saves = corpus["saves"].as_array().unwrap();
    let expected_saves = golden["saves"].as_array().unwrap();
    assert_eq!(saves.len(), expected_saves.len());

    let mut worst = (0.0_f64, String::new());
    for (save, expected) in saves.iter().zip(expected_saves) {
        let label = save["name"].as_str().unwrap();
        let target = match save["target"].as_str().unwrap() {
            "anonymous" => GlyphTarget::Anonymous,
            "codePoint" => {
                GlyphTarget::CodePoint(u32::try_from(save["code"].as_u64().unwrap()).unwrap())
            }
            _ => GlyphTarget::Named {
                name: save["name"].as_str().unwrap().to_string(),
                unicode: u32::try_from(save["unicode"].as_u64().unwrap_or(0)).unwrap(),
            },
        };
        let force = save["force"].as_bool().unwrap_or(false);
        let result = if force {
            sink.save_forced(&target, &contents(save))
        } else {
            sink.save(&target, &contents(save))
        };

        // Upstream records the message it threw; the port records whether it
        // threw. The text is the same, but matching on it would tie the test to
        // wording rather than behaviour.
        assert_eq!(
            result.is_err(),
            !expected["error"].is_null(),
            "{label}: whether the save failed"
        );
        if let Ok(saved) = result {
            let filed = &expected["saved"];
            match (saved, filed) {
                (Some(index), Value::Object(_)) => {
                    assert_eq!(
                        sink.store.glyphs()[index].advance_width,
                        number(&filed["width"]),
                        "{label}: saved width"
                    );
                    assert_eq!(
                        sink.store.glyphs()[index].identifier.as_deref(),
                        filed["name"].as_str(),
                        "{label}: saved name"
                    );
                }
                (None, Value::Null) => {}
                (actual, wanted) => {
                    panic!("{label}: saved {actual:?}, expected {wanted}");
                }
            }
        }
    }

    assert_eq!(
        sink.store.size(),
        usize::try_from(golden["size"].as_u64().unwrap()).unwrap(),
        "named glyph count"
    );

    let entries = golden["entries"].as_array().unwrap();
    let ours = sink.store.named_entries();
    assert_eq!(ours.len(), entries.len(), "named entry count");
    for ((name, index), wanted) in ours.iter().zip(entries) {
        assert_eq!(name, wanted["name"].as_str().unwrap(), "entry name");
        let unicodes: Vec<u64> = sink
            .store
            .unicode_of(*index)
            .unwrap_or_default()
            .iter()
            .map(|code| u64::from(*code))
            .collect();
        let expected_codes: Vec<u64> = wanted["unicodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect();
        assert_eq!(unicodes, expected_codes, "{name}: code points");
        assert_eq!(
            sink.store.glyphs()[*index].advance_width,
            number(&wanted["width"]),
            "{name}: width"
        );

        let our_path = path_from_geometry(&sink.store.glyphs()[*index].geometry);
        let their_path = path_from_json(&wanted["arcs"]);
        let deviation = directed(&sample(&our_path), &their_path)
            .max(directed(&sample(&their_path), &our_path));
        if deviation > worst.0 {
            worst = (deviation, name.clone());
        }
        assert!(
            deviation <= TOLERANCE,
            "{name}: deviation {deviation} over {TOLERANCE}"
        );
    }
    eprintln!(
        "glyph store: {} saves, {} entries, worst deviation {:.3} on {}",
        saves.len(),
        ours.len(),
        worst.0,
        worst.1
    );
}
