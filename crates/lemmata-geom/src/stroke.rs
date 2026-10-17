//! Stroke expansion: offset a filled shape's boundary and union the sides.
//!
//! Ported from `stroke.mjs` and the arc helpers it uses from `curve-util.mjs`.
//! The arcs being offset can be any [`CurveArc`], matching upstream, where a
//! stroke's bones are whatever the geometry tree hands over. Non-cubic arcs (the
//! offsets and the round caps) are fitted to cubics with the same subdivision
//! scheme typo-geom uses, so the fitted contours match.

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

use kurbo::{BezPath, PathSeg, Point, Vec2};
use linesweeper::BinaryOp;

use crate::boolean::boolean;
use crate::fit::{CurveArc, fit_contour, straight_bez3};

/// Fit tolerance for the cubic conversion, from `curve-util.mjs`.
const GEOMETRY_PRECISION: f64 = 0.25;
/// Gap size that earns a round cap between two offset arcs.
const OCCURRENT_PRECISION: f64 = 1.0 / 16.0;
/// Finite-difference step for offset curve derivatives.
const FINITE_DIFFERENCE_DELTA: f64 = 1.0 / 65536.0;

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

impl CurveArc for Bez3 {
    fn eval(&self, t: f64) -> Point {
        Bez3::eval(self, t)
    }

    fn derivative(&self, t: f64) -> Vec2 {
        Bez3::derivative(self, t)
    }
}

/// An offset of a bone curve along its normal.
#[derive(Clone, Debug)]
struct OffsetCurve<A> {
    bone: A,
    offset: f64,
    contrast: f64,
}

impl<A: CurveArc> CurveArc for OffsetCurve<A> {
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
}

impl CurveArc for RoundCapCurve {
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
enum Curve<A> {
    Offset(OffsetCurve<A>),
    Cap(RoundCapCurve),
    Reverted(Box<Curve<A>>),
}

impl<A: CurveArc> CurveArc for Curve<A> {
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
pub fn stroke_offsets<A: CurveArc>(
    contour: &[A],
    radius: f64,
    contrast: f64,
) -> (Vec<Vec<Bez3>>, Vec<Vec<Bez3>>) {
    let left = offset_contour(contour, -radius, contrast);
    let right = offset_contour(contour, radius, contrast);

    (
        vec![fit_contour(&left, GEOMETRY_PRECISION)],
        vec![fit_contour(&right, GEOMETRY_PRECISION)],
    )
}

/// The `strokeArcs` entry point: offset every contour, union the sides, and
/// optionally clip to the original shape.
#[must_use]
pub fn stroke_arcs<A: CurveArc>(
    contours: &[Vec<A>],
    radius: f64,
    contrast: f64,
    inside: bool,
) -> Vec<Vec<Bez3>> {
    let mut current: Option<Vec<Vec<Bez3>>> = None;
    for contour in contours {
        let left = offset_contour(contour, -radius, contrast);
        let right = offset_contour(contour, radius, contrast);

        let mut bezs = vec![fit_contour(&left, GEOMETRY_PRECISION)];
        bezs.push(fit_contour(&right, GEOMETRY_PRECISION));

        current = Some(match current {
            None => bezs,
            Some(previous) => boolean_shapes(&previous, &bezs, BinaryOp::Union),
        });
    }

    match current {
        None => Vec::new(),
        Some(current) if inside => {
            let original: Vec<Vec<Bez3>> = contours
                .iter()
                .map(|contour| fit_contour(contour, GEOMETRY_PRECISION))
                .collect();
            boolean_shapes(&original, &current, BinaryOp::Intersection)
        }
        Some(current) => current,
    }
}

fn offset_contour<A: CurveArc>(contour: &[A], distance: f64, contrast: f64) -> Vec<Curve<A>> {
    let reverse = distance < 0.0;
    let mut offset_arcs = Vec::new();

    // Start from the last arc so the first iteration has a predecessor.
    let mut previous = Curve::Offset(OffsetCurve {
        bone: contour[contour.len() - 1].clone(),
        offset: distance,
        contrast,
    });

    for arc in contour {
        let current = Curve::Offset(OffsetCurve {
            bone: arc.clone(),
            offset: distance,
            contrast,
        });

        let previous_end = previous.eval(1.0);
        let current_start = current.eval(0.0);
        if (previous_end.x - current_start.x).abs() > OCCURRENT_PRECISION
            || (previous_end.y - current_start.y).abs() > OCCURRENT_PRECISION
        {
            let (bone0, bone1) = match (&previous, &current) {
                (Curve::Offset(p), Curve::Offset(c)) => (&p.bone, &c.bone),
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

/// A shape of cubics as kurbo paths, one per contour.
#[must_use]
pub fn shape_to_paths(shape: &[Vec<Bez3>]) -> Vec<BezPath> {
    shape
        .iter()
        .map(|contour| {
            let mut path = BezPath::new();
            if let Some(first) = contour.first() {
                path.move_to(first.a);
                for segment in contour {
                    path.curve_to(segment.b, segment.c, segment.d);
                }
                path.close_path();
            }

            path
        })
        .collect()
}

/// A shape of kurbo paths as cubics, one list per contour.
#[must_use]
pub fn paths_to_shape(paths: &[BezPath]) -> Vec<Vec<Bez3>> {
    paths
        .iter()
        .map(|path| {
            path.segments()
                .map(|segment| match segment {
                    PathSeg::Line(line) => straight_bez3(line.p0, line.p1),
                    PathSeg::Quad(quad) => {
                        let c1 = quad.p0 + (quad.p1 - quad.p0) * (2.0 / 3.0);
                        let c2 = quad.p2 + (quad.p1 - quad.p2) * (2.0 / 3.0);

                        Bez3::new(quad.p0, c1, c2, quad.p2)
                    }
                    PathSeg::Cubic(cubic) => Bez3::new(cubic.p0, cubic.p1, cubic.p2, cubic.p3),
                })
                .collect()
        })
        .collect()
}

/// Applies a boolean operation to two shapes of cubics.
#[must_use]
pub fn boolean_shapes(a: &[Vec<Bez3>], b: &[Vec<Bez3>], op: BinaryOp) -> Vec<Vec<Bez3>> {
    // A zero-area operand contributes nothing, and the sweep can turn one into
    // a zero-width seam in the result.
    let a = crate::boolean::drop_degenerate(a);
    let b = crate::boolean::drop_degenerate(b);
    let a_paths = shape_to_paths(&a);
    let b_paths = shape_to_paths(&b);
    let mut a_path = BezPath::new();
    for path in &a_paths {
        a_path.extend(path.iter());
    }
    let mut b_path = BezPath::new();
    for path in &b_paths {
        b_path.extend(path.iter());
    }

    if let Ok(contours) = boolean(&a_path, &b_path, op) {
        let mut input = a;
        input.extend_from_slice(&b);

        crate::boolean::annex(
            &crate::boolean::clean_shape(&paths_to_shape(&contours)),
            &input,
        )
    } else {
        // The sweep refused the input. Keep both shapes, but still drop the
        // degenerate arcs they may carry.
        let mut fallback = a;
        fallback.extend_from_slice(&b);

        crate::boolean::clean_shape(&fallback)
    }
}

fn derivative_from_finite_difference(curve: &impl CurveArc, t: f64) -> Vec2 {
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

fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
