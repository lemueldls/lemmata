//! Compares the Rust geometry tree against Iosevka's classes.
//!
//! `corpus/tree.json` describes trees of the classes in `index.mjs`; the golden
//! file carries their `toBezArcs` or `toContours` output. Cases that pass
//! through boolean operations allow a small deviation, since the two engines
//! differ.

#![allow(clippy::cast_precision_loss)]

use std::fs;
use std::path::{Path, PathBuf};

use kurbo::{Affine, BezPath, ParamCurve, ParamCurveNearest, Point, Vec2};
use lemmata_geom::{Bez3, BiKnot, Geometry, KnotType, MonoKnot, PenKnot, shape_to_paths};
use linesweeper::BinaryOp;
use serde_json::Value;

/// Samples per segment when comparing shapes.
const SAMPLES_PER_SEGMENT: usize = 16;

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

fn profile(value: &Value) -> Vec<Vec2> {
    value
        .as_array()
        .expect("profile")
        .iter()
        .map(|p| Vec2::new(number(&p["x"]), number(&p["y"])))
        .collect()
}

fn build(value: &Value) -> Geometry {
    match value["kind"].as_str().expect("kind") {
        "spiro" => Geometry::Spiro {
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
        },
        "diSpiro" => Geometry::DiSpiro {
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
                            Some(Vec2::new(number(&normal["x"]), number(&normal["y"])));
                    }
                    knot
                })
                .collect(),
        },
        "spiroPen" => Geometry::SpiroPen {
            gizmo: affine(value.get("gizmo")),
            closed: value["closed"].as_bool().unwrap(),
            pen_profile: profile(&value["profile"]),
            knots: value["knots"]
                .as_array()
                .unwrap()
                .iter()
                .map(|k| {
                    let knot_profile = k.get("profile").filter(|p| !p.is_null()).map(profile);
                    PenKnot::new(
                        knot_type(k["type"].as_str().unwrap()),
                        number(&k["x"]),
                        number(&k["y"]),
                        knot_profile,
                    )
                })
                .collect(),
        },
        "stroke" => Geometry::Stroke {
            gizmo: affine(value.get("gizmo")),
            radius: number(&value["radius"]),
            contrast: number(&value["contrast"]),
            inside: value["inside"].as_bool().unwrap(),
            geom: Box::new(build(&value["geom"])),
        },
        "removeHoles" => Geometry::RemoveHoles {
            gizmo: affine(value.get("gizmo")),
            geom: Box::new(build(&value["geom"])),
        },
        "simplify" => Geometry::Simplify {
            gizmo: affine(value.get("gizmo")),
            geom: Box::new(build(&value["geom"])),
        },
        "combine" => Geometry::Combine(
            value["parts"]
                .as_array()
                .unwrap()
                .iter()
                .map(build)
                .collect(),
        ),
        "boolean" => Geometry::Boolean {
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
                .map(build)
                .collect(),
        },
        "transform" => Geometry::Transformed {
            tf: affine(value.get("tf")),
            geom: Box::new(build(&value["geom"])),
        },
        "radical" => Geometry::Radical(Box::new(build(&value["geom"]))),
        other => panic!("unknown kind {other}"),
    }
}

fn path_from_arcs(contours: &Value) -> BezPath {
    let mut path = BezPath::new();
    for contour in contours.as_array().unwrap() {
        for (index, arc) in contour.as_array().unwrap().iter().enumerate() {
            if index == 0 {
                path.move_to(point(&arc["a"]));
            }
            path.curve_to(point(&arc["b"]), point(&arc["c"]), point(&arc["d"]));
        }
        path.close_path();
    }
    path
}

fn sample_shape(shape: &[Vec<Bez3>]) -> (Vec<Point>, BezPath) {
    let mut samples = Vec::new();
    let mut combined = BezPath::new();
    for path in shape_to_paths(shape) {
        for segment in path.segments() {
            for index in 0..=SAMPLES_PER_SEGMENT {
                samples.push(segment.eval(index as f64 / SAMPLES_PER_SEGMENT as f64));
            }
        }
        combined.extend(path.iter());
    }
    (samples, combined)
}

