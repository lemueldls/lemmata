//! The small punctuation from `symbol/punctuation/small.ptl`.
//!
//! Ported: the period and the colon in their round and square forms. The
//! comma, semicolon, raised and stacked forms and the three-dot punctuation
//! are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::{
    metrics::Metrics,
    shapes::{self, Design},
    spiro_kit::Kit,
};

/// The period and colon glyphs this module builds.
pub struct Small<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl Small<'_> {
    /// `period.<form>`.
    fn period(&self, square: bool) -> Geometry {
        let metrics = self.metrics;
        let middle = metrics.middle;
        let radius = metrics.period_radius
            * if square {
                metrics.square_dot_scalar
            } else {
                1.0
            };
        let centre = metrics.period_radius
            * if square {
                metrics.square_dot_scalar
            } else {
                1.0
            };
        let radius = if square { centre } else { radius - metrics.o };

        if square {
            spiro_geometry(shapes::rect(
                self.kit,
                centre + radius,
                centre - radius,
                middle - radius,
                middle + radius,
            ))
        } else {
            spiro_geometry(shapes::dot_at(self.kit, metrics, middle, centre, radius))
        }
    }

    /// `xhPeriod.<form>`.
    fn xh_period(&self, square: bool) -> Geometry {
        let metrics = self.metrics;
        let middle = metrics.middle;
        let centre = if square {
            metrics.xh - metrics.period_radius * metrics.square_dot_scalar
        } else {
            metrics.xh - metrics.period_radius
        };
        let radius = if square {
            metrics.period_radius * metrics.square_dot_scalar
        } else {
            metrics.period_radius - metrics.o
        };

        if square {
            spiro_geometry(shapes::rect(
                self.kit,
                centre + radius,
                centre - radius,
                middle - radius,
                middle + radius,
            ))
        } else {
            spiro_geometry(shapes::dot_at(self.kit, metrics, middle, centre, radius))
        }
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for (suffix, square) in [("round", false), ("square", true)] {
            out.push((format!("period.{suffix}"), self.period(square)));
            out.push((format!("xhPeriod.{suffix}"), self.xh_period(square)));
            out.push((
                format!("colon.{suffix}"),
                Geometry::Combine(vec![self.period(square), self.xh_period(square)]),
            ));
        }

        out
    }
}

fn spiro_geometry(outline: lemmata_ptl::spiro_kit::SpiroOutline) -> Geometry {
    Geometry::Spiro {
        gizmo: kurbo::Affine::IDENTITY,
        closed: outline.closed,
        knots: outline.knots,
    }
}
