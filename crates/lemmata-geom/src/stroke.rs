//! Stroke expansion: offset a filled shape's boundary and union the sides.
//!
//! Ported from `stroke.mjs` and the arc helpers it uses from `curve-util.mjs`.
//! Non-cubic arcs (the offsets and the round caps) are fitted to cubics with the
//! same subdivision scheme typo-geom uses, so the fitted contours match.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::similar_names,
    clippy::suboptimal_flops,
    clippy::too_many_arguments,
    clippy::while_float
)]

use kurbo::{BezPath, Point, Vec2};
use linesweeper::BinaryOp;

use crate::boolean::boolean;

/// Fit tolerance for the cubic conversion, from `curve-util.mjs`.
const GEOMETRY_PRECISION: f64 = 0.25;
/// Gap size that earns a round cap between two offset arcs.
const OCCURRENT_PRECISION: f64 = 1.0 / 16.0;
/// Finite-difference step for offset curve derivatives.
const FINITE_DIFFERENCE_DELTA: f64 = 1.0 / 65536.0;
/// Probes per fit attempt.
const PROBE_STOPS: usize = 4;
/// Recursion cap in the fitter.
const MAX_FIT_DEPTH: usize = 8;

/// A cubic Bezier, in the shape order the upstream tools use.
#[derive(Clone, Copy, Debug)]
pub struct Bez3 {
    pub a: Point,
    pub b: Point,
    pub c: Point,
    pub d: Point,
}

impl Bez3 {
    #[must_use]
    pub const fn new(a: Point, b: Point, c: Point, d: Point) -> Self {
        Self { a, b, c, d }
    }

    #[must_use]
    pub fn eval(&self, t: f64) -> Point {
        let u = 1.0 - t;
        let (w0, w1, w2, w3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        Point::new(
            w0 * self.a.x + w1 * self.b.x + w2 * self.c.x + w3 * self.d.x,
            w0 * self.a.y + w1 * self.b.y + w2 * self.c.y + w3 * self.d.y,
        )
    }

    #[must_use]
    pub fn derivative(&self, t: f64) -> Vec2 {
        let u = 1.0 - t;
        let (w0, w1, w2) = (3.0 * u * u, 6.0 * u * t, 3.0 * t * t);
        Vec2::new(
            w0 * (self.b.x - self.a.x) + w1 * (self.c.x - self.b.x) + w2 * (self.d.x - self.c.x),
            w0 * (self.b.y - self.a.y) + w1 * (self.c.y - self.b.y) + w2 * (self.d.y - self.c.y),
        )
    }
}

/// An offset of a cubic along its normal.
#[derive(Clone, Copy, Debug)]
struct OffsetCurve {
    bone: Bez3,
    offset: f64,
    contrast: f64,
}

impl OffsetCurve {
    fn eval(&self, t: f64) -> Point {
        let c = self.bone.eval(t);
        let d = self.bone.derivative(t);
        let abs_d = d.hypot();
        Point::new(
            c.x - (d.y / abs_d) * self.offset * self.contrast,
            c.y + (d.x / abs_d) * self.offset,
        )
    }

    fn derivative(&self, t: f64) -> Vec2 {
        derivative_from_finite_difference(self, t)
    }
}

/// A round cap between two offset curves.
#[derive(Clone, Copy, Debug)]
struct RoundCapCurve {
    contrast: f64,
    center0: Point,
    center1: Point,
    theta0: f64,
    theta1: f64,
    r0: f64,
    r1: f64,
}

impl RoundCapCurve {
    fn new(
        side: bool,
        contrast: f64,
        center0: Point,
        point0: Point,
        center1: Point,
        point1: Point,
    ) -> Self {
        let theta0 = (point0.y - center0.y).atan2((point0.x - center0.x) / contrast);
        let mut theta1 = (point1.y - center1.y).atan2((point1.x - center1.x) / contrast);
        if side {
            while theta1 < theta0 {
                theta1 += 2.0 * std::f64::consts::PI;
            }
        } else {
            while theta1 > theta0 {
                theta1 -= 2.0 * std::f64::consts::PI;
            }
        }
        let r0 = (center0.y - point0.y).hypot((center0.x - point0.x) / contrast);
        let r1 = (center1.y - point1.y).hypot((center1.x - point1.x) / contrast);
        Self {
            contrast,
            center0,
            center1,
            theta0,
            theta1,
            r0,
            r1,
        }
    }

