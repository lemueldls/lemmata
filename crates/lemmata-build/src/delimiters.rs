//! Drawn hooks for the stretchy delimiters.
//!
//! A delimiter assembly needs a top and a bottom hook that end in a vertical
//! stem. The text glyph has no such stem: its whole side is one curve, so a
//! band cut from it cannot be tangent to a straight bar. These hooks are drawn
//! from the glyph's measured stem and tip instead, with a cubic that leaves the
//! tip horizontally and meets the stem vertically.

#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{QuadifyKind, QuadifyPoint};

use crate::font::GlyphData;

/// The x range of the ink near a height.
fn range_at(glyph: &GlyphData, y: f64, tolerance: f64) -> Option<(f64, f64)> {
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for point in glyph.contours.iter().flatten() {
        if (point.y - y).abs() <= tolerance {
            low = low.min(point.x);
            high = high.max(point.x);
        }
    }

    (low.is_finite() && high.is_finite()).then_some((low, high))
}

/// A point on a cubic.
fn cubic(p0: (f64, f64), c0: (f64, f64), c1: (f64, f64), p1: (f64, f64), t: f64) -> (f64, f64) {
    let u = 1.0 - t;
    (
        u * u * u * p0.0 + 3.0 * u * u * t * c0.0 + 3.0 * u * t * t * c1.0 + t * t * t * p1.0,
        u * u * u * p0.1 + 3.0 * u * u * t * c0.1 + 3.0 * u * t * t * c1.1 + t * t * t * p1.1,
    )
}

/// The derivative of a cubic.
fn tangent(p0: (f64, f64), c0: (f64, f64), c1: (f64, f64), p1: (f64, f64), t: f64) -> (f64, f64) {
    let u = 1.0 - t;
    (
        3.0 * u * u * (c0.0 - p0.0) + 6.0 * u * t * (c1.0 - c0.0) + 3.0 * t * t * (p1.0 - c1.0),
        3.0 * u * u * (c0.1 - p0.1) + 6.0 * u * t * (c1.1 - c0.1) + 3.0 * t * t * (p1.1 - c1.1),
    )
}

/// A stroked cubic that ends in a vertical bar of the same width, grounded at
/// the bottom. The cap at the tip is round.
fn hook(
    p0: (f64, f64),
    c0: (f64, f64),
    c1: (f64, f64),
    p1: (f64, f64),
    width: f64,
    stub: f64,
    down: bool,
) -> Vec<Vec<QuadifyPoint>> {
    const SAMPLES: usize = 24;

    let half = width / 2.0;
    let mut outer = Vec::new();
    let mut inner = Vec::new();
    for step in 0..=SAMPLES {
        let t = f64::from(u32::try_from(step).unwrap_or(0))
            / f64::from(u32::try_from(SAMPLES).unwrap_or(1));
        let p = cubic(p0, c0, c1, p1, t);
        let d = tangent(p0, c0, c1, p1, t);
        let length = f64::hypot(d.0, d.1);
        if length <= f64::EPSILON {
            continue;
        }
        let normal = (-d.1 / length, d.0 / length);
        outer.push((p.0 + normal.0 * half, p.1 + normal.1 * half));
        inner.push((p.0 - normal.0 * half, p.1 - normal.1 * half));
    }
    if outer.len() < 2 {
        return Vec::new();
    }

    let bottom = if down { p1.1 - stub } else { p1.1 + stub };
    let mut points: Vec<(f64, f64)> = Vec::new();
    points.extend(outer.iter().copied());
    points.push((outer[SAMPLES].0, bottom));
    points.push((inner[SAMPLES].0, bottom));
    points.extend(inner.iter().rev().copied());

    // The tip is a flat cap: the contour closes straight across the stroke,
    // matching the drawn ends of the rest of the font.
    let area = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(a, b)| a.0 * b.1 - b.0 * a.1)
        .sum::<f64>();
    if area < 0.0 {
        points.reverse();
    }

    // Round before grounding so the piece's bbox and the advance the table
    // writes are the same integer.
    let low = points
        .iter()
        .fold(f64::INFINITY, |low, point| low.min(point.1.round()));
    let body = points
        .into_iter()
        .map(|(x, y)| QuadifyPoint {
            kind: QuadifyKind::Corner,
            x: x.round(),
            y: y.round() - low,
        })
        .collect();

    vec![body]
}

