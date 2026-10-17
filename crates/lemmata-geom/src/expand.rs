//! Contrast expansion of spiro skeletons.
//!
//! Ported from `spiro-expand.mjs` and the `DiSpiroGeometry` driver in
//! `index.mjs`. A skeleton knot carries a left and right stroke width. The
//! expander first estimates a tangent at each knot by running spiro over the
//! skeleton, then offsets each knot by its widths along the normal. Knots marked
//! unimportant get interpolated between their important neighbours.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::similar_names,
    clippy::suboptimal_flops
)]

use kurbo::{Affine, BezPath, Point, Vec2};

use crate::spiro::{Knot, KnotType, SpiroArc, spiro_arcs, spiro_path};

/// Finite-difference step for tangent estimation, same as `curve-util.mjs`.
const FINITE_DIFFERENCE_DELTA: f64 = 1.0 / 65536.0;

/// A skeleton knot with stroke widths.
#[derive(Clone, Debug)]
pub struct BiKnot {
    pub ty: KnotType,
    pub x: f64,
    pub y: f64,
    /// Left width.
    pub d1: f64,
    /// Right width.
    pub d2: f64,
    /// Normal forced by the glyph definition, in transformed space.
    pub proposed_normal: Option<Vec2>,
    pub unimportant: bool,
    /// Estimated tangent, filled in by the normal pass.
    pub orig_tangent: Option<Vec2>,
}

impl BiKnot {
    #[must_use]
    pub const fn new(ty: KnotType, x: f64, y: f64, d1: f64, d2: f64) -> Self {
        Self {
            ty,
            x,
            y,
            d1,
            d2,
            proposed_normal: None,
            unimportant: false,
            orig_tangent: None,
        }
    }

    fn with_gizmo(&self, gizmo: Affine) -> Self {
        let point = gizmo * Point::new(self.x, self.y);
        Self {
            ty: self.ty,
            x: point.x,
            y: point.y,
            d1: self.d1,
            d2: self.d2,
            proposed_normal: self.proposed_normal.map(|v| apply_vector(gizmo, v)),
            unimportant: self.unimportant,
            orig_tangent: self.orig_tangent.map(|v| apply_vector(gizmo, v)),
        }
    }

    const fn to_mono(&self) -> MonoKnot {
        MonoKnot::new(self.ty, self.x, self.y, self.unimportant)
    }
}

/// A skeleton knot without widths, as the spiro evaluator sees it.
#[derive(Clone, Debug)]
pub struct MonoKnot {
    pub ty: KnotType,
    pub x: f64,
    pub y: f64,
    pub unimportant: bool,
}

impl MonoKnot {
    #[must_use]
    pub const fn new(ty: KnotType, x: f64, y: f64, unimportant: bool) -> Self {
        Self {
            ty,
            x,
            y,
            unimportant,
        }
    }

    pub(crate) const fn to_knot(&self) -> Knot {
        Knot::new(self.x, self.y, self.ty)
    }

    pub(crate) const fn reverse_type(&mut self) {
        self.ty = match self.ty {
            KnotType::Left => KnotType::Right,
            KnotType::Right => KnotType::Left,
            other => other,
        };
    }
}

/// Left and right sides of the expanded skeleton.
pub struct ExpandResult {
    pub lhs: Vec<MonoKnot>,
    pub rhs: Vec<MonoKnot>,
    pub lhs_untransformed: Vec<MonoKnot>,
    pub rhs_untransformed: Vec<MonoKnot>,
}

/// The expander behind `DiSpiroGeometry`.
pub struct SpiroExpander {
    gizmo: Affine,
    contrast: f64,
    closed: bool,
    knots_u: Vec<BiKnot>,
    knots_t: Vec<BiKnot>,
}

impl SpiroExpander {
    #[must_use]
    pub fn new(gizmo: Affine, contrast: f64, closed: bool, knots: &[BiKnot]) -> Self {
        Self {
            gizmo,
            contrast,
            closed,
            knots_u: knots.to_vec(),
            knots_t: knots.iter().map(|k| k.with_gizmo(gizmo)).collect(),
        }
    }