/// Largest distance from a point of `a` to the nearest point of `b`, both ways.
fn point_hausdorff(a: &[Point], b: &[Point]) -> f64 {
    let directed = |from: &[Point], to: &[Point]| {
        let mut worst: f64 = 0.0;
        for p in from {
            let mut nearest = f64::INFINITY;
            for q in to {
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
    };
    directed(a, b).max(directed(b, a))
}

/// Depth by which points of one outline stick out of the other. A point that
/// lies inside the other is a seam: two boolean engines can split the same
/// region differently, and that is not drift.
fn directed(samples: &[Point], path: &BezPath) -> f64 {
    use kurbo::Shape;
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
        if nearest > worst && !path.contains(*p) {
            worst = nearest;
        }
    }
    worst
}

#[test]
fn geometry_tree_matches_upstream() {
    let corpus: Value =
        serde_json::from_str(&fs::read_to_string(workspace_path("corpus/tree.json")).unwrap())
            .unwrap();
    let golden: Value =
        serde_json::from_str(&fs::read_to_string(workspace_path("golden/tree.json")).unwrap())
            .unwrap();

    let cases = corpus["cases"].as_array().unwrap();
    let expected_cases = golden["cases"].as_array().unwrap();
    assert_eq!(cases.len(), expected_cases.len());

    let mut worst = 0.0_f64;
    let mut failures: Vec<String> = Vec::new();
    for (case, expected) in cases.iter().zip(expected_cases) {
        let name = case["name"].as_str().unwrap();
        let tolerance = number(&case["tolerance"]);
        let geometry = build(&case["geom"]);

        if case["output"].as_str().unwrap() == "contours" {
            let ours = geometry.to_contours();
            let theirs = expected["contours"].as_array().unwrap();
            assert_eq!(ours.len(), theirs.len(), "{name}: contour count");
            for (contour, expected_contour) in ours.iter().zip(theirs) {
                let expected_points = expected_contour.as_array().unwrap();
                assert_eq!(contour.len(), expected_points.len(), "{name}: point count");

                let ours_points: Vec<Point> =
                    contour.iter().map(|p| Point::new(p.x, p.y)).collect();
                let theirs_points: Vec<Point> = expected_points.iter().map(point).collect();
                let ours_quadratics = contour
                    .iter()
                    .filter(|p| p.kind == lemmata_geom::QuadifyKind::Quadratic)
                    .count();
                let theirs_quadratics = expected_points
                    .iter()
                    .filter(|b| b["quadratic"].as_bool().unwrap())
                    .count();
                assert_eq!(
                    ours_quadratics, theirs_quadratics,
                    "{name}: quadratic count"
                );

                let deviation = point_hausdorff(&ours_points, &theirs_points);
                worst = worst.max(deviation / tolerance.max(1e-9));
                if deviation > tolerance {
                    failures.push(format!(
                        "{name}: contour deviation {deviation} over budget {tolerance}"
                    ));
                }
            }
            continue;
        }

        let (ours_samples, ours_path) = sample_shape(&geometry.to_bez_arcs());
        let theirs_path = path_from_arcs(&expected["arcs"]);
        let mut theirs_samples = Vec::new();
        for segment in theirs_path.segments() {
            for index in 0..=SAMPLES_PER_SEGMENT {
                theirs_samples.push(segment.eval(index as f64 / SAMPLES_PER_SEGMENT as f64));
            }
        }
        let deviation =
            directed(&ours_samples, &theirs_path).max(directed(&theirs_samples, &ours_path));
        worst = worst.max(deviation / tolerance.max(1e-9));
        if deviation > tolerance {
            use kurbo::Shape;
            let ours_area: f64 = shape_to_paths(&geometry.to_bez_arcs())
                .iter()
                .map(BezPath::area)
                .sum();
            let theirs_area: f64 = path_from_arcs(&expected["arcs"]).area();
            let ours_contours = geometry.to_bez_arcs().len();
            let theirs_contours = expected["arcs"].as_array().unwrap().len();
            failures.push(format!(
                "{name}: deviation {deviation:.2} over budget {tolerance}; ours {ours_contours} contours area {ours_area:.0}, theirs {theirs_contours} area {theirs_area:.0}"
            ));
        }
    }
    eprintln!("geometry tree: worst deviation {worst:.3} of budget");
    assert!(
        failures.is_empty(),
        "{} of {} cases over budget:\n  {}",
        failures.len(),
        cases.len(),
        failures.join("\n  ")
    );
}
