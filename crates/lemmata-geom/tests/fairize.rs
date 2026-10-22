//! Compares the Rust fairization against typo-geom's `fairizeBezierShape`.

use std::{
    fs,
    path::{Path, PathBuf},
};

use kurbo::Point;
use lemmata_geom::{Bez3, FairArc, fairize};
use serde_json::Value;

/// Coordinate tolerance for the fairized arcs.
const TOLERANCE: f64 = 1e-9;

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

fn bez3(arc: &Value) -> Bez3 {
    Bez3::new(
        point(&arc["a"]),
        point(&arc["b"]),
        point(&arc["c"]),
        point(&arc["d"]),
    )
}

fn assert_close(actual: Point, expected: Point, context: &str) {
    assert!(
        (actual.x - expected.x).abs() <= TOLERANCE && (actual.y - expected.y).abs() <= TOLERANCE,
        "{context}: got ({}, {}), expected ({}, {})",
        actual.x,
        actual.y,
        expected.x,
        expected.y
    );
}

#[test]
fn fairize_matches_upstream() {
    let corpus: Value =
        serde_json::from_str(&fs::read_to_string(workspace_path("corpus/fairize.json")).unwrap())
            .unwrap();
    let golden: Value =
        serde_json::from_str(&fs::read_to_string(workspace_path("golden/fairize.json")).unwrap())
            .unwrap();

    let cases = corpus["cases"].as_array().unwrap();
    let expected_cases = golden["cases"].as_array().unwrap();
    assert_eq!(cases.len(), expected_cases.len());

    for (case, expected) in cases.iter().zip(expected_cases) {
        let name = case["name"].as_str().unwrap();
        let shape: Vec<Vec<Bez3>> = case["contours"]
            .as_array()
            .unwrap()
            .iter()
            .map(|contour| contour.as_array().unwrap().iter().map(bez3).collect())
            .collect();

        let ours = fairize(&shape);
        let theirs = expected["contours"].as_array().unwrap();
        assert_eq!(ours.len(), theirs.len(), "{name}: contour count");

        for (index, (arcs, expected_arcs)) in ours.iter().zip(theirs).enumerate() {
            let expected_arcs = expected_arcs.as_array().unwrap();
            assert_eq!(
                arcs.len(),
                expected_arcs.len(),
                "{name} contour {index}: arc count"
            );

            for (arc_index, (arc, expected_arc)) in arcs.iter().zip(expected_arcs).enumerate() {
                let context = format!("{name} contour {index} arc {arc_index}");
                match arc {
                    FairArc::Straight { a, b } => {
                        assert_eq!(
                            expected_arc["kind"].as_str().unwrap(),
                            "straight",
                            "{context}: kind"
                        );
                        assert_close(*a, point(&expected_arc["a"]), &format!("{context} a"));
                        assert_close(*b, point(&expected_arc["b"]), &format!("{context} b"));
                    }
                    FairArc::Combined(combined) => {
                        assert_eq!(
                            expected_arc["kind"].as_str().unwrap(),
                            "combined",
                            "{context}: kind"
                        );
                        let expected_segments = expected_arc["segments"].as_array().unwrap();
                        assert_eq!(
                            combined.segments().len(),
                            expected_segments.len(),
                            "{context}: segment count"
                        );

                        for (segment, expected_segment) in
                            combined.segments().iter().zip(expected_segments)
                        {
                            assert_close(segment.a, point(&expected_segment["a"]), &context);
                            assert_close(segment.b, point(&expected_segment["b"]), &context);
                            assert_close(segment.c, point(&expected_segment["c"]), &context);
                            assert_close(segment.d, point(&expected_segment["d"]), &context);
                        }
                    }
                }
            }
        }
    }
}