    /// First normal pass, over the skeleton itself.
    pub fn initialize_normals(&mut self) {
        let stage: Vec<MonoKnot> = self.knots_t.iter().map(BiKnot::to_mono).collect();
        rectify_normals(&stage, &mut self.knots_t, self.closed);
    }

    /// One refinement pass over the mid-line of the expanded sides.
    pub fn iterate_normals(&mut self) -> f64 {
        let center = self.pass2_knots();
        rectify_normals(&center, &mut self.knots_t, self.closed)
    }

    fn pass2_knots(&mut self) -> Vec<MonoKnot> {
        let expanded = self.expand();
        let mut middles = Vec::with_capacity(self.knots_t.len());
        for (j, knot) in self.knots_t.iter().enumerate() {
            middles.push(MonoKnot::new(
                knot.ty,
                mix(expanded.lhs[j].x, expanded.rhs[j].x, 0.5),
                mix(expanded.lhs[j].y, expanded.rhs[j].y, 0.5),
                knot.unimportant,
            ));
        }
        middles
    }

    /// Offsets every important knot along its normal.
    pub fn expand(&mut self) -> ExpandResult {
        let n = self.knots_t.len();
        // Every knot keeps its type and importance, important or not: the
        // solver reads them, and the outline simplifier groups runs by them.
        let mut lhs_t: Vec<MonoKnot> = self
            .knots_t
            .iter()
            .map(|knot| MonoKnot::new(knot.ty, 0.0, 0.0, knot.unimportant))
            .collect();
        let mut rhs_t = lhs_t.clone();
        let mut lhs_u = lhs_t.clone();
        let mut rhs_u = lhs_t.clone();

        let inverse = self.gizmo.inverse();
        for j in 0..n {
            let knot = self.knots_t[j].clone();
            if knot.unimportant {
                continue;
            }
            let (dx, dy) = if let Some(normal) = knot.proposed_normal {
                (normal.x, normal.y)
            } else {
                let tangent = knot.orig_tangent.unwrap_or(Vec2::ZERO);
                (normal_x(tangent, self.contrast), normal_y(tangent))
            };

            lhs_t[j].x = knot.x + knot.d1 * dx;
            lhs_t[j].y = knot.y + knot.d1 * dy;
            rhs_t[j].x = knot.x - knot.d2 * dx;
            rhs_t[j].y = knot.y - knot.d2 * dy;

            lhs_u[j] = untransform(&inverse, &lhs_t[j]);
            rhs_u[j] = untransform(&inverse, &rhs_t[j]);
        }

        self.interpolate_unimportant_knots(&mut lhs_t, &mut rhs_t, &mut lhs_u, &mut rhs_u);
        ExpandResult {
            lhs: lhs_t,
            rhs: rhs_t,
            lhs_untransformed: lhs_u,
            rhs_untransformed: rhs_u,
        }
    }

    fn interpolate_unimportant_knots(
        &self,
        lhs_t: &mut [MonoKnot],
        rhs_t: &mut [MonoKnot],
        lhs_u: &mut [MonoKnot],
        rhs_u: &mut [MonoKnot],
    ) {
        let mut first_important: Option<usize> = None;
        let mut last_important: Option<usize> = None;

        for j in 0..self.knots_u.len() {
            if self.knots_u[j].unimportant {
                continue;
            }
            if let Some(last) = last_important {
                self.interpolate_range(lhs_t, rhs_t, lhs_u, rhs_u, last, j);
            }
            if first_important.is_none() {
                first_important = Some(j);
            }
            last_important = Some(j);
        }

        if let (Some(first), Some(last)) = (first_important, last_important) {
            self.interpolate_range(lhs_t, rhs_t, lhs_u, rhs_u, last, first);
        }
    }

