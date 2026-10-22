//! Compares the ported lowercase w against the build's own.
//!
//! The plan selects the straight serifless form, which is what the capture
//! holds.
//!
//! Skips without a capture.

#![allow(clippy::cast_precision_loss)]

use std::fs;

use lemmata_capture::{dump_path, metrics_from_capture};
use lemmata_geom::{BiKnot, Geometry};
use lemmata_glyphs::letter::latin::lower_w::LowerW;
use lemmata_ptl::{shapes::Design, spiro_kit::Kit};
use serde_json::Value;

const TOLERANCE: f64 = 1e-6;
const BOX_TOLERANCE: f64 = 1.0;

struct OurNode {
    contrast: f64,
    closed: bool,
    knots: Vec<BiKnot>,
}

fn collect_spiro<'a>(geometry: &'a Geometry, out: &mut Vec<&'a Geometry>) {
    match geometry {
        Geometry::Spiro { .. } => out.push(geometry),
        Geometry::Combine(parts) => {
            for part in parts {
                collect_spiro(part, out);
            }
        }
        Geometry::Boolean { operands, .. } => {
            for operand in operands {
                collect_spiro(operand, out);
            }
        }
        Geometry::Transformed { geom, .. }
        | Geometry::RemoveHoles { geom, .. }
        | Geometry::Simplify { geom, .. }
        | Geometry::Stroke { geom, .. }
        | Geometry::Tagged(geom)
        | Geometry::Radical(geom) => collect_spiro(geom, out),
        _ => {}
    }
}

fn node_of(geometry: &Geometry) -> OurNode {
    let Geometry::DiSpiro {
        contrast,
        closed,
        knots,
        ..
    } = geometry
    else {
        panic!("expected a di-spiro node");
    };

    OurNode {
        contrast: *contrast,
        closed: *closed,
        knots: knots.clone(),
    }
}

fn bounding_box(points: impl Iterator<Item = (f64, f64)>) -> (f64, f64, f64, f64) {
    points.fold(
        (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |acc, (x, y)| (acc.0.min(x), acc.1.min(y), acc.2.max(x), acc.3.max(y)),
    )
}

fn entry<'a>(entries: &'a [Value], name: &str) -> Option<&'a Value> {
    entries.iter().find(|entry| entry["name"] == name)
}

