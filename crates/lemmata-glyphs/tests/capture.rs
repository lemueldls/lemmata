//! Replays captured glyph geometry and compares the outlines it produces with
//! the contours Iosevka's own conversion produced.
//!
//! Skips when no capture is present, like the reference-font tests.
//!
//! The comparison is coverage-aware. A boundary point of one outline that lies
//! outside the other is drift of at least that depth; a point that lies inside
//! the other is a seam or a coincident boundary, which different boolean engines
//! split differently while describing the same region.

#![allow(clippy::cast_precision_loss, clippy::suboptimal_flops)]

use std::fs;
use std::path::{Path, PathBuf};

use kurbo::{BezPath, ParamCurve, ParamCurveNearest, Point, Shape};
use lemmata_geom::{QuadifyKind, QuadifyPoint};
use serde_json::Value;

/// The drift budget, in font units, from PLAN.md's `r[drift.outline]`. It is
/// calibrated rather than chosen: the shapes are exact and the residual is the
/// fairization artifact described in docs/glyphs.md, worth 0.075 px at 16 ppem.
const TOLERANCE: f64 = 5.0;
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
        if deviation > TOLERANCE {
            // Point counts separate a quadify decision from a placement
            // difference, which is worth knowing when this fails.
            over_budget.push(format!(
                "{name} {deviation:.2} (points {} vs {})",
                ours.iter().map(Vec::len).sum::<usize>(),
                theirs
                    .iter()
                    .map(|contour| contour.as_array().unwrap().len())
                    .sum::<usize>()
            ));
        }
    }

    eprintln!(
        "captured glyphs: {} glyphs, {quadratic_points} quadratic points, worst {worst:.3} on {worst_name}",
        glyphs.len()
    );
    eprintln!(
        "over the {TOLERANCE} unit drift budget: {} of {}",
        over_budget.len(),
        glyphs.len()
    );
    assert!(
        over_budget.is_empty(),
        "over the {TOLERANCE} unit drift budget: {}",
        over_budget.join(", ")
    );
}

/// Fraction of a grid over the outline where the two shapes disagree.
fn coverage_mismatch(ours: &BezPath, theirs: &BezPath, bounds: kurbo::Rect) -> f64 {
    const GRID: usize = 128;
    let mut mismatches = 0usize;
    let mut total = 0usize;
    for row in 0..GRID {
        let y = bounds.y1 - (row as f64 + 0.5) / GRID as f64 * bounds.height();
        for col in 0..GRID {
            let x = bounds.x0 + (col as f64 + 0.5) / GRID as f64 * bounds.width();
            let point = Point::new(x, y);
            total += 1;
            if ours.contains(point) != theirs.contains(point) {
                mismatches += 1;
            }
        }
    }
    mismatches as f64 / total as f64
}

/// The outlines should cover the same region, not merely have nearby
/// boundaries. A missing wedge at a stroke joint shows up here and nowhere
/// else.
#[test]
fn captured_glyphs_cover_the_same_region() {
    /// Allowed disagreement, as a fraction of the outline's area.
    const BUDGET: f64 = 0.05;
    let path = dump_path();
    let Ok(data) = fs::read(&path) else {
        eprintln!("skipping: no capture at {}", path.display());
        return;
    };
    let glyphs = lemmata_glyphs::load(&data).expect("decode capture");
    let reference: Value = serde_json::from_str(
        &fs::read_to_string(format!("{}.json", path.display())).expect("reference contours"),
    )
    .expect("parse reference contours");
    let reference = reference.as_array().expect("array");

    let mut worst = (0.0_f64, String::new());
    for (glyph, expected) in glyphs.iter().zip(reference) {
        let mut ours = BezPath::new();
        for contour in lemmata_glyphs::contours(glyph) {
            ours.extend(contour_path(&contour).iter());
        }
        let mut theirs = BezPath::new();
        for contour in expected["contours"].as_array().unwrap() {
            theirs.extend(reference_contour_path(contour).iter());
        }
        let fraction = coverage_mismatch(&ours, &theirs, ours.bounding_box());
        if fraction > worst.0 {
            worst = (fraction, glyph.name.clone());
        }
    }
    eprintln!(
        "coverage: worst mismatch {:.2}% on {}",
        100.0 * worst.0,
        worst.1
    );
    assert!(
        worst.0 <= BUDGET,
        "{} disagrees over {:.2}% of its area, over the {:.0}% budget",
        worst.1,
        100.0 * worst.0,
        100.0 * BUDGET
    );
}