/// The top or bottom hook of a delimiter, from the glyph's own stem and tip.
#[must_use]
pub fn drawn_hook(
    glyph: &GlyphData,
    top: bool,
    hook_fraction: f64,
    stub_fraction: f64,
    stem_fraction: f64,
    stem_override: Option<(f64, f64)>,
    stem_at_edge: bool,
) -> Option<Vec<Vec<QuadifyPoint>>> {
    let (x_min, y_min, x_max, y_max) = glyph.bounding_box()?;
    let h = f64::from(y_max - y_min);
    let (mut stem, width) = if let Some(stem) = stem_override {
        stem
    } else {
        let stem_y = if top {
            f64::from(y_max) - h * stem_fraction
        } else {
            f64::from(y_min) + h * stem_fraction
        };
        let (stem_low, stem_high) = range_at(glyph, stem_y, h * 0.03)?;
        (f64::midpoint(stem_low, stem_high), stem_high - stem_low)
    };
    if width <= 0.0 {
        return None;
    }
    let start = if top {
        f64::from(y_max)
    } else {
        f64::from(y_min)
    };
    let (tip_low, tip_high) = range_at(
        glyph,
        start + if top { -h * 0.02 } else { h * 0.02 },
        h * 0.04,
    )?;
    let measured = f64::midpoint(tip_low, tip_high);
    // A paren or bracket's stem is its outer edge, so a composed size keeps the
    // width the text glyph has. A brace's stem is its centre and stays measured.
    if stem_at_edge {
        stem = if measured > stem {
            f64::from(x_min) + width / 2.0
        } else {
            f64::from(x_max) - width / 2.0
        };
    }
    // The cap's outer edge is the glyph's own extreme.
    // A flat cap ends at the tip itself, so the tip is the glyph's own extreme.
    let tip = if measured > stem {
        f64::from(x_max)
    } else {
        f64::from(x_min)
    };
    let hook_height = h * hook_fraction;
    let stub = h * stub_fraction;
    let direction = if top { -1.0 } else { 1.0 };
    let p0 = (tip, start);
    let c0 = (tip + 0.55 * (stem - tip), start);
    let c1 = (stem, start + direction * 0.45 * hook_height);
    let p1 = (stem, start + direction * hook_height);

    let contours = hook(p0, c0, c1, p1, width, stub, top);
    (!contours.is_empty()).then_some(contours)
}

/// A drawn radical: the bottom hook, a vertical stem, and a flat top at the
/// stem's right edge where the renderer starts its vinculum.
///
/// `height` is the ink height and `advance` the cell the content starts at. The
/// stem is one rule thick and sits against the advance, so the content does not
/// touch it.
#[must_use]
pub fn radical(advance: f64, height: f64, rule: f64) -> Vec<Vec<QuadifyPoint>> {
    // The stem and hook carry the drawing's stroke, which is heavier than the
    // rule a renderer draws for the vinculum.
    let stroke = rule * 1.5;
    let gap = advance * 0.20;
    let stem_right = advance - gap;
    let stem_left = stem_right - stroke;
    let stem = stem_right - stroke / 2.0;
    let start = (stroke / 2.0, height * 0.45);
    let end = (stem, stroke / 2.0);
    let hook = hook(
        start,
        (
            start.0 + (end.0 - start.0) / 3.0,
            start.1 + (end.1 - start.1) / 3.0,
        ),
        (
            start.0 + 2.0 * (end.0 - start.0) / 3.0,
            start.1 + 2.0 * (end.1 - start.1) / 3.0,
        ),
        end,
        stroke,
        0.0,
        true,
    );

    let rect = |left: f64, right: f64, low: f64, high: f64| {
        vec![
            QuadifyPoint {
                kind: QuadifyKind::Corner,
                x: left,
                y: low,
            },
            QuadifyPoint {
                kind: QuadifyKind::Corner,
                x: right,
                y: low,
            },
            QuadifyPoint {
                kind: QuadifyKind::Corner,
                x: right,
                y: high,
            },
            QuadifyPoint {
                kind: QuadifyKind::Corner,
                x: left,
                y: high,
            },
        ]
    };

    let mut contours = hook;
    contours.push(rect(stem_left, stem_right, 0.0, height - rule));
    // The top bar is the renderer's rule thickness, not the stem's, so the
    // rule that starts at the advance continues it without a step. It bridges
    // the gap the stem leaves.
    contours.push(rect(stem_left, advance, height - rule, height));

    contours
}

/// The three pieces of the radical's assembly, bottom to top.
///
/// The bottom piece carries the hook, the extender is the stem, and the top
/// piece carries the bridge that meets the renderer's vinculum.
#[must_use]
pub fn radical_pieces(advance: f64, height: f64, rule: f64) -> Vec<Vec<Vec<QuadifyPoint>>> {
    let stroke = rule * 1.5;
    let gap = advance * 0.20;
    let stem_right = advance - gap;
    let stem_left = stem_right - stroke;
    let stem = stem_right - stroke / 2.0;
    let low_cut = (height * 0.55).round();
    let high_cut = (height * 0.75).round();

    let start = (stroke / 2.0, height * 0.45);
    let end = (stem, stroke / 2.0);
    let hook = hook(
        start,
        (
            start.0 + (end.0 - start.0) / 3.0,
            start.1 + (end.1 - start.1) / 3.0,
        ),
        (
            start.0 + 2.0 * (end.0 - start.0) / 3.0,
            start.1 + 2.0 * (end.1 - start.1) / 3.0,
        ),
        end,
        stroke,
        0.0,
        true,
    );
    let rect = |left: f64, right: f64, low: f64, high: f64| {
        vec![
            QuadifyPoint {
                kind: QuadifyKind::Corner,
                x: left,
                y: low,
            },
            QuadifyPoint {
                kind: QuadifyKind::Corner,
                x: right,
                y: low,
            },
            QuadifyPoint {
                kind: QuadifyKind::Corner,
                x: right,
                y: high,
            },
            QuadifyPoint {
                kind: QuadifyKind::Corner,
                x: left,
                y: high,
            },
        ]
    };

    let mut bottom = hook;
    bottom.push(rect(stem_left, stem_right, 0.0, low_cut));
    let extender = vec![rect(stem_left, stem_right, 0.0, high_cut - low_cut)];
    let top_height = height - high_cut;
    let top = vec![
        rect(stem_left, stem_right, 0.0, top_height - rule),
        rect(stem_left, advance, top_height - rule, top_height),
    ];

    vec![bottom, extender, top]
}