#[test]
fn lower_w_matches_capture() {
    let Some(metrics) = metrics_from_capture() else {
        eprintln!("skipping: no capture");
        return;
    };
    let Ok(data) = fs::read(dump_path()) else {
        eprintln!("skipping: no capture");
        return;
    };
    let read = |suffix: &str| -> Value {
        let name = if suffix.is_empty() {
            format!("{}.json", dump_path().display())
        } else {
            format!("{}.{suffix}.json", dump_path().display())
        };
        serde_json::from_str(
            &fs::read_to_string(&name).unwrap_or_else(|error| panic!("reading {name}: {error}")),
        )
        .expect("parse side data")
    };
    let expansions = read("expansions");
    let expansions = expansions.as_array().expect("array");
    let contours = read("");
    let contours = contours.as_array().expect("array");

    let glyphs = lemmata_capture::load(&data).expect("decode capture");
    let design = Design::default();
    let kit = Kit::new(
        metrics.stroke,
        design.superness,
        metrics.contrast,
        metrics.tiny,
    );
    let body = LowerW {
        metrics: &metrics,
        kit: &kit,
        design: &design,
    };

    let cases: Vec<(String, Geometry)> = body.glyphs();

    let mut worst = (0.0_f64, "none".to_string());
    let mut knot_count = 0usize;
    let mut compared = 0usize;
    for (name, ours) in &cases {
        let captured_name = match name.as_str() {
            "w.straightSerifless" => "w",
            _ => continue,
        };
        let Some(glyph) = glyphs.iter().find(|glyph| glyph.name == captured_name) else {
            continue;
        };
        compared += 1;
        let mut captured = Vec::new();
        lemmata_capture::collect_di_spiro(&glyph.geometry, &mut captured);
        let mut our_nodes = Vec::new();
        lemmata_capture::collect_di_spiro(ours, &mut our_nodes);
        assert_eq!(captured.len(), our_nodes.len(), "{name}: dispiro count");

        for (index, (node, expected)) in our_nodes.iter().zip(&captured).enumerate() {
            let ours = node_of(node);
            let expected = node_of(expected);
            assert!(
                (ours.contrast - expected.contrast).abs() < 1e-9,
                "{name} {index}: contrast {} vs {}",
                ours.contrast,
                expected.contrast
            );
            assert_eq!(ours.closed, expected.closed, "{name} {index}: closed");
            assert_eq!(
                ours.knots.len(),
                expected.knots.len(),
                "{name} {index}: knot count"
            );
            knot_count += ours.knots.len();

            for (knot, expected_knot) in ours.knots.iter().zip(&expected.knots) {
                let deviation = (knot.x - expected_knot.x)
                    .abs()
                    .max((knot.y - expected_knot.y).abs());
                if deviation > worst.0 {
                    worst = (
                        deviation,
                        format!("{name} {index} at ({}, {})", knot.x, knot.y),
                    );
                }
                assert!(
                    deviation <= TOLERANCE,
                    "{name} {index}: knot at ({}, {}) is off by {deviation}",
                    knot.x,
                    knot.y
                );
                assert_eq!(
                    knot.ty, expected_knot.ty,
                    "{name} {index}: knot type at ({}, {})",
                    knot.x, knot.y
                );
                assert_eq!(
                    knot.unimportant, expected_knot.unimportant,
                    "{name} {index}: importance at ({}, {})",
                    knot.x, knot.y
                );
                match (&knot.proposed_normal, &expected_knot.proposed_normal) {
                    (Some(a), Some(b)) => {
                        assert!(
                            (a.x - b.x).abs().max((a.y - b.y).abs()) <= 1e-9,
                            "{name} {index}: normal at ({}, {}) is ({}, {}), not ({}, {})",
                            knot.x,
                            knot.y,
                            a.x,
                            a.y,
                            b.x,
                            b.y
                        )
                    }
                    (None, None) => {}
                    _ => {
                        panic!(
                            "{name} {index}: one side has a normal at ({}, {}) and the other does not",
                            knot.x, knot.y
                        )
                    }
                }
            }
        }

        let mut captured_spiros = Vec::new();
        collect_spiro(&glyph.geometry, &mut captured_spiros);
        let mut our_spiros = Vec::new();
        collect_spiro(ours, &mut our_spiros);
        assert_eq!(
            captured_spiros.len(),
            our_spiros.len(),
            "{name}: spiro count"
        );
        for (index, (node, expected)) in our_spiros.iter().zip(&captured_spiros).enumerate() {
            let Geometry::Spiro { closed, knots, .. } = node else {
                unreachable!();
            };
            let Geometry::Spiro {
                closed: expected_closed,
                knots: expected_knots,
                ..
            } = expected
            else {
                unreachable!();
            };
            assert_eq!(closed, expected_closed, "{name} spiro {index}: closed");
            assert_eq!(
                knots.len(),
                expected_knots.len(),
                "{name} spiro {index}: knot count"
            );
            for (knot, expected_knot) in knots.iter().zip(expected_knots) {
                assert_eq!(
                    knot.ty, expected_knot.ty,
                    "{name} spiro {index}: knot type at ({}, {})",
                    knot.x, knot.y
                );
                let deviation = (knot.x - expected_knot.x)
                    .abs()
                    .max((knot.y - expected_knot.y).abs());
                assert!(
                    deviation <= TOLERANCE,
                    "{name} spiro {index}: knot at ({}, {}) is off by {deviation}",
                    knot.x,
                    knot.y
                );
            }
        }

        let expected_expansions = entry(expansions, captured_name)
            .unwrap_or_else(|| panic!("{name} is not in the expansions"))["expansions"]
            .as_array()
            .expect("array");
        assert_eq!(
            expected_expansions.len(),
            our_nodes.len(),
            "{name}: expansion count"
        );
        for (index, (node, expected)) in our_nodes.iter().zip(expected_expansions).enumerate() {
            let ours = node_of(node);
            let result = lemmata_geom::di_spiro_expand(
                kurbo::Affine::IDENTITY,
                ours.contrast,
                ours.closed,
                &ours.knots,
            );

            for (side, side_knots) in [
                ("lhs", &result.lhs_untransformed),
                ("rhs", &result.rhs_untransformed),
            ] {
                let theirs = expected[side].as_array().unwrap();
                assert_eq!(
                    side_knots.len(),
                    theirs.len(),
                    "{name} {index}: {side} knot count"
                );

                for (knot, expected_knot) in side_knots.iter().zip(theirs) {
                    let deviation = (knot.x - expected_knot[0].as_f64().unwrap())
                        .abs()
                        .max((knot.y - expected_knot[1].as_f64().unwrap()).abs());
                    assert!(
                        deviation <= TOLERANCE,
                        "{name} {index}: {side} knot at ({}, {}) is off by {deviation}",
                        knot.x,
                        knot.y
                    );
                }
            }
        }

        let their_box = bounding_box(
            entry(contours, captured_name)
                .unwrap_or_else(|| panic!("{name} is not in the contours"))["contours"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|contour| contour.as_array().unwrap())
                .map(|point| (point["x"].as_f64().unwrap(), point["y"].as_f64().unwrap())),
        );
        let our_box = bounding_box(
            Geometry::Simplify {
                gizmo: kurbo::Affine::IDENTITY,
                geom: Box::new(ours.clone()),
            }
            .to_contours()
            .into_iter()
            .flatten()
            .map(|point| (point.x, point.y)),
        );

        let their_points: usize = entry(contours, captured_name).unwrap()["contours"]
            .as_array()
            .unwrap()
            .iter()
            .map(|contour| contour.as_array().unwrap().len())
            .sum();
        let our_points: usize = Geometry::Simplify {
            gizmo: kurbo::Affine::IDENTITY,
            geom: Box::new(ours.clone()),
        }
        .to_contours()
        .iter()
        .map(Vec::len)
        .sum();
        // A boolean cut can quadify its seam into a point or two either way.
        // A structural mistake moves the count by tens of points.
        assert!(
            our_points.abs_diff(their_points) <= 4,
            "{name}: outline point count {our_points} vs {their_points}"
        );

        for (side, (our, their)) in [
            ("left", (our_box.0, their_box.0)),
            ("bottom", (our_box.1, their_box.1)),
            ("right", (our_box.2, their_box.2)),
            ("top", (our_box.3, their_box.3)),
        ] {
            assert!(
                (our - their).abs() <= BOX_TOLERANCE,
                "{name}: outline {side} is {our}, not {their}"
            );
        }
    }

    assert!(compared > 0, "the capture holds no lowercase w");
    eprintln!(
        "lower w: {compared} of {} cases compared, {knot_count} knots, worst deviation {:.9} on {}",
        cases.len(),
        worst.0,
        worst.1
    );
}
