//! The interpuncts from `symbol/punctuation/interpuncts.ptl`.
//!
//! Ported: the round and square forms. The plan selects the round one for the
//! middle dot, which is what the capture holds.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::{
    metrics::Metrics,
    shapes::{self, Design},
    spiro_kit::Kit,
};

/// The interpunct glyphs this module builds.
pub struct Interpuncts<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl Interpuncts<'_> {
    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let metrics = self.metrics;
        let middle = metrics.middle;
        let mid = metrics.symbol_mid;
        let period = metrics.period_radius;
        let square = period * metrics.square_dot_scalar;

        vec![
            (
                "interpunct.round".to_string(),
                spiro_geometry(shapes::dot_at(
                    self.kit,
                    metrics,
                    middle,
                    mid,
                    period - metrics.o,
                )),
            ),
            ("interpunct.square".to_string(), Geometry::Spiro {
                gizmo: kurbo::Affine::IDENTITY,
                closed: shapes::rect(
                    self.kit,
                    mid + square,
                    mid - square,
                    middle - square,
                    middle + square,
                )
                .closed,
                knots: shapes::rect(
                    self.kit,
                    mid + square,
                    mid - square,
                    middle - square,
                    middle + square,
                )
                .knots,
            }),
        ]
    }
}

fn spiro_geometry(outline: lemmata_ptl::spiro_kit::SpiroOutline) -> Geometry {
    Geometry::Spiro {
        gizmo: kurbo::Affine::IDENTITY,
        closed: outline.closed,
        knots: outline.knots,
    }
}
