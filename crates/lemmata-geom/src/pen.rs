//! Pen expansion: a profile swept along a skeleton.
//!
//! Ported from `spiro-pen-expand.mjs`. Each profile edge traces its own copy of
//! the skeleton, adjacent traces are joined into curved strips, and the strips
//! are unioned. `PenSpiroGeometry.toBezArcs` unions with typo-geom;
//! [`pen_spiro_paths`] unions with the linesweeper wrapper in `boolean`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::similar_names,
    clippy::suboptimal_flops,
    clippy::too_many_arguments
)]

use kurbo::{Affine, BezPath, Point, Vec2};

use crate::{
    expand::{MonoKnot, linreg},
    spiro::{Knot, KnotType, SpiroArc, spiro_arcs},
};

/// Subdivision delta for the strips, from `curve-util.mjs`.
const GEOMETRY_PRECISION: f64 = 0.25;
/// Stop count cap in `subdivideKnotPair`.
const MAX_STOPS: usize = 16;

/// A skeleton knot with a pen profile, one offset per profile edge.
#[derive(Clone, Debug)]
pub struct PenKnot {
    pub ty: KnotType,
    pub x: f64,
    pub y: f64,
    /// `None` marks the knot unimportant, which is interpolated between its
    /// neighbours for every edge.
    pub profile: Option<Vec<Vec2>>,
}

impl PenKnot {
    #[must_use]
    pub const fn new(ty: KnotType, x: f64, y: f64, profile: Option<Vec<Vec2>>) -> Self {
        Self { ty, x, y, profile }
    }

    fn with_gizmo(&self, gizmo: Affine) -> Self {
        let point = gizmo * Point::new(self.x, self.y);
        Self {
            ty: self.ty,
            x: point.x,
            y: point.y,
            profile: self.profile.clone(),
        }
    }
}

/// The expander behind `SpiroPenGeometry`.
pub struct PenExpander {
    gizmo: Affine,
    closed: bool,
    knots_u: Vec<PenKnot>,
    knots_t: Vec<PenKnot>,
    profile_edges: usize,
    traces: Vec<Vec<SpiroArc>>,
}

impl PenExpander {
    #[must_use]
    pub fn new(gizmo: Affine, profile_edges: usize, closed: bool, knots: &[PenKnot]) -> Self {
        Self {
            gizmo,
            closed,
            knots_u: knots.to_vec(),
            knots_t: knots.iter().map(|k| k.with_gizmo(gizmo)).collect(),
            profile_edges,
            traces: Vec::new(),
        }
    }

    /// The raw strips, before union. One closed contour per subdivision piece.
    pub fn geometry(&mut self) -> Vec<BezPath> {
        self.trace_all();
        let mut contours = Vec::new();
        for i in 0..self.traces.len() {
            let next = (i + 1) % self.traces.len();
            if self.traces[i].len() != self.traces[next].len() {
                continue;
            }

            for j in 0..self.traces[i].len() {
                make_profiled_stroke(&mut contours, &self.traces[i][j], &self.traces[next][j]);
            }
        }

        contours
    }

    fn trace_all(&mut self) {
        for edge in 0..self.profile_edges {
            self.calculate_shifted_profile(edge);
        }
    }

    fn calculate_shifted_profile(&mut self, edge: usize) {
        let inverse = self.gizmo.inverse();
        let mut trace_t = Vec::with_capacity(self.knots_t.len());
        let mut trace_u = Vec::with_capacity(self.knots_t.len());
        for knot in &self.knots_t {
            let tr_t = Self::get_trace(knot, edge);
            let tr_u = untransform(&inverse, &tr_t);
            trace_t.push(tr_t);
            trace_u.push(tr_u);
        }
        self.interpolate_unimportant_trace_knots(&mut trace_u, &mut trace_t);
        let arcs = spiro_arcs(&to_knots(&trace_t), self.closed);
        self.traces.push(arcs);
    }

    fn get_trace(knot: &PenKnot, edge: usize) -> MonoKnot {
        match &knot.profile {
            Some(profile) => {
                MonoKnot::new(
                    knot.ty,
                    knot.x + profile[edge].x,
                    knot.y + profile[edge].y,
                    false,
                )
            }
            None => MonoKnot::new(knot.ty, knot.x, knot.y, true),
        }
    }

    fn interpolate_unimportant_trace_knots(
        &self,
        trace_u: &mut [MonoKnot],
        trace_t: &mut [MonoKnot],
    ) {
        let mut first: Option<usize> = None;
        let mut last: Option<usize> = None;

        for i in 0..trace_u.len() {
            if trace_u[i].unimportant {
                continue;
            }
            if let Some(previous) = last {
                self.interpolate_range(trace_u, trace_t, previous, i);
            }
            if first.is_none() {
                first = Some(i);
            }
            last = Some(i);
        }

        if let (Some(first), Some(last)) = (first, last) {
            self.interpolate_range(trace_u, trace_t, last, first);
        }
    }

