//! The dashes from `symbol/punctuation/dashes.ptl`.
//!
//! Ported: `hyphen`, `figureDash`, `enDash`, `dblHyphen`, `overline` and the
//! three `underscore` settings. The open boxes, the Armenian hyphen, the
//! em dash widths, the spear and the derived compositions are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::{
    letter_model::DivFrame,
    metrics::Metrics,
    shapes::{self, Design},
    spiro_kit::Kit,
    util::mix,
};

/// `DesignParameters.equal_wideness`, from `meta/aesthetics.ptl`.
const EQUAL_WIDENESS: f64 = 0.075;

/// The dash glyphs this module builds.
pub struct Dashes<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl Dashes<'_> {
    fn df_f(&self) -> DivFrame {
        DivFrame::new(self.metrics.clone(), Some(1.0), None, None)
    }

    /// `hyphen`.
    fn hyphen(&self) -> Geometry {
        let metrics = self.metrics;
        let df = self.df_f();

        shapes::hbar_m(
            self.kit,
            metrics,
            df.left_sb(),
            df.right_sb(),
            metrics.symbol_mid,
            None,
        )
        .geometry(kurbo::Affine::IDENTITY)
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let metrics = self.metrics;
        let right_sb = metrics.width - metrics.sb;
        let equal_half = (metrics.oper_top - metrics.oper_bot) * 2.0 * EQUAL_WIDENESS;
        let mut out = vec![("hyphen".to_string(), self.hyphen())];
        out.push((
            "figureDash".to_string(),
            shapes::hbar_m(
                self.kit,
                metrics,
                metrics.sb,
                right_sb,
                metrics.symbol_mid,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        ));
        out.push((
            "enDash".to_string(),
            shapes::hbar_m(
                self.kit,
                metrics,
                mix(0.0, metrics.sb, 0.5),
                mix(metrics.width, right_sb, 0.5),
                metrics.symbol_mid,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        ));
        out.push((
            "overline".to_string(),
            shapes::hbar_t(self.kit, metrics, metrics.sb, right_sb, metrics.cap, None)
                .geometry(kurbo::Affine::IDENTITY),
        ));
        out.push((
            "dblHyphen".to_string(),
            Geometry::Combine(vec![
                shapes::hbar_m(
                    self.kit,
                    metrics,
                    metrics.sb,
                    right_sb,
                    metrics.symbol_mid + equal_half,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
                shapes::hbar_m(
                    self.kit,
                    metrics,
                    metrics.sb,
                    right_sb,
                    metrics.symbol_mid - equal_half,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            ]),
        ));
        for (suffix, y) in [
            ("aboveBaseline", 0.0),
            ("high", -metrics.stroke),
            ("low", metrics.descender),
        ] {
            out.push((
                format!("underscore.{suffix}"),
                shapes::hbar_b(self.kit, metrics, metrics.sb, right_sb, y, None)
                    .geometry(kurbo::Affine::IDENTITY),
            ));
        }

        out
    }
}
