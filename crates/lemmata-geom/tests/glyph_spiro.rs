//! Replays glyph skeleton knots captured from a real Iosevka build.
//!
//! `corpus/glyph-spiro.json` holds knot sets that the Iosevka build passed to
//! its Spiro geometry, and `golden/glyph-spiro.json` holds the Bezier output it
//! produced. The test measures how far our evaluation lands from that output.

use std::fs;
use std::path::{Path, PathBuf};

use kurbo::{BezPath, PathEl, Point};
use lemmata_geom::{Knot, KnotType, spiro_path};
use serde_json::Value;

/// Subdivision delta for our evaluation.
const DELTA: f64 = 0.01;
/// Flattening tolerance for the comparison, well below the drift budget.
const FLATTEN: f64 = 0.005;
/// Deviation budget in font units.
const TOLERANCE: f64 = 1.0;

fn workspace_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
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

fn number(value: &Value) -> f64 {
    value.as_f64().expect("number")
}

fn point(value: &Value) -> Point {
    Point::new(number(&value["x"]), number(&value["y"]))
}

fn flatten(path: &BezPath) -> Vec<Point> {
    let mut points = Vec::new();
    kurbo::flatten(path.iter(), FLATTEN, |element| match element {
        PathEl::MoveTo(p) | PathEl::LineTo(p) => points.push(p),
        PathEl::QuadTo(..) | PathEl::CurveTo(..) | PathEl::ClosePath => {}
    });

    points
}

/// Largest distance from a point of `a` to the nearest point of `b`.
fn directed(a: &[Point], b: &[Point]) -> f64 {
    let mut worst: f64 = 0.0;
    for p in a {
        let mut nearest = f64::INFINITY;
        for q in b {
            let distance = (*p - *q).hypot();
            if distance < nearest {
                nearest = distance;
            }
        }

        if nearest > worst {
            worst = nearest;
        }
    }

    worst
}

fn max_deviation(a: &[Point], b: &[Point]) -> f64 {
    directed(a, b).max(directed(b, a))
}

fn shape_from_capture(contours: &[Value]) -> BezPath {
    let mut path = BezPath::new();
    for contour in contours {
        for (index, arc) in contour.as_array().expect("contour").iter().enumerate() {
            let start = point(&arc["a"]);
            if index == 0 {
                path.move_to(start);
            }
            path.curve_to(point(&arc["b"]), point(&arc["c"]), point(&arc["d"]));
        }
        path.close_path();
    }

    path
}

#[test]
fn captured_glyph_skeletons_match() {
    let corpus: Value = serde_json::from_str(
        &fs::read_to_string(workspace_path("corpus/glyph-spiro.json")).unwrap(),
    )
    .unwrap();
    let golden: Value = serde_json::from_str(
        &fs::read_to_string(workspace_path("golden/glyph-spiro.json")).unwrap(),
    )
    .unwrap();

    let cases = corpus["cases"].as_array().unwrap();
    let expected_cases = golden["cases"].as_array().unwrap();
    assert_eq!(cases.len(), expected_cases.len());

    let mut worst = 0.0_f64;
    for (case, expected) in cases.iter().zip(expected_cases) {
        let name = case["name"].as_str().unwrap();
        let closed = case["closed"].as_bool().unwrap();
        let knots: Vec<Knot> = case["knots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| {
                Knot::new(
                    number(&k["x"]),
                    number(&k["y"]),
                    knot_type(k["type"].as_str().unwrap()),
                )
            })
            .collect();

        let ours = flatten(&spiro_path(&knots, closed, DELTA));
        let theirs = flatten(&shape_from_capture(expected["output"].as_array().unwrap()));
        let deviation = max_deviation(&ours, &theirs);
        worst = worst.max(deviation);
        assert!(
            deviation <= TOLERANCE,
            "{name}: deviation {deviation} over budget"
        );
    }
    eprintln!(
        "captured glyph skeletons: {} cases, worst deviation {worst} font units",
        cases.len()
    );
}