    fn interpolate_range(
        &self,
        lhs_t: &mut [MonoKnot],
        rhs_t: &mut [MonoKnot],
        lhs_u: &mut [MonoKnot],
        rhs_u: &mut [MonoKnot],
        j_before: usize,
        j_after: usize,
    ) {
        let len = lhs_t.len();
        let count = if j_after > j_before {
            j_after - j_before
        } else {
            len - j_before + j_after
        };
        for offset in 1..count {
            let j = (j_before + offset) % len;
            let knot_before = &self.knots_u[j_before];
            let knot = &self.knots_u[j];
            let knot_after = &self.knots_u[j_after];

            lhs_u[j].x = linreg(
                knot_before.x,
                lhs_u[j_before].x,
                knot_after.x,
                lhs_u[j_after].x,
                knot.x,
            );
            lhs_u[j].y = linreg(
                knot_before.y,
                lhs_u[j_before].y,
                knot_after.y,
                lhs_u[j_after].y,
                knot.y,
            );
            rhs_u[j].x = linreg(
                knot_before.x,
                rhs_u[j_before].x,
                knot_after.x,
                rhs_u[j_after].x,
                knot.x,
            );
            rhs_u[j].y = linreg(
                knot_before.y,
                rhs_u[j_before].y,
                knot_after.y,
                rhs_u[j_after].y,
                knot.y,
            );

            lhs_t[j] = transform(&self.gizmo, &lhs_u[j]);
            rhs_t[j] = transform(&self.gizmo, &rhs_u[j]);
        }
    }
}

/// Runs the normal passes and returns both sides.
#[must_use]
pub fn di_spiro_expand(
    gizmo: Affine,
    contrast: f64,
    closed: bool,
    knots: &[BiKnot],
) -> ExpandResult {
    let mut expander = SpiroExpander::new(gizmo, contrast, closed, knots);
    expander.initialize_normals();
    for _ in 0..8 {
        if expander.iterate_normals() < 1e-8 {
            break;
        }
    }
    expander.expand()
}

/// The full `DiSpiroGeometry` path: expand, then draw both sides.
///
/// Closed skeletons become two closed contours. Open skeletons become one
/// closed contour that runs down one side and back up the other.
#[must_use]
pub fn di_spiro_paths(
    gizmo: Affine,
    contrast: f64,
    closed: bool,
    knots: &[BiKnot],
    delta: f64,
) -> Vec<BezPath> {
    let expanded = di_spiro_expand(gizmo, contrast, closed, knots);
    let mut lhs = expanded.lhs_untransformed;
    let mut rhs = expanded.rhs_untransformed;
    for knot in &mut rhs {
        knot.reverse_type();
    }
    rhs.reverse();

    let mut paths = if closed {
        vec![
            spiro_path(&to_knots(&lhs), true, delta),
            spiro_path(&to_knots(&rhs), true, delta),
        ]
    } else {
        if !lhs.is_empty() {
            let last = lhs.len() - 1;
            lhs[0].ty = KnotType::Corner;
            lhs[last].ty = KnotType::Corner;
        }
        if !rhs.is_empty() {
            let last = rhs.len() - 1;
            rhs[0].ty = KnotType::Corner;
            rhs[last].ty = KnotType::Corner;
        }
        let all: Vec<Knot> = lhs
            .iter()
            .chain(rhs.iter())
            .map(MonoKnot::to_knot)
            .collect();
        vec![spiro_path(&all, true, delta)]
    };

    for path in &mut paths {
        path.apply_affine(gizmo);
    }
    paths
}

fn to_knots(knots: &[MonoKnot]) -> Vec<Knot> {
    knots.iter().map(MonoKnot::to_knot).collect()
}

fn transform(gizmo: &Affine, knot: &MonoKnot) -> MonoKnot {
    let point = *gizmo * Point::new(knot.x, knot.y);
    MonoKnot::new(knot.ty, point.x, point.y, knot.unimportant)
}

/// Applies the linear part of an affine transform to a vector.
fn apply_vector(gizmo: Affine, v: Vec2) -> Vec2 {
    let c = gizmo.as_coeffs();
    Vec2::new(c[0] * v.x + c[2] * v.y, c[1] * v.x + c[3] * v.y)
}

fn untransform(inverse: &Affine, knot: &MonoKnot) -> MonoKnot {
    let point = *inverse * Point::new(knot.x, knot.y);
    MonoKnot::new(knot.ty, point.x, point.y, knot.unimportant)
}

