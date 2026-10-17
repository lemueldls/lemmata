//! The spiro outline path: solve, simplify, fit.
//!
//! Ported from `spiro-to-outline.mjs`. `SpiroGeometry.toBezArcs` calls this:
//! spiro arcs are grouped into runs that end at important knots, runs shorter
//! than a hair are dropped, and every remaining arc is fitted to cubics. A run
//! is flattened before fitting, matching typo-geom, which handles a combined arc
//! by recursing into its segments rather than fitting the combination as one
//! curve.

#![allow(clippy::cast_precision_loss, clippy::suboptimal_flops)]

use kurbo::{Affine, Vec2};

use crate::expand::MonoKnot;
use crate::fit;
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
    Sequence(Vec<SpiroArc>),
}

impl Run {
    fn arcs(&self) -> &[SpiroArc] {
        match self {
            Self::Single(arc) => std::slice::from_ref(arc),
            Self::Sequence(arcs) => arcs,
        }
    }
}

impl CurveArc for SpiroArc {
    fn eval(&self, t: f64) -> kurbo::Point {
        SpiroArc::eval(self, t)
    }

    fn derivative(&self, t: f64) -> Vec2 {
        SpiroArc::derivative(self, t)
    }
}

use crate::fit::CurveArc;

/// `spiroToBezArcsWithSimplification`: the outline of a spiro skeleton, as one
/// contour of cubics, transformed by `gizmo`.
#[must_use]
pub fn spiro_to_bez_arcs(knots: &[MonoKnot], closed: bool, gizmo: Affine) -> Vec<Vec<Bez3>> {
    let plain: Vec<Knot> = knots.iter().map(MonoKnot::to_knot).collect();
    let arcs = spiro_arcs(&plain, closed);
    let runs = simplify(knots, arcs);

    let mut segments = Vec::new();
    for run in &runs {
        for arc in run.arcs() {
            fit::fit_arc_into(arc, OCCURRENT_PRECISION, &mut segments);
        }
    }
    fit::close_contour(&mut segments);
    for segment in &mut segments {
        *segment = transform_bez3(gizmo, *segment);
    }
    vec![segments]
}

/// Groups raw arcs into runs, flushing at every important knot.
fn simplify(knots: &[MonoKnot], arcs: Vec<SpiroArc>) -> Vec<Run> {
    let mut runs = Vec::new();
    let mut ongoing: Vec<SpiroArc> = Vec::new();

    for (processed, arc) in (1usize..).zip(arcs) {
        ongoing.push(arc);
        if let Some(knot) = knots.get(processed)
            && !knot.unimportant
        {
            flush(&mut runs, &mut ongoing);
        }
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
        let arc_length: f64 = ongoing.iter().map(|arc| arc.arc_length).sum();
        if arc_length > LENGTH_EPSILON {
            runs.push(Run::Sequence(std::mem::take(ongoing)));
        }
    }
    ongoing.clear();
}

/// Applies an affine transform to a cubic.
pub(crate) fn transform_bez3(gizmo: Affine, cubic: Bez3) -> Bez3 {
    Bez3::new(
        gizmo * cubic.a,
        gizmo * cubic.b,
        gizmo * cubic.c,
        gizmo * cubic.d,
    )
}
