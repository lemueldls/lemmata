//! The spiro outline path: solve, simplify, fit.
//!
//! Ported from `spiro-to-outline.mjs`. `SpiroGeometry.toBezArcs` calls this:
//! spiro arcs are grouped into runs that end at important knots, then each run
//! is fitted to cubics and transformed by the gizmo.

#![allow(
    clippy::cast_precision_loss,
    clippy::needless_range_loop,
    clippy::suboptimal_flops
)]

use kurbo::{Affine, Point, Vec2};

use crate::expand::MonoKnot;
use crate::fit::{CurveArc, fit_contour};
use crate::spiro::{Knot, SpiroArc, spiro_arcs};
use crate::stroke::Bez3;

/// Fit tolerance for the cubic conversion, from `curve-util.mjs`.
const OCCURRENT_PRECISION: f64 = 1.0 / 16.0;
/// Runs shorter than this are dropped by the simplifier.
const LENGTH_EPSILON: f64 = 1e-6;

/// Spiro arcs grouped into a run between two important knots.
#[derive(Clone, Debug)]
enum Run {
    Single(SpiroArc),
    Sequence(SequenceArc),
}

impl CurveArc for Run {
    fn eval(&self, t: f64) -> Point {
        match self {
            Self::Single(arc) => arc.eval(t),
            Self::Sequence(sequence) => sequence.eval(t),
        }
    }

    fn derivative(&self, t: f64) -> Vec2 {
        match self {
            Self::Single(arc) => arc.derivative(t),
            Self::Sequence(sequence) => sequence.derivative(t),
        }
    }
}

/// A chain of spiro arcs, parameterized by normalized arc length.
#[derive(Clone, Debug)]
struct SequenceArc {
    segments: Vec<SpiroArc>,
    stops: Vec<f64>,
    end_adjustments: Vec<Vec2>,
    total_length: f64,
}

impl SequenceArc {
    fn new(arcs: Vec<SpiroArc>) -> Self {
        let mut segments = Vec::with_capacity(arcs.len());
        for arc in arcs {
            if arc.arc_length > 0.0 {
                segments.push(arc);
            }
        }

        let mut stops = vec![0.0; segments.len()];
        let mut total_length = 0.0;
        for (index, segment) in segments.iter().enumerate() {
            stops[index] = total_length;
            total_length += segment.arc_length;
        }
        for stop in &mut stops {
            *stop /= total_length;
        }

        let mut end_adjustments = vec![Vec2::ZERO; segments.len()];
        for i in 0..segments.len() {
            if i + 1 < segments.len() {
                let a = segments[i].eval(1.0);
                let b = segments[i + 1].eval(0.0);
                end_adjustments[i] = Vec2::new(b.x - a.x, b.y - a.y);
            }
        }

        Self {
            segments,
            stops,
            end_adjustments,
            total_length,
        }
    }

    fn is_empty(&self) -> bool {
        self.segments.is_empty() || self.total_length == 0.0
    }

    fn locate(&self, t: f64) -> (usize, f64) {
        let index = segment_search(&self.stops, t);
        let t_before = self.stops[index];
        let t_next = if index < self.stops.len() - 1 {
            self.stops[index + 1]
        } else {
            1.0
        };
        let mut t_relative = (t - t_before) / (t_next - t_before);
        if t <= t_before {
            t_relative = 0.0;
        } else if t >= t_next {
            t_relative = 1.0;
        }
        (index, t_relative)
    }

    fn eval(&self, t: f64) -> Point {
        let (index, t_relative) = self.locate(t);
        let base = self.segments[index].eval(t_relative);
        let adjustment = self.end_adjustments[index];
        Point::new(
            base.x + t_relative * adjustment.x,
            base.y + t_relative * adjustment.y,
        )
    }

    fn derivative(&self, t: f64) -> Vec2 {
        let (index, t_relative) = self.locate(t);
        let t_before = self.stops[index];
        let t_next = if index < self.stops.len() - 1 {
            self.stops[index + 1]
        } else {
            1.0
        };
        let scale = 1.0 / (t_next - t_before);
        let d = self.segments[index].derivative(t_relative);
        let adjustment = self.end_adjustments[index];
        Vec2::new(
            (d.x + adjustment.x) * scale,
            (d.y + adjustment.y) * scale,
        )
    }
}

impl CurveArc for SpiroArc {
    fn eval(&self, t: f64) -> Point {
        SpiroArc::eval(self, t)
    }

    fn derivative(&self, t: f64) -> Vec2 {
        SpiroArc::derivative(self, t)
    }
}

/// `spiroToBezArcsWithSimplification`: the outline of a spiro skeleton, as one
/// contour of cubics, transformed by `gizmo`.
#[must_use]
pub fn spiro_to_bez_arcs(knots: &[MonoKnot], closed: bool, gizmo: Affine) -> Vec<Vec<Bez3>> {
    let plain: Vec<Knot> = knots.iter().map(MonoKnot::to_knot).collect();
    let arcs = spiro_arcs(&plain, closed);
    let runs = simplify(knots, arcs);
    let mut fitted = fit_contour(&runs, OCCURRENT_PRECISION);
    for segment in &mut fitted {
        *segment = transform_bez3(gizmo, *segment);
    }
    vec![fitted]
}

/// Groups raw arcs into runs, flushing at every important knot.
fn simplify(knots: &[MonoKnot], arcs: Vec<SpiroArc>) -> Vec<Run> {
    let mut runs = Vec::new();
    let mut ongoing: Vec<SpiroArc> = Vec::new();
    let mut processed = 1usize;

    for arc in arcs {
        ongoing.push(arc);
        if let Some(knot) = knots.get(processed)
            && !knot.unimportant
        {
            flush(&mut runs, &mut ongoing);
        }
        processed += 1;
    }
    flush(&mut runs, &mut ongoing);
    runs
}

fn flush(runs: &mut Vec<Run>, ongoing: &mut Vec<SpiroArc>) {
    if ongoing.is_empty() {
        return;
    }
    if ongoing.len() == 1 {
        let arc = ongoing[0].clone();
        if arc.arc_length > LENGTH_EPSILON {
            runs.push(Run::Single(arc));
        }
    } else {
        let sequence = SequenceArc::new(std::mem::take(ongoing));
        if !sequence.is_empty() && sequence.total_length > LENGTH_EPSILON {
            runs.push(Run::Sequence(sequence));
        }
    }
    ongoing.clear();
}

fn segment_search(stops: &[f64], t: f64) -> usize {
    if t <= 0.0 || stops.len() == 1 {
        return 0;
    }
    if t >= stops[stops.len() - 1] {
        return stops.len() - 1;
    }
    let (mut l, mut r) = (0, stops.len());
    while l < r {
        let m = usize::midpoint(l, r);
        if stops[m] > t {
            r = m;
        } else {
            l = m + 1;
        }
    }
    r - 1
}

/// Applies the linear-and-translate part of an affine transform to a cubic.
pub(crate) fn transform_bez3(gizmo: Affine, cubic: Bez3) -> Bez3 {
    Bez3::new(
        gizmo * cubic.a,
        gizmo * cubic.b,
        gizmo * cubic.c,
        gizmo * cubic.d,
    )
}