    fn eval(&self, t: f64) -> Point {
        let center_x = mix(self.center0.x, self.center1.x, t);
        let center_y = mix(self.center0.y, self.center1.y, t);
        let r = mix(self.r0, self.r1, t);
        let theta = mix(self.theta0, self.theta1, t);
        Point::new(
            center_x + r * theta.cos() * self.contrast,
            center_y + r * theta.sin(),
        )
    }

    fn derivative(&self, t: f64) -> Vec2 {
        let theta = mix(self.theta0, self.theta1, t);
        let r = mix(self.r0, self.r1, t);
        let dx = self.center1.x - self.center0.x
            + self.contrast
                * ((self.r1 - self.r0) * theta.cos()
                    - (self.theta1 - self.theta0) * r * theta.sin());
        let dy = self.center1.y - self.center0.y
            + ((self.r1 - self.r0) * theta.sin() + (self.theta1 - self.theta0) * r * theta.cos());
        Vec2::new(dx, dy)
    }
}

/// Arc types the offset step produces.
#[derive(Clone, Debug)]
enum Curve {
    Offset(OffsetCurve),
    Cap(RoundCapCurve),
    Reverted(Box<Curve>),
}

impl Curve {
    fn eval(&self, t: f64) -> Point {
        match self {
            Self::Offset(offset) => offset.eval(t),
            Self::Cap(cap) => cap.eval(t),
            Self::Reverted(inner) => inner.eval(1.0 - t),
        }
    }

