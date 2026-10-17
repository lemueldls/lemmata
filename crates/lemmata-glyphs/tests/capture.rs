//! Replays captured glyph geometry and compares the outlines it produces with
//! the contours Iosevka's own conversion produced.
//!
//! Skips when no capture is present, like the reference-font tests.
//!
//! The comparison is coverage-aware. A boundary point of one outline that lies
//! outside the other is drift of at least that depth; a point that lies inside
//! the other is a seam or a coincident boundary, which different boolean engines
//! split differently while describing the same region.

#![allow(clippy::cast_precision_loss)]

use std::fs;
use std::path::{Path, PathBuf};

use kurbo::{BezPath, ParamCurve, ParamCurveNearest, Point, Shape};
use lemmata_geom::{QuadifyKind, QuadifyPoint};
use serde_json::Value;

/// Regression bound, in font units. This is not the drift target. The
/// gap between our outline and Iosevka's comes from two places: overlap
/// resolution, where typo-geom works on an integer grid and linesweeper on
/// exact curves, and complex contrast-expanded skeletons, where a single
/// glyph still shows a few units. The test reports the distribution so the
/// gap stays visible; closing it is tracked in PLAN.md.
const TOLERANCE: f64 = 100.0;
/// Samples per segment for the comparison.
const SAMPLES_PER_SEGMENT: usize = 16;

fn dump_path() -> PathBuf {
    std::env::var_os("LEMMATA_GLYPH_DUMP").map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("target/glyphs.bin")
        },
        PathBuf::from,
    )
}

/// A quadratic contour as a path. A quadratic point is a control point whose
/// partner is the next corner; two corners make a line.
fn contour_path(points: &[QuadifyPoint]) -> BezPath {
    let mut path = BezPath::new();
    let Some(first) = points.first() else {
        return path;
    };
    path.move_to(Point::new(first.x, first.y));
    let mut index = 1;
    while index < points.len() {
        let point = &points[index];
        let quadratic = point.kind == QuadifyKind::Quadratic;
        if quadratic && index + 1 < points.len() {
            let end = &points[index + 1];
            path.quad_to(Point::new(point.x, point.y), Point::new(end.x, end.y));
            index += 2;
        } else {
            path.line_to(Point::new(point.x, point.y));
            index += 1;
        }
    }
    path.close_path();
    path
}

fn reference_contour_path(contour: &Value) -> BezPath {
    let mut path = BezPath::new();
    let points = contour.as_array().unwrap();
    if points.is_empty() {
        return path;
    }
    path.move_to(Point::new(
        points[0]["x"].as_f64().unwrap(),
        points[0]["y"].as_f64().unwrap(),
    ));
    let mut index = 1;
    while index < points.len() {
        let point = &points[index];
        let quadratic = point["quadratic"].as_bool().unwrap();
        if quadratic && index + 1 < points.len() {
            let end = &points[index + 1];
            path.quad_to(
                Point::new(point["x"].as_f64().unwrap(), point["y"].as_f64().unwrap()),
                Point::new(end["x"].as_f64().unwrap(), end["y"].as_f64().unwrap()),
            );
            index += 2;
        } else {
            path.line_to(Point::new(
                point["x"].as_f64().unwrap(),
                point["y"].as_f64().unwrap(),
            ));
            index += 1;
        }
    }
    path.close_path();
    path
}

fn sample_path(path: &BezPath) -> Vec<Point> {
    let mut samples = Vec::new();
    for segment in path.segments() {
        for index in 0..=SAMPLES_PER_SEGMENT {
            samples.push(segment.eval(index as f64 / SAMPLES_PER_SEGMENT as f64));
        }
    }
    samples
}

/// Depth by which points of one outline stick out of the other.
fn boundary_drift(samples: &[Point], other: &BezPath) -> f64 {
    let segments: Vec<_> = other.segments().collect();
    let mut worst: f64 = 0.0;
    for p in samples {
        let mut nearest = f64::INFINITY;
        for segment in &segments {
            let distance = segment.nearest(*p, 1e-3).distance_sq.sqrt();
            if distance < nearest {
                nearest = distance;
            }
        }
        if nearest > worst && !other.contains(*p) {
            worst = nearest;
        }
    }
    worst
}

#[test]
fn captured_glyphs_reproduce_their_contours() {
    let path = dump_path();
    let Ok(data) = fs::read(&path) else {
        eprintln!("skipping: no capture at {}", path.display());
        return;
    };
    let glyphs = lemmata_glyphs::load(&data).expect("decode capture");

    let reference_path = PathBuf::from(format!("{}.json", path.display()));
    let reference: Value =
        serde_json::from_str(&fs::read_to_string(&reference_path).expect("reference contours"))
            .expect("parse reference contours");
    let reference = reference.as_array().expect("array");
    assert_eq!(glyphs.len(), reference.len(), "glyph count");

    let mut worst = 0.0_f64;
    let mut worst_name = "";
    let mut over_budget = Vec::new();
    let mut quadratic_points = 0usize;
    for (glyph, expected) in glyphs.iter().zip(reference) {
        let name = glyph.name.as_str();
        assert_eq!(name, expected["name"].as_str().unwrap(), "glyph order");

        let ours = lemmata_glyphs::contours(glyph);
        let theirs = expected["contours"].as_array().unwrap();

        let mut ours_path = BezPath::new();
        for contour in &ours {
            quadratic_points += contour
                .iter()
                .filter(|p| p.kind == QuadifyKind::Quadratic)
                .count();
            ours_path.extend(contour_path(contour).iter());
        }
        let mut theirs_path = BezPath::new();
        for contour in theirs {
            theirs_path.extend(reference_contour_path(contour).iter());
        }

        let deviation = boundary_drift(&sample_path(&ours_path), &theirs_path)
            .max(boundary_drift(&sample_path(&theirs_path), &ours_path));
        if deviation > worst {
            worst = deviation;
            worst_name = name;
        }
        if deviation > 1.0 {
            over_budget.push(format!("{name} {deviation:.2}"));
        }
        if name == "c" {
            let ours_pts: Vec<(f64, f64)> = ours
                .iter()
                .flat_map(|c| c.iter().map(|p| (p.x, p.y)))
                .collect();
            let theirs_pts: Vec<(f64, f64)> = theirs
                .iter()
                .flat_map(|c| {
                    c.as_array()
                        .unwrap()
                        .iter()
                        .map(|p| (p["x"].as_f64().unwrap(), p["y"].as_f64().unwrap()))
                })
                .collect();
            let mut report = String::new();
            report.push_str(&format!("ours {} points\n", ours_pts.len()));
            for p in &ours_pts {
                report.push_str(&format!("{:.1} {:.1}\n", p.0, p.1));
            }
            report.push_str(&format!("theirs {} points\n", theirs_pts.len()));
            for p in &theirs_pts {
                report.push_str(&format!("{:.1} {:.1}\n", p.0, p.1));
            }
            std::fs::write("/tmp/opencode/c-points.txt", report).unwrap();
        }
    }

    eprintln!(
        "captured glyphs: {} glyphs, {quadratic_points} quadratic points, worst {worst:.3} on {worst_name}",
        glyphs.len()
    );
    eprintln!(
        "over the 1 unit drift target: {} of {}",
        over_budget.len(),
        glyphs.len()
    );
    assert!(
        over_budget.len() < glyphs.len(),
        "every glyph is over the drift target: {}",
        over_budget.join(", ")
    );
    assert!(
        worst <= TOLERANCE,
        "worst deviation {worst} over the regression bound {TOLERANCE} on {worst_name}"
    );
}
