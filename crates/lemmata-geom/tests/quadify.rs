//! Compares the Rust quadratic conversion against Iosevka's `QuadifySink`.

use std::fs;
use std::path::{Path, PathBuf};

use lemmata_geom::{Bez3, QuadifyKind, quadify};
use serde_json::Value;

/// Coordinate tolerance for the quadratified contours.
const TOLERANCE: f64 = 1e-9;

fn workspace_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn number(value: &Value) -> f64 {
    value.as_f64().expect("number")
}

fn point(value: &Value) -> kurbo::Point {
    kurbo::Point::new(number(&value["x"]), number(&value["y"]))
}

fn bez3(arc: &Value) -> Bez3 {
    Bez3::new(
        point(&arc["a"]),
        point(&arc["b"]),
        point(&arc["c"]),
        point(&arc["d"]),
    )
}

#[test]
fn quadify_matches_upstream() {
    let corpus: Value =
        serde_json::from_str(&fs::read_to_string(workspace_path("corpus/quadify.json")).unwrap())
            .unwrap();
    let golden: Value =
        serde_json::from_str(&fs::read_to_string(workspace_path("golden/quadify.json")).unwrap())
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

        let ours = quadify(&shape);
        let theirs = expected["contours"].as_array().unwrap();
        assert_eq!(ours.len(), theirs.len(), "{name}: contour count");

        for (index, (contour, expected_contour)) in ours.iter().zip(theirs).enumerate() {
            let expected_points = expected_contour.as_array().unwrap();
            assert_eq!(
                contour.len(),
                expected_points.len(),
                "{name} contour {index}: point count"
            );
            for (point_index, (a, b)) in contour.iter().zip(expected_points).enumerate() {
                let expected_quadratic = b["quadratic"].as_bool().unwrap();
                let is_quadratic = a.kind == QuadifyKind::Quadratic;
                assert_eq!(
                    is_quadratic, expected_quadratic,
                    "{name} contour {index} point {point_index}: kind"
                );
                let (ex, ey) = (number(&b["x"]), number(&b["y"]));
                assert!(
                    (a.x - ex).abs() <= TOLERANCE && (a.y - ey).abs() <= TOLERANCE,
                    "{name} contour {index} point {point_index}: got ({}, {}), expected ({ex}, {ey})",
                    a.x,
                    a.y
                );
            }
        }
    }
}