fn rectify_normals(stage: &[MonoKnot], knots_t: &mut [BiKnot], closed: bool) -> f64 {
    let knots = to_knots(stage);
    let arcs = spiro_arcs(&knots, closed);
    let mut processed = 1usize;
    let mut total_delta = 0.0;
    for arc in &arcs {
        if processed == 1 {
            update_knot_tangent(&mut knots_t[0], start_tangent(arc), &mut total_delta);
        }
        if let Some(knot) = knots_t.get_mut(processed) {
            update_knot_tangent(knot, end_tangent(arc), &mut total_delta);
        }
        processed += 1;
    }
    if processed == 0 {
        return 0.0;
    }
    total_delta / processed as f64
}

fn update_knot_tangent(knot: &mut BiKnot, d: Vec2, total_delta: &mut f64) {
    match knot.orig_tangent {
        Some(t) if is_tangent_valid(t) => {
            *total_delta += (d.x - t.x).powi(2) + (d.y - t.y).powi(2);
        }
        _ => *total_delta += 4.0,
    }
    knot.orig_tangent = Some(d);
}

fn start_tangent(arc: &SpiroArc) -> Vec2 {
    let d = Vec2::new(arc.derive_x0, arc.derive_y0);
    if is_tangent_valid(d) {
        return d;
    }
    let fallback = derivative_forward(arc, 0.0);
    if is_tangent_valid(fallback) {
        return fallback;
    }
    chord_tangent(arc)
}

fn end_tangent(arc: &SpiroArc) -> Vec2 {
    let d = Vec2::new(arc.derive_x1, arc.derive_y1);
    if is_tangent_valid(d) {
        return d;
    }
    let fallback = derivative_backward(arc, 1.0);
    if is_tangent_valid(fallback) {
        return fallback;
    }
    chord_tangent(arc)
}

fn chord_tangent(arc: &SpiroArc) -> Vec2 {
    let c0 = arc.eval(0.0);
    let c1 = arc.eval(1.0);
    let chord = Vec2::new(c1.x - c0.x, c1.y - c0.y);
    if is_tangent_valid(chord) {
        return chord;
    }
    let length = if arc.arc_length != 0.0 && arc.arc_length.is_finite() {
        arc.arc_length
    } else {
        1.0
    };
    Vec2::new(length, 0.0)
}

fn derivative_forward(arc: &SpiroArc, t: f64) -> Vec2 {
    let center = arc.eval(t);
    let forward1 = arc.eval(t + FINITE_DIFFERENCE_DELTA);
    let forward2 = arc.eval(t + 2.0 * FINITE_DIFFERENCE_DELTA);
    Vec2::new(
        (-3.0 * center.x + 4.0 * forward1.x - forward2.x) / (2.0 * FINITE_DIFFERENCE_DELTA),
        (-3.0 * center.y + 4.0 * forward1.y - forward2.y) / (2.0 * FINITE_DIFFERENCE_DELTA),
    )
}

fn derivative_backward(arc: &SpiroArc, t: f64) -> Vec2 {
    let center = arc.eval(t);
    let backward1 = arc.eval(t - FINITE_DIFFERENCE_DELTA);
    let backward2 = arc.eval(t - 2.0 * FINITE_DIFFERENCE_DELTA);
    Vec2::new(
        (3.0 * center.x - 4.0 * backward1.x + backward2.x) / (2.0 * FINITE_DIFFERENCE_DELTA),
        (3.0 * center.y - 4.0 * backward1.y + backward2.y) / (2.0 * FINITE_DIFFERENCE_DELTA),
    )
}

const fn is_tangent_valid(d: Vec2) -> bool {
    d.x.is_finite() && d.y.is_finite()
}

fn normal_x(tangent: Vec2, contrast: f64) -> f64 {
    contrast * (-tangent.y / tangent.hypot())
}

fn normal_y(tangent: Vec2) -> f64 {
    tangent.x / tangent.hypot()
}

fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

pub(crate) fn linreg(x0: f64, y0: f64, x1: f64, y1: f64, x: f64) -> f64 {
    y0 + (y1 - y0) * (x - x0) / (x1 - x0)
}
