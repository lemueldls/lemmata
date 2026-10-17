//! Compares the Rust spiro outline path against Iosevka's `SpiroGeometry`.

use std::fs;
use std::path::{Path, PathBuf};

use kurbo::{Affine, Point};
use lemmata_geom::{Bez3, KnotType, MonoKnot, outline::spiro_to_bez_arcs};
use serde_json::Value;

/// Coordinate tolerance for the fitted outline.
const TOLERANCE: f64 = 1e-6;

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

fn affine(value: Option<&Value>) -> Affine {
    match value {
        None => Affine::IDENTITY,
        Some(t) => Affine::new([
            number(&t["xx"]),
            number(&t["yx"]),
            number(&t["xy"]),
            number(&t["yy"]),
            number(&t["tx"]),
            number(&t["ty"]),
        ]),
    }
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
fn spiro_outline_matches_upstream() {
    let corpus: Value =
        serde_json::from_str(&fs::read_to_string(workspace_path("corpus/outline.json")).unwrap())
            .unwrap();
    let golden: Value =
        serde_json::from_str(&fs::read_to_string(workspace_path("golden/outline.json")).unwrap())
            .unwrap();

    let cases = corpus["cases"].as_array().unwrap();
    let expected_cases = golden["cases"].as_array().unwrap();
    assert_eq!(cases.len(), expected_cases.len());

    for (case, expected) in cases.iter().zip(expected_cases) {
        let name = case["name"].as_str().unwrap();
        let closed = case["closed"].as_bool().unwrap();
        let gizmo = affine(case.get("transform"));
        let knots: Vec<MonoKnot> = case["knots"]
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
            .collect();

        let ours = spiro_to_bez_arcs(&knots, closed, gizmo);
        if name == "closed-six-g4-unimportant" {
            for (index, contour) in ours.iter().enumerate() {
                for (arc_index, arc) in contour.iter().enumerate().skip(5).take(6) {
                    eprintln!(
                        "ours[{index}][{arc_index}] a=({:.3},{:.3}) b=({:.3},{:.3}) c=({:.3},{:.3}) d=({:.3},{:.3})",
                        arc.a.x, arc.a.y, arc.b.x, arc.b.y, arc.c.x, arc.c.y, arc.d.x, arc.d.y
                    );
                }
            }
            for (index, contour) in expected["contours"].as_array().unwrap().iter().enumerate() {
                for (arc_index, arc) in contour
                    .as_array()
                    .unwrap()
                    .iter()
                    .enumerate()
                    .skip(5)
                    .take(6)
                {
                    eprintln!(
                        "theirs[{index}][{arc_index}] a=({:.3},{:.3}) b=({:.3},{:.3}) c=({:.3},{:.3}) d=({:.3},{:.3})",
                        number(&arc["a"]["x"]),
                        number(&arc["a"]["y"]),
                        number(&arc["b"]["x"]),
                        number(&arc["b"]["y"]),
                        number(&arc["c"]["x"]),
                        number(&arc["c"]["y"]),
                        number(&arc["d"]["x"]),
                        number(&arc["d"]["y"])
                    );
                }
            }
        }
        let theirs = expected["contours"].as_array().unwrap();
        assert_eq!(ours.len(), theirs.len(), "{name}: contour count");

        for (index, (contour, expected_contour)) in ours.iter().zip(theirs).enumerate() {
            let expected_arcs = expected_contour.as_array().unwrap();
            assert_eq!(
                contour.len(),
                expected_arcs.len(),
                "{name} contour {index}: arc count"
            );
            for (arc_index, (arc, expected_arc)) in contour.iter().zip(expected_arcs).enumerate() {
                let context = format!("{name} contour {index} arc {arc_index}");
                let expected: Bez3 = Bez3::new(
                    point(&expected_arc["a"]),
                    point(&expected_arc["b"]),
                    point(&expected_arc["c"]),
                    point(&expected_arc["d"]),
                );
                assert_close(arc.a, expected.a, &context);
                assert_close(arc.b, expected.b, &context);
                assert_close(arc.c, expected.c, &context);
                assert_close(arc.d, expected.d, &context);
            }
        }
    }
}
