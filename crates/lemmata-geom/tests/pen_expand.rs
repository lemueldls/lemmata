//! Compares the Rust pen expansion against Iosevka's.
//!
//! Two levels: the raw strips (exact) and the unioned shape (within the slack
//! between linesweeper and typo-geom's integer grid).

use std::fs;
use std::path::{Path, PathBuf};

use kurbo::{Affine, BezPath, PathEl, Point, Shape, Vec2};
use lemmata_geom::{KnotType, PenExpander, PenKnot, pen_spiro_paths};
use serde_json::Value;

/// Coordinate tolerance for the raw strips.
const STRIP_TOLERANCE: f64 = 1e-6;
/// Flattening tolerance for the union comparison.
const FLATTEN: f64 = 0.01;

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

fn vector(value: &Value) -> Vec2 {
    Vec2::new(number(&value["x"]), number(&value["y"]))
}

fn knots_from(case: &Value) -> Vec<PenKnot> {
    case["knots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| {
            let profile = k["profile"]
                .as_array()
                .map(|points| points.iter().map(vector).collect::<Vec<Vec2>>());
            PenKnot::new(
                knot_type(k["type"].as_str().unwrap()),
                number(&k["x"]),
                number(&k["y"]),
                profile,
            )
        })
        .collect()
}

/// Path points in the order the strips use: move, cubic controls, line, cubic.
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

fn flatten(path: &BezPath, points: &mut Vec<Point>) {
    kurbo::flatten(path.iter(), FLATTEN, |element| match element {
        PathEl::MoveTo(p) | PathEl::LineTo(p) => points.push(p),
        PathEl::QuadTo(..) | PathEl::CurveTo(..) | PathEl::ClosePath => {}
    });
}

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

#[test]
fn pen_expansion_matches_upstream() {
    let corpus: Value = serde_json::from_str(
        &fs::read_to_string(workspace_path("corpus/pen-expand.json")).unwrap(),
    )
    .unwrap();
    let golden: Value = serde_json::from_str(
        &fs::read_to_string(workspace_path("golden/pen-expand.json")).unwrap(),
    )
    .unwrap();

    let cases = corpus["cases"].as_array().unwrap();
    let expected_cases = golden["cases"].as_array().unwrap();
    assert_eq!(cases.len(), expected_cases.len());

    let mut worst_overreach = 0.0_f64;
    let mut worst_upstream = 0.0_f64;
    for (case, expected) in cases.iter().zip(expected_cases) {
        let name = case["name"].as_str().unwrap();
        let closed = case["closed"].as_bool().unwrap();
        let profile_edges = case["profile"].as_array().unwrap().len();
        let knots = knots_from(case);

        // Raw strips must match exactly.
        let mut expander = PenExpander::new(Affine::IDENTITY, profile_edges, closed, &knots);
        let strips = expander.geometry();
        let expected_strips = expected["raw"].as_array().unwrap();
        assert_eq!(strips.len(), expected_strips.len(), "{name}: strip count");
        for (strip, expected_strip) in strips.iter().zip(expected_strips) {
            let ours = path_points(strip);
            let theirs = expected_strip.as_array().unwrap();
            assert_eq!(ours.len(), theirs.len(), "{name}: strip point count");
            for (index, (a, b)) in ours.iter().zip(theirs).enumerate() {
                let expected = Point::new(number(&b["x"]), number(&b["y"]));
                let deviation = (a.x - expected.x).abs().max((a.y - expected.y).abs());
                assert!(
                    deviation <= STRIP_TOLERANCE,
                    "{name} strip point {index}: got ({}, {}), expected ({}, {})",
                    a.x,
                    a.y,
                    expected.x,
                    expected.y
                );
            }
        }

        // The union is ours to build (Iosevka unions with typo-geom on an
        // integer grid). Check that it covers nothing beyond the strips, and
        // record how far it lands from Iosevka's union as a measurement.
        let union_paths = pen_spiro_paths(Affine::IDENTITY, profile_edges, closed, &knots);
        let mut union_points = Vec::new();
        for path in &union_paths {
            flatten(path, &mut union_points);
        }
        let union_area: f64 = union_paths.iter().map(BezPath::area).sum();
        let strip_area: f64 = strips.iter().map(|strip| strip.area().abs()).sum();
        assert!(union_area > 0.0, "{name}: empty union");
        assert!(
            union_area <= strip_area.abs() + 1.0,
            "{name}: union area {union_area} exceeds its strips {strip_area}"
        );

        let mut strip_points = Vec::new();
        for strip in &strips {
            flatten(strip, &mut strip_points);
        }
        let overreach = directed(&union_points, &strip_points);
        assert!(
            overreach <= 2.5,
            "{name}: union boundary reaches {overreach} beyond the strips"
        );
        worst_overreach = worst_overreach.max(overreach);

        let mut theirs = Vec::new();
        for contour in expected["output"].as_array().unwrap() {
            let mut path = BezPath::new();
            for (index, arc) in contour.as_array().unwrap().iter().enumerate() {
                if index == 0 {
                    path.move_to(Point::new(number(&arc["a"]["x"]), number(&arc["a"]["y"])));
                }
                path.curve_to(
                    Point::new(number(&arc["b"]["x"]), number(&arc["b"]["y"])),
                    Point::new(number(&arc["c"]["x"]), number(&arc["c"]["y"])),
                    Point::new(number(&arc["d"]["x"]), number(&arc["d"]["y"])),
                );
            }
            path.close_path();
            flatten(&path, &mut theirs);
        }
        worst_upstream = worst_upstream.max(max_deviation(&union_points, &theirs));
    }
    eprintln!(
        "pen expansion: worst overreach {worst_overreach} font units, worst gap to Iosevka's union {worst_upstream}"
    );
}