    fn derivative(&self, t: f64) -> Vec2 {
        match self {
            Self::Offset(offset) => offset.derivative(t),
            Self::Cap(cap) => cap.derivative(t),
            Self::Reverted(inner) => -inner.derivative(1.0 - t),
        }
    }
}

/// Offsets one contour both ways and fits the result to cubics.
///
/// Returns the left and right fitted contours, matching the two shapes
/// `strokeArcs` converts before it unions anything.
#[must_use]
pub fn stroke_offsets(
    contour: &[Bez3],
    radius: f64,
    contrast: f64,
) -> (Vec<BezPath>, Vec<BezPath>) {
    let left = offset_contour(contour, -radius, contrast);
    let right = offset_contour(contour, radius, contrast);
    (
        vec![contour_to_path(&fit_contour(&left, GEOMETRY_PRECISION))],
        vec![contour_to_path(&fit_contour(&right, GEOMETRY_PRECISION))],
    )
}

/// The `strokeArcs` entry point: offset every contour, union the sides, and
/// optionally clip to the original shape.
#[must_use]
pub fn stroke_arcs(
    contours: &[Vec<Bez3>],
    radius: f64,
    contrast: f64,
    inside: bool,
) -> Vec<BezPath> {
    let mut current: Option<Vec<BezPath>> = None;
    for contour in contours {
        let left = offset_contour(contour, -radius, contrast);
        let right = offset_contour(contour, radius, contrast);

        let mut bezs = vec![contour_to_path(&fit_contour(&left, GEOMETRY_PRECISION))];
        bezs.push(contour_to_path(&fit_contour(&right, GEOMETRY_PRECISION)));

        current = Some(match current {
            None => bezs,
            Some(previous) => boolean_shapes(&previous, &bezs, BinaryOp::Union),
        });
    }

    match current {
        None => Vec::new(),
        Some(current) if inside => {
            let original: Vec<BezPath> = contours.iter().map(|c| contour_to_path(c)).collect();
            boolean_shapes(&original, &current, BinaryOp::Intersection)
        }
        Some(current) => current,
    }
}

fn offset_contour(contour: &[Bez3], distance: f64, contrast: f64) -> Vec<Curve> {
    let reverse = distance < 0.0;
    let mut offset_arcs = Vec::new();

    // Start from the last arc so the first iteration has a predecessor.
    let mut previous = Curve::Offset(OffsetCurve {
        bone: contour[contour.len() - 1],
        offset: distance,
        contrast,
    });

    for arc in contour {
        let current = Curve::Offset(OffsetCurve {
            bone: *arc,
            offset: distance,
            contrast,
        });

        let previous_end = previous.eval(1.0);
        let current_start = current.eval(0.0);
        if (previous_end.x - current_start.x).abs() > OCCURRENT_PRECISION
            || (previous_end.y - current_start.y).abs() > OCCURRENT_PRECISION
        {
            let (bone0, bone1) = match (&previous, &current) {
                (Curve::Offset(p), Curve::Offset(c)) => (p.bone, c.bone),
                _ => unreachable!("offset contour holds only offsets"),
            };
            offset_arcs.push(Curve::Cap(RoundCapCurve::new(
                distance < 0.0,
                contrast,
                bone0.eval(1.0),
                previous_end,
                bone1.eval(0.0),
                current_start,
            )));
        }

        offset_arcs.push(current);
        previous = offset_arcs[offset_arcs.len() - 1].clone();
    }

    if reverse {
        offset_arcs.reverse();
        for arc in &mut offset_arcs {
            *arc = Curve::Reverted(Box::new(arc.clone()));
        }
    }
    offset_arcs
}

fn fit_contour(arcs: &[Curve], err: f64) -> Vec<Bez3> {
    let mut segments: Vec<Bez3> = Vec::new();
    for arc in arcs {
        fit_arc(arc, err, &mut segments, 0.0, 1.0, 0);
    }
    if let (Some(first), Some(last)) = (segments.first().copied(), segments.last().copied())
        && !points_close(last.d, first.a, 1e-6)
    {
        segments.push(straight_bez3(last.d, first.a));
    }
    segments
}

fn fit_arc(arc: &Curve, err: f64, sink: &mut Vec<Bez3>, t0: f64, t1: f64, depth: usize) {
    let test = slice_from_arc(arc, t0, t1);
    if depth >= MAX_FIT_DEPTH {
        sink_add(sink, test);
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
        fit_arc(arc, err, sink, t0, mid, depth + 1);
        fit_arc(arc, err, sink, mid, t1, depth + 1);
    } else {
        sink_add(sink, test);
    }
}

fn slice_from_arc(arc: &Curve, t0: f64, t1: f64) -> Bez3 {
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

fn sink_add(segments: &mut Vec<Bez3>, segment: Bez3) {
    if let Some(previous) = segments.last()
        && !points_close(previous.d, segment.a, 1e-6)
    {
        segments.push(straight_bez3(previous.d, segment.a));
    }
    segments.push(segment);
}

fn straight_bez3(a: Point, b: Point) -> Bez3 {
    Bez3::new(a, mix_point(a, b, 1.0 / 3.0), mix_point(a, b, 2.0 / 3.0), b)
}

fn contour_to_path(contour: &[Bez3]) -> BezPath {
    let mut path = BezPath::new();
    if let Some(first) = contour.first() {
        path.move_to(first.a);
        for segment in contour {
            path.curve_to(segment.b, segment.c, segment.d);
        }
        path.close_path();
    }
    path
}

fn boolean_shapes(a: &[BezPath], b: &[BezPath], op: BinaryOp) -> Vec<BezPath> {
    let mut a_path = BezPath::new();
    for path in a {
        a_path.extend(path.iter());
    }
    let mut b_path = BezPath::new();
    for path in b {
        b_path.extend(path.iter());
    }
    if let Ok(contours) = boolean(&a_path, &b_path, op) {
        contours
    } else {
        let mut fallback = a.to_vec();
        fallback.extend_from_slice(b);
        fallback
    }
}

fn derivative_from_finite_difference(curve: &impl CurveEval, t: f64) -> Vec2 {
    let forward2 = curve.eval(t + 2.0 * FINITE_DIFFERENCE_DELTA);
    let forward1 = curve.eval(t + FINITE_DIFFERENCE_DELTA);
    let backward1 = curve.eval(t - FINITE_DIFFERENCE_DELTA);
    let backward2 = curve.eval(t - 2.0 * FINITE_DIFFERENCE_DELTA);
    Vec2::new(
        (backward2.x - 8.0 * backward1.x + 8.0 * forward1.x - forward2.x)
            / (12.0 * FINITE_DIFFERENCE_DELTA),
        (backward2.y - 8.0 * backward1.y + 8.0 * forward1.y - forward2.y)
            / (12.0 * FINITE_DIFFERENCE_DELTA),
    )
}

trait CurveEval {
    fn eval(&self, t: f64) -> Point;
}

impl CurveEval for OffsetCurve {
    fn eval(&self, t: f64) -> Point {
        OffsetCurve::eval(self, t)
    }
}

fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn mix_point(a: Point, b: Point, t: f64) -> Point {
    Point::new(mix(a.x, b.x, t), mix(a.y, b.y, t))
}

fn points_close(a: Point, b: Point, tolerance: f64) -> bool {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy < tolerance * tolerance
}
