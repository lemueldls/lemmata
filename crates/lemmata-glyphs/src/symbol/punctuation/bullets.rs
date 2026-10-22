//! The bullets from `symbol/punctuation/bullets.ptl`.
//!
//! Ported: `bullet`, `triangularBullet` and `whiteBullet`. The capture holds
//! `bullet`; the other two are built but ungated.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    metrics::Metrics,
    shapes::{self, Design},
    spiro_kit::Kit,
};

/// The bullet glyphs this module builds.
pub struct Bullets<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl Bullets<'_> {
    fn radius(&self) -> f64 {
        (self.metrics.width * 2.0 / 7.0).max(self.metrics.advice_stroke(3.0, 1.0, 1.0))
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let metrics = self.metrics;
        let radius = self.radius();
        let middle = metrics.middle;
        let mid = metrics.symbol_mid;
        let triangle = radius * 1.5;
        let x_mid = middle - triangle / 6.0;
        let white = metrics.advice_stroke(5.0, 1.0, 1.0).min(radius * 0.5);

        vec![
            (
                "bullet".to_string(),
                spiro_geometry(shapes::dot_at(
                    self.kit,
                    metrics,
                    middle,
                    mid,
                    radius - metrics.o,
                )),
            ),
            (
                "triangularBullet".to_string(),
                spiro_geometry(self.kit.spiro_outline(vec![
                    self.kit.knot(
                        KnotType::Corner,
                        x_mid + triangle - metrics.o,
                        mid,
                        Vec::new(),
                    ),
                    self.kit.knot(
                        KnotType::Corner,
                        x_mid - triangle / 2.0,
                        mid - triangle / 2.0 * 3.0_f64.sqrt(),
                        Vec::new(),
                    ),
                    self.kit.knot(
                        KnotType::Corner,
                        x_mid - triangle / 2.0,
                        mid + triangle / 2.0 * 3.0_f64.sqrt(),
                        Vec::new(),
                    ),
                ])),
            ),
            (
                "whiteBullet".to_string(),
                lemmata_ptl::glyph::difference(vec![
                    spiro_geometry(shapes::dot_at(
                        self.kit,
                        metrics,
                        middle,
                        mid,
                        radius - metrics.o,
                    )),
                    spiro_geometry(shapes::dot_at(
                        self.kit,
                        metrics,
                        middle,
                        mid,
                        radius - metrics.o - white,
                    )),
                ]),
            ),
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
