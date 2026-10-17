//! Fitting arbitrary arcs to cubics.
//!
//! This is typo-geom's `convertArcToBez3`: build a Hermite-style cubic over a
//! parameter range, probe it against the arc, and split until it fits. Stroke
//! expansion and the spiro outline both use it, on different arc types.
//!
//! Like typo-geom, a combined arc is not fitted as one curve. Callers expand it
//! into its segments and fit each one.

#![allow(clippy::cast_precision_loss, clippy::suboptimal_flops)]

use kurbo::{Point, Vec2};

use crate::stroke::Bez3;

/// Anything with a position and a tangent, so it can be fitted.
pub trait CurveArc: Clone {
    fn eval(&self, t: f64) -> Point;
    fn derivative(&self, t: f64) -> Vec2;
}

/// Probes per fit attempt.
const PROBE_STOPS: usize = 4;
/// Recursion cap.
const MAX_FIT_DEPTH: usize = 8;

/// Fits one contour of arcs to cubics, connecting gaps the way typo-geom does.
pub fn fit_contour<A: CurveArc>(arcs: &[A], err: f64) -> Vec<Bez3> {
    let mut segments: Vec<Bez3> = Vec::new();
    for arc in arcs {
        fit_arc_into(arc, err, &mut segments);
    }
    close_contour(&mut segments);

    segments
}

/// Fits one arc, appending its cubics and any connector they need.
pub fn fit_arc_into<A: CurveArc>(arc: &A, err: f64, sink: &mut Vec<Bez3>) {
    fit_recursive(arc, err, sink, 0.0, 1.0, 0);
}

/// Connects the last cubic back to the first if they do not meet.
pub fn close_contour(segments: &mut Vec<Bez3>) {
    if let (Some(first), Some(last)) = (segments.first().copied(), segments.last().copied())
        && !points_close(last.d, first.a, 1e-6)
    {
        push_segment(segments, straight_bez3(last.d, first.a));
    }
}

fn fit_recursive<A: CurveArc>(
    arc: &A,
    err: f64,
    sink: &mut Vec<Bez3>,
    t0: f64,
    t1: f64,
    depth: usize,
) {
    let test = slice_from_arc(arc, t0, t1);
    if depth >= MAX_FIT_DEPTH {
        push_segment(sink, test);
        return;
    }

    let mut needs_subdivide = false;
    for k in 1..PROBE_STOPS {
        let fraction = k as f64 / PROBE_STOPS as f64;
        let on_slice = test.eval(fraction);
        let on_arc = arc.eval(mix(t0, t1, fraction));

        if !points_close(on_slice, on_arc, err) {
            needs_subdivide = true;
            break;
        }
    }

    if needs_subdivide {
        let mid = mix(t0, t1, 0.5);
        fit_recursive(arc, err, sink, t0, mid, depth + 1);
        fit_recursive(arc, err, sink, mid, t1, depth + 1);
    } else {
        push_segment(sink, test);
    }
}

fn push_segment(segments: &mut Vec<Bez3>, segment: Bez3) {
    if let Some(previous) = segments.last()
        && !points_close(previous.d, segment.a, 1e-6)
    {
        segments.push(straight_bez3(previous.d, segment.a));
    }
    segments.push(segment);
}

/// The exact sub-curve of `arc` over `t0..=t1`.
///
/// A cubic is fixed by its ends and end derivatives, so the Hermite form
/// reproduces a slice of a cubic exactly. Used to rejoin pieces of one arc.
#[must_use]
pub fn slice_from_arc<A: CurveArc>(arc: &A, t0: f64, t1: f64) -> Bez3 {
    let scalar = t1 - t0;
    let z0 = arc.eval(t0);
    let z1 = arc.eval(t1);
    let d0 = arc.derivative(t0);
    let d1 = arc.derivative(t1);

    Bez3::new(
        z0,
        Point::new(z0.x + (scalar / 3.0) * d0.x, z0.y + (scalar / 3.0) * d0.y),
        Point::new(z1.x - (scalar / 3.0) * d1.x, z1.y - (scalar / 3.0) * d1.y),
        z1,
    )
}

pub(crate) fn straight_bez3(a: Point, b: Point) -> Bez3 {
    Bez3::new(a, mix_point(a, b, 1.0 / 3.0), mix_point(a, b, 2.0 / 3.0), b)
}

fn points_close(a: Point, b: Point, tolerance: f64) -> bool {
    let dx = a.x - b.x;
    let dy = a.y - b.y;

    dx * dx + dy * dy < tolerance * tolerance
}

fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn mix_point(a: Point, b: Point, t: f64) -> Point {
    Point::new(mix(a.x, b.x, t), mix(a.y, b.y, t))
}
