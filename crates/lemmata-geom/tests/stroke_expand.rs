//! Compares the Rust stroke expansion against Iosevka's.
//!
//! The fitted offsets must match; the unioned output is compared loosely because
//! the two boolean engines differ.

#![allow(clippy::cast_precision_loss)]

use std::fs;
use std::path::{Path, PathBuf};

use kurbo::{BezPath, ParamCurve, ParamCurveNearest, PathEl, Point};
use lemmata_geom::{Bez3, stroke_arcs, stroke_offsets};
use serde_json::Value;

/// Coordinate tolerance for the fitted offsets.
const OFFSET_TOLERANCE: f64 = 1e-6;
/// Deviation budget for the unioned shape. The two boolean engines agree to a
/// few 1e-5 on these cases.
const UNION_TOLERANCE: f64 = 1e-3;
/// Samples per curve for the shape comparison.
const SAMPLES_PER_CURVE: usize = 16;

fn workspace_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn number(value: &Value) -> f64 {
    value.as_f64().expect("number")
}

fn point(value: &Value) -> Point {
    Point::new(number(&value["x"]), number(&value["y"]))
}

fn contours_from(case: &Value) -> Vec<Vec<Bez3>> {
    case["contours"]
        .as_array()
        .unwrap()
        .iter()
        .map(|contour| {
            contour
                .as_array()
                .unwrap()
                .iter()
                .map(|arc| {
                    Bez3::new(
                        point(&arc["a"]),
                        point(&arc["b"]),
                        point(&arc["c"]),
                        point(&arc["d"]),
                    )
                })
                .collect()
        })
        .collect()
}

/// Path points in shape order: move, then each cubic's controls and end.
fn path_points(path: &BezPath) -> Vec<Point> {
    let mut points = Vec::new();
    for element in path.elements() {
        match element {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => points.push(*p),
            PathEl::CurveTo(c1, c2, end) => {
                points.push(*c1);
                points.push(*c2);
                points.push(*end);
            }
            PathEl::QuadTo(..) | PathEl::ClosePath => {}
        }
    }
    points
}

fn golden_points(shape: &Value) -> Vec<Point> {
    let mut points = Vec::new();
    for contour in shape.as_array().unwrap() {
        for (index, arc) in contour.as_array().unwrap().iter().enumerate() {
            if index == 0 {
                points.push(point(&arc["a"]));
            }
            points.push(point(&arc["b"]));
            points.push(point(&arc["c"]));
            points.push(point(&arc["d"]));
        }
    }
    points
}

fn assert_close(actual: &[Point], expected: &[Point], tolerance: f64, context: &str) {
    assert_eq!(actual.len(), expected.len(), "{context}: point count");
    for (index, (a, b)) in actual.iter().zip(expected).enumerate() {
        let deviation = (a.x - b.x).abs().max((a.y - b.y).abs());
        assert!(
            deviation <= tolerance,
            "{context} point {index}: got ({}, {}), expected ({}, {})",
            a.x,
            a.y,
            b.x,
            b.y
        );
    }
}

fn sample_path(path: &BezPath, points: &mut Vec<Point>) {
    for segment in path.segments() {
        for index in 0..=SAMPLES_PER_CURVE {
            let t = index as f64 / SAMPLES_PER_CURVE as f64;
            points.push(segment.eval(t));
        }
    }
}

fn directed(samples: &[Point], path: &BezPath) -> f64 {
    let segments: Vec<_> = path.segments().collect();
    let mut worst: f64 = 0.0;
    for p in samples {
        let mut nearest = f64::INFINITY;
        for segment in &segments {
            let distance = segment.nearest(*p, 1e-3).distance_sq.sqrt();
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

#[test]
fn stroke_expansion_matches_upstream() {
    let corpus: Value =
        serde_json::from_str(&fs::read_to_string(workspace_path("corpus/stroke.json")).unwrap())
            .unwrap();
    let golden: Value =
        serde_json::from_str(&fs::read_to_string(workspace_path("golden/stroke.json")).unwrap())
            .unwrap();

    let cases = corpus["cases"].as_array().unwrap();
    let expected_cases = golden["cases"].as_array().unwrap();
    assert_eq!(cases.len(), expected_cases.len());

    let mut worst_union = 0.0_f64;
    for (case, expected) in cases.iter().zip(expected_cases) {
        let name = case["name"].as_str().unwrap();
        let radius = number(&case["radius"]);
        let contrast = number(&case["contrast"]);
        let inside = case["inside"].as_bool().unwrap();
        let contours = contours_from(case);
        let contour = &contours[0];

        let (left, right) = stroke_offsets(contour, radius, contrast);
        assert_close(
            &path_points(&left[0]),
            &golden_points(&expected["left"]),
            OFFSET_TOLERANCE,
            &format!("{name} left"),
        );
        assert_close(
            &path_points(&right[0]),
            &golden_points(&expected["right"]),
            OFFSET_TOLERANCE,
            &format!("{name} right"),
        );

        let mut ours_samples = Vec::new();
        let mut ours_path = BezPath::new();
        for path in stroke_arcs(&contours, radius, contrast, inside) {
            sample_path(&path, &mut ours_samples);
            ours_path.extend(path.iter());
        }
        assert!(!ours_samples.is_empty(), "{name}: empty stroke");

        let mut theirs_path = BezPath::new();
        for contour in expected["output"].as_array().unwrap() {
            for (index, arc) in contour.as_array().unwrap().iter().enumerate() {
                if index == 0 {
                    theirs_path.move_to(point(&arc["a"]));
                }
                theirs_path.curve_to(point(&arc["b"]), point(&arc["c"]), point(&arc["d"]));
            }
            theirs_path.close_path();
        }
        let mut theirs_samples = Vec::new();
        sample_path(&theirs_path, &mut theirs_samples);

        let deviation =
            directed(&ours_samples, &theirs_path).max(directed(&theirs_samples, &ours_path));
        worst_union = worst_union.max(deviation);
        assert!(
            deviation <= UNION_TOLERANCE,
            "{name}: union deviation {deviation} over budget"
        );
    }
    eprintln!("stroke expansion: worst union deviation {worst_union} font units");
}