    fn interpolate_range(
        &self,
        trace_u: &mut [MonoKnot],
        trace_t: &mut [MonoKnot],
        last: usize,
        next: usize,
    ) {
        let count = if next > last {
            next - last
        } else {
            trace_u.len() - last + next
        };
        for offset in 1..count {
            let i = (last + offset) % trace_u.len();

            let (rx_b, ry_b) = (self.knots_u[last].x, self.knots_u[last].y);
            let (ux_b, uy_b) = (trace_u[last].x, trace_u[last].y);
            let (rx_c, ry_c) = (self.knots_u[i].x, self.knots_u[i].y);
            let (rx_a, ry_a) = (self.knots_u[next].x, self.knots_u[next].y);
            let (ux_a, uy_a) = (trace_u[next].x, trace_u[next].y);

            let x = linreg(rx_b, ux_b, rx_a, ux_a, rx_c);
            let y = linreg(ry_b, uy_b, ry_a, uy_a, ry_c);
            trace_u[i].x = x;
            trace_u[i].y = y;

            let point = self.gizmo * Point::new(x, y);
            trace_t[i].x = point.x;
            trace_t[i].y = point.y;
        }
    }
}

/// The full `SpiroPenGeometry` path: expand, then union the strips.
#[must_use]
pub fn pen_spiro_paths(
    gizmo: Affine,
    profile_edges: usize,
    closed: bool,
    knots: &[PenKnot],
) -> Vec<BezPath> {
    let mut expander = PenExpander::new(gizmo, profile_edges, closed, knots);
    let contours = expander.geometry();

    // Union the strips one at a time, the way the upstream operand stack does.
    let mut accumulated: Vec<BezPath> = Vec::new();
    for contour in &contours {
        if accumulated.is_empty() {
            accumulated.push(contour.clone());
            continue;
        }
        let mut a = BezPath::new();
        for path in &accumulated {
            a.extend(path.iter());
        }
        let mut b = BezPath::new();
        b.extend(contour.iter());

        accumulated = if let Ok(contours) = crate::boolean::union(&a, &b) {
            contours
        } else {
            let mut fallback = accumulated;
            fallback.push(contour.clone());

            fallback
        };
    }

    accumulated
}

fn make_profiled_stroke(contours: &mut Vec<BezPath>, forward: &SpiroArc, backward: &SpiroArc) {
    let (forward_parts, backward_parts) = subdivide_knot_pair(forward, backward);
    for (fwd, bwd) in forward_parts.iter().zip(&backward_parts) {
        let [a1, b1, c1, d1] = fwd.cubic();
        let [a2, b2, c2, d2] = bwd.cubic();

        let mut path = BezPath::new();
        path.move_to(a1);
        path.curve_to(b1, c1, d1);
        path.line_to(d2);
        path.curve_to(c2, b2, a2);
        path.close_path();
        contours.push(path);
    }
}

fn subdivide_knot_pair(forward: &SpiroArc, backward: &SpiroArc) -> (Vec<SpiroArc>, Vec<SpiroArc>) {
    let mut forward_parts = Vec::new();
    let mut backward_parts = Vec::new();
    for stops in 1..MAX_STOPS {
        forward_parts = uniform_subdivide(forward, stops);
        backward_parts = uniform_subdivide(backward, stops);
        let flat = forward_parts
            .iter()
            .chain(&backward_parts)
            .all(|part| part.bend <= GEOMETRY_PRECISION);

        if flat {
            break;
        }
    }

    (forward_parts, backward_parts)
}

fn uniform_subdivide(arc: &SpiroArc, stops: usize) -> Vec<SpiroArc> {
    let mut sink = Vec::new();
    let mut current = arc.clone();
    let mut stops = stops;
    while stops > 1 {
        let (left, right) = current.subdivide(1.0 / stops as f64);
        sink.push(left);
        current = right;
        stops -= 1;
    }
    sink.push(current);

    sink
}

fn to_knots(knots: &[MonoKnot]) -> Vec<Knot> {
    knots.iter().map(MonoKnot::to_knot).collect()
}

fn untransform(inverse: &Affine, knot: &MonoKnot) -> MonoKnot {
    let point = *inverse * Point::new(knot.x, knot.y);
    MonoKnot::new(knot.ty, point.x, point.y, knot.unimportant)
}
