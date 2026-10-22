//! Compares the ported composition layer against Iosevka's.
//!
//! `corpus/glyph-compose.json` describes sequences of includes, transforms,
//! boolean combinations and tag ejections; the golden carries the arcs
//! upstream's `Glyph` and `GlyphProc` resolve them to. The comparison is
//! shape-based, like the tree test, so a difference in how the two engines
//! split contours does not count.

#![allow(clippy::cast_precision_loss)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use kurbo::{Affine, BezPath, ParamCurve, ParamCurveNearest, Point, Shape};
use lemmata_geom::{BiKnot, Geometry, KnotType, MonoKnot};
use lemmata_ptl::glyph::{Glyph, GlyphProc, Operation};
use linesweeper::BinaryOp;
use serde_json::Value;

/// Samples per segment.
const SAMPLES_PER_SEGMENT: usize = 16;
/// Boundary tolerance, in font units. The two engines resolve overlaps
/// differently, so a boolean case can part by a fraction of a unit.
const TOLERANCE: f64 = 0.5;

fn workspace_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn number(value: &Value) -> f64 {
    value.as_f64().expect("number")
}

fn affine(value: Option<&Value>) -> Affine {
    match value {
        None => Affine::IDENTITY,
        Some(t) => {
            Affine::new([
                number(&t["xx"]),
                number(&t["yx"]),
                number(&t["xy"]),
                number(&t["yy"]),
                number(&t["tx"]),
                number(&t["ty"]),
            ])
        }
    }
}

fn knot_type(name: &str) -> KnotType {
    match name {
        "open" => KnotType::Open,
        "open_end" => KnotType::OpenEnd,
        "left" => KnotType::Left,
        "right" => KnotType::Right,
        "g2" => KnotType::G2,
        "g4" => KnotType::G4,
        _ => KnotType::Corner,
    }
}

fn geometry(value: &Value) -> Geometry {
    match value["kind"].as_str().expect("kind") {
        "spiro" => {
            Geometry::Spiro {
                gizmo: affine(value.get("gizmo")),
                closed: value["closed"].as_bool().unwrap(),
                knots: value["knots"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|k| {
                        MonoKnot::new(
                            knot_type(k["type"].as_str().unwrap()),
                            number(&k["x"]),
                            number(&k["y"]),
                            k["unimportant"].as_bool().unwrap_or(false),
                        )
                    })
                    .collect(),
            }
        }
        "diSpiro" => {
            Geometry::DiSpiro {
                gizmo: affine(value.get("gizmo")),
                contrast: number(&value["contrast"]),
                closed: value["closed"].as_bool().unwrap(),
                knots: value["knots"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|k| {
                        let mut knot = BiKnot::new(
                            knot_type(k["type"].as_str().unwrap()),
                            number(&k["x"]),
                            number(&k["y"]),
                            number(&k["d1"]),
                            number(&k["d2"]),
                        );
                        knot.unimportant = k["unimportant"].as_bool().unwrap_or(false);
                        if let Some(normal) = k.get("proposedNormal") {
                            knot.proposed_normal =
                                Some(kurbo::Vec2::new(number(&normal["x"]), number(&normal["y"])));
                        }

                        knot
                    })
                    .collect(),
            }
        }
        "combine" => {
            Geometry::Combine(
                value["parts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(geometry)
                    .collect(),
            )
        }
        "boolean" => {
            Geometry::Boolean {
                op: match value["op"].as_str().unwrap() {
                    "intersection" => BinaryOp::Intersection,
                    "difference" => BinaryOp::Difference,
                    "xor" => BinaryOp::Xor,
                    _ => BinaryOp::Union,
                },
                operands: value["operands"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(geometry)
                    .collect(),
            }
        }
        "transform" => {
            Geometry::Transformed {
                tf: affine(value.get("tf")),
                geom: Box::new(geometry(&value["geom"])),
            }
        }
        other => panic!("unknown geometry {other}"),
    }
}

fn operations(value: &Value) -> Vec<Operation> {
    value["ops"]
        .as_array()
        .unwrap()
        .iter()
        .map(|op| {
            match op["op"].as_str().expect("op") {
                "includeGeometry" => Operation::IncludeGeometry(geometry(&op["geom"])),
                "includeProc" => {
                    Operation::IncludeProc(Box::new(GlyphProc::new(operations(&op["proc"]))))
                }
                "includeGlyph" => Operation::IncludeGlyph(Box::new(glyph(&op["glyph"]))),
                "applyTransform" => Operation::ApplyTransform(affine(op.get("tf"))),
                "setWidth" => Operation::SetWidth(number(&op["w"])),
                "ctxTag" => Operation::CtxTag(op["tag"].as_str().unwrap().to_string()),
                "ejectTagged" => Operation::EjectTagged(op["tag"].as_str().unwrap().to_string()),
                "boole" => {
                    Operation::Boole {
                        operator: match op["operator"].as_str().unwrap() {
                            "intersection" => BinaryOp::Intersection,
                            "difference" => BinaryOp::Difference,
                            "xor" => BinaryOp::Xor,
                            _ => BinaryOp::Union,
                        },
                        operands: op["operands"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(glyph)
                            .collect(),
                    }
                }
                other => panic!("unknown op {other}"),
            }
        })
        .collect()
}

/// Builds a glyph, applying the context tag where the corpus sets one.
fn glyph(value: &Value) -> Glyph {
    let mut glyph = match value.get("identifier").and_then(Value::as_str) {
        Some(identifier) => Glyph::with_identifier(identifier),
        None => Glyph::new(),
    };
    glyph.include(&GlyphProc::new(operations(value)), false, false);

    glyph
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
    for contour in contours.as_array().unwrap() {
        let arcs = contour.as_array().unwrap();
        let Some(first) = arcs.first() else {
            continue;
        };
        let p = |v: &Value| Point::new(number(&v["x"]), number(&v["y"]));
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
fn composition_matches_upstream() {
    let corpus: Value = serde_json::from_str(
        &fs::read_to_string(workspace_path("corpus/glyph-compose.json")).unwrap(),
    )
    .unwrap();
    let golden: Value = serde_json::from_str(
        &fs::read_to_string(workspace_path("golden/glyph-compose.json")).unwrap(),
    )
    .unwrap();

    let cases = corpus["cases"].as_array().unwrap();
    let expected_cases = golden["cases"].as_array().unwrap();
    assert_eq!(cases.len(), expected_cases.len());

    let mut worst = (0.0_f64, String::new());
    for (case, expected) in cases.iter().zip(expected_cases) {
        let name = case["name"].as_str().unwrap();
        let built = glyph(case);
        assert_eq!(
            built.advance_width,
            number(&expected["width"]),
            "{name}: advance width"
        );
        let ours = path_from_geometry(&built.geometry);
        let theirs = path_from_json(&expected["arcs"]);
        let deviation = directed(&sample(&ours), &theirs).max(directed(&sample(&theirs), &ours));
        if deviation > worst.0 {
            worst = (deviation, name.to_string());
        }
        assert!(
            deviation <= TOLERANCE,
            "{name}: deviation {deviation} over {TOLERANCE}"
        );
    }
    eprintln!(
        "glyph composition: {} cases, worst deviation {:.3} on {}",
        cases.len(),
        worst.0,
        worst.1
    );
}
