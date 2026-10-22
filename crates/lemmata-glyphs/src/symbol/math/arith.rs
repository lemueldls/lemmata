//! The arithmetic signs from `symbol/math/arith.ptl`.
//!
//! Ported: `plus`. The minus, the Hebrew plus, the enclosures and the derived
//! signs are not ported.

use lemmata_geom::Geometry;
use lemmata_ptl::{
    metrics::Metrics,
    shapes::{self, Design},
    spiro_kit::Kit,
};

/// The arithmetic signs this module builds.
pub struct Arith<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl Arith<'_> {
    /// `PlusShape`.
    fn plus(&self) -> Geometry {
        let metrics = self.metrics;
        let sw = Some(metrics.operator_stroke);

        lemmata_ptl::glyph::union(vec![
            shapes::hbar_m(
                self.kit,
                metrics,
                metrics.sb,
                metrics.right_sb,
                metrics.symbol_mid,
                sw,
            )
            .geometry(kurbo::Affine::IDENTITY),
            shapes::vbar_m(
                self.kit,
                metrics,
                metrics.middle,
                metrics.plus_top,
                metrics.plus_bot,
                metrics.operator_stroke,
            )
            .geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        vec![("plus".to_string(), self.plus())]
    }
}
