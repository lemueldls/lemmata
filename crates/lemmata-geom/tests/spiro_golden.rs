//! Compares the Rust spiro port against the JavaScript strands in `golden/spiro-cases.json`.

use std::{
    fs,
    path::{Path, PathBuf},
};

use kurbo::PathEl;
use lemmata_geom::{Knot, KnotType, spiro_path};
use serde_json::Value;

const TOLERANCE: f64 = 1e-9;

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

fn point(value: &Value) -> (f64, f64) {
    (number(&value["x"]), number(&value["y"]))
}

fn assert_close(actual: (f64, f64), expected: (f64, f64), context: &str) {
    let dx = (actual.0 - expected.0).abs();
    let dy = (actual.1 - expected.1).abs();
    assert!(
        dx <= TOLERANCE && dy <= TOLERANCE,
        "{context}: got {actual:?}, expected {expected:?}"
    );
}

#[test]
fn rust_port_matches_javascript_strands() {
    let corpus: Value = serde_json::from_str(
        &fs::read_to_string(workspace_path("corpus/spiro-knots.json")).unwrap(),
    )
    .unwrap();
    let golden: Value = serde_json::from_str(
        &fs::read_to_string(workspace_path("golden/spiro-cases.json")).unwrap(),
    )
    .unwrap();

    let cases = corpus["cases"].as_array().unwrap();
    let golden_cases = golden["cases"].as_array().unwrap();
    assert_eq!(cases.len(), golden_cases.len());

    for (case, expected) in cases.iter().zip(golden_cases) {
        let name = case["name"].as_str().unwrap();
        assert_eq!(name, expected["name"].as_str().unwrap());
        let closed = case["closed"].as_bool().unwrap();
        let delta = number(&case["delta"]);
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

        let path = spiro_path(&knots, closed, delta);
        let mut elements = path.elements().iter();
        match elements.next() {
            Some(PathEl::MoveTo(_)) => {}
            other => panic!("{name}: expected move, got {other:?}"),
        }

        for strand in expected["strands"].as_array().unwrap() {
            let order = strand["order"].as_u64().unwrap();
            match order {
                1 => {
                    match elements.next() {
                        Some(PathEl::LineTo(p)) => {
                            assert_close((p.x, p.y), point(&strand["end"]), name);
                        }
                        other => panic!("{name}: expected line, got {other:?}"),
                    }
                }
                3 => {
                    match elements.next() {
                        Some(PathEl::CurveTo(c1, c2, end)) => {
                            assert_close((c1.x, c1.y), point(&strand["c1"]), name);
                            assert_close((c2.x, c2.y), point(&strand["c2"]), name);
                            assert_close((end.x, end.y), point(&strand["end"]), name);
                        }
                        other => panic!("{name}: expected cubic, got {other:?}"),
                    }
                }
                other => panic!("{name}: unexpected strand order {other}"),
            }
        }

        if let Some(extra) = elements.next() {
            panic!("{name}: extra path element {extra:?}");
        }
    }
}
