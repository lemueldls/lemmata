//! Compares the Rust contrast expander against Iosevka's.
//!
//! `corpus/spiro-expand.json` holds synthetic skeletons. The golden file carries
//! the expanded knots and the Bezier strands of the knot lists Iosevka draws.
//! The strands come from npm `spiro`, so this measures the port's wiring and
//! drawing against the same curve, not against Iosevka's Bezier fit of it.

use std::fs;
use std::path::{Path, PathBuf};

use kurbo::{Affine, BezPath, PathEl, Point, Vec2};
use lemmata_geom::{BiKnot, KnotType, MonoKnot, di_spiro_expand, di_spiro_paths};
use serde_json::Value;

/// Subdivision delta for our evaluation. Must match the oracle.
const DELTA: f64 = 0.01;
/// Coordinate tolerance for the expanded knots, which must match exactly.
const KNOT_TOLERANCE: f64 = 1e-9;
/// Coordinate tolerance for drawn strands. The interpolation case feeds an
/// interpolated knot through the solver, where a knot difference near 1e-9
/// amplifies to about 1e-5 in the output.
const STRAND_TOLERANCE: f64 = 1e-3;

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

fn deviation(actual: Point, expected: Point) -> f64 {
    (actual.x - expected.x)
        .abs()
        .max((actual.y - expected.y).abs())
}

fn assert_close(actual: Point, expected: Point, tolerance: f64, context: &str) {
    assert!(
        deviation(actual, expected) <= tolerance,
        "{context}: got ({}, {}), expected ({}, {})",
        actual.x,
        actual.y,
        expected.x,
        expected.y
    );
}

fn compare_knots(name: &str, side: &str, ours: &[MonoKnot], theirs: &[Value]) {
    assert_eq!(ours.len(), theirs.len(), "{name} {side}: length");
    for (index, (a, b)) in ours.iter().zip(theirs).enumerate() {
        let expected = Point::new(number(&b["x"]), number(&b["y"]));
        assert_close(
            Point::new(a.x, a.y),
            expected,
            KNOT_TOLERANCE,
            &format!("{name} {side}[{index}]"),
        );
    }
}

fn compare_side(name: &str, index: usize, path: &BezPath, strands: &[Value]) -> f64 {
    let mut worst = 0.0_f64;
    let mut elements = path.elements().iter().copied();
    match elements.next() {
        Some(PathEl::MoveTo(_)) => {}
        other => panic!("{name} side {index}: expected move, got {other:?}"),
    }
    for (strand_index, strand) in strands.iter().enumerate() {
        let order = strand["order"].as_u64().unwrap();
        let context = format!("{name} side {index} strand {strand_index}");

        match order {
            1 => match elements.next() {
                Some(PathEl::LineTo(p)) => {
                    let d = deviation(p, point(&strand["end"]));
                    if d > 1.0 {
                        eprintln!(
                            "{context}: line to ({}, {}), expected ({}, {}), d={d}",
                            p.x,
                            p.y,
                            number(&strand["end"]["x"]),
                            number(&strand["end"]["y"])
                        );
                    }
                    worst = worst.max(d);
                }
                other => panic!("{context}: expected line, got {other:?}"),
            },
            3 => match elements.next() {
                Some(PathEl::CurveTo(c1, c2, end)) => {
                    let d1 = deviation(c1, point(&strand["c1"]));
                    let d2 = deviation(c2, point(&strand["c2"]));
                    let d3 = deviation(end, point(&strand["end"]));
                    if d1.max(d2).max(d3) > 1.0 {
                        eprintln!(
                            "{context}: c1=({}, {}) exp ({}, {}) c2=({}, {}) exp ({}, {}) end=({}, {}) exp ({}, {})",
                            c1.x,
                            c1.y,
                            number(&strand["c1"]["x"]),
                            number(&strand["c1"]["y"]),
                            c2.x,
                            c2.y,
                            number(&strand["c2"]["x"]),
                            number(&strand["c2"]["y"]),
                            end.x,
                            end.y,
                            number(&strand["end"]["x"]),
                            number(&strand["end"]["y"])
                        );
                    }
                    worst = worst.max(d1).max(d2).max(d3);
                }
                other => panic!("{context}: expected cubic, got {other:?}"),
            },
            other => panic!("{context}: unexpected strand order {other}"),
        }
    }
    if let Some(extra) = elements.next() {
        panic!("{name} side {index}: extra path element {extra:?}");
    }
    assert!(
        worst <= STRAND_TOLERANCE,
        "{name} side {index}: strand deviation {worst} over budget"
    );

    worst
}

#[test]
fn contrast_expansion_matches_upstream() {
    let corpus: Value = serde_json::from_str(
        &fs::read_to_string(workspace_path("corpus/spiro-expand.json")).unwrap(),
    )
    .unwrap();
    let golden: Value = serde_json::from_str(
        &fs::read_to_string(workspace_path("golden/spiro-expand.json")).unwrap(),
    )
    .unwrap();

    let cases = corpus["cases"].as_array().unwrap();
    let expected_cases = golden["cases"].as_array().unwrap();
    assert_eq!(cases.len(), expected_cases.len());

    let mut worst = 0.0_f64;
    for (case, expected) in cases.iter().zip(expected_cases) {
        let name = case["name"].as_str().unwrap();
        let closed = case["closed"].as_bool().unwrap();
        let contrast = number(&case["contrast"]);
        let gizmo = affine(case.get("transform"));

        let knots: Vec<BiKnot> = case["knots"]
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
                        Some(Vec2::new(number(&normal["x"]), number(&normal["y"])));
                }

                knot
            })
            .collect();

        let expanded = di_spiro_expand(gizmo, contrast, closed, &knots);
        compare_knots(
            name,
            "lhs",
            &expanded.lhs_untransformed,
            expected["expanded"]["lhs"].as_array().unwrap(),
        );
        compare_knots(
            name,
            "rhs",
            &expanded.rhs_untransformed,
            expected["expanded"]["rhs"].as_array().unwrap(),
        );

        let paths = di_spiro_paths(gizmo, contrast, closed, &knots, DELTA);
        let sides = expected["sides"].as_array().unwrap();
        assert_eq!(paths.len(), sides.len(), "{name}: side count");

        for (index, (path, strands)) in paths.iter().zip(sides).enumerate() {
            worst = worst.max(compare_side(name, index, path, strands.as_array().unwrap()));
        }
    }
    eprintln!("contrast expansion: worst strand deviation {worst} font units");
}
