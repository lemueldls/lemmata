//! The number sign from `symbol/punctuation/slashes-and-number-sign.ptl`.
//!
//! Ported: the four upright configs. The slanted configs, the joining (`jr`)
//! forms and the cap-height selector are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::{
    glyph,
    metrics::Metrics,
    shapes::{self, Design},
    spiro_kit::Kit,
    util::mix,
};

/// The number sign glyphs this module builds.
pub struct NumberSign<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl NumberSign<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    fn fine(&self) -> f64 {
        self.metrics.advice_stroke(2.75, 1.0, 1.0)
    }

    fn hsp(&self, l: f64, r: f64) -> f64 {
        let fine = self.fine();

        ((r - l) - self.hsw_to_v(2.0 * fine)) * 0.3
    }

    /// `UprightHBars`.
    fn hbars(&self, lx: f64, r: f64, top: f64, bot: f64) -> Geometry {
        let metrics = self.metrics;

        Geometry::Combine(vec![
            shapes::hbar_m(self.kit, metrics, lx, r, mix(top, bot, 0.3), None)
                .geometry(kurbo::Affine::IDENTITY),
            shapes::hbar_m(self.kit, metrics, lx, r, mix(top, bot, 0.7), None)
                .geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// `UprightHBarsOpen`.
    fn hbars_open(&self, l: f64, r: f64, top: f64, bot: f64, p: f64) -> Geometry {
        let fine = self.fine();
        let hsp = self.hsp(l, r);
        let mask = spiro_geometry(shapes::rect(
            self.kit,
            top,
            bot,
            l + hsp + fine * (0.5 + p),
            r - hsp - fine * (0.5 + p),
        ));

        glyph::difference(vec![self.hbars(l, r, top, bot), mask])
    }

    /// `UprightVBars`.
    fn vbars(&self, l: f64, r: f64, top: f64, bot: f64, p: f64) -> Geometry {
        let metrics = self.metrics;
        let fine = self.fine();
        let hsp = self.hsp(l, r);

        Geometry::Combine(vec![
            shapes::vbar_l(self.kit, metrics, l + hsp + p * fine, bot, top, fine)
                .geometry(kurbo::Affine::IDENTITY),
            shapes::vbar_r(self.kit, metrics, r - hsp - p * fine, bot, top, fine)
                .geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// One number sign config.
    fn glyph(&self, top: f64, bot: f64, open: bool) -> Geometry {
        let fine = self.fine();
        let l = self.metrics.sb - fine / 2.0;
        let r = self.metrics.width - self.metrics.sb + fine / 2.0;
        let p = if open { 1.0 / 8.0 } else { 0.0 };
        let hbars = if open {
            self.hbars_open(l, r, top, bot, p)
        } else {
            self.hbars(l, r, top, bot)
        };

        Geometry::Combine(vec![hbars, self.vbars(l, r, top, bot, p)])
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let metrics = self.metrics;
        let normal = (mix(0.0, metrics.cap, 0.975), mix(0.0, metrics.cap, 0.025));
        let tall = (
            mix(metrics.oper_top, metrics.paren_top, 0.8),
            mix(metrics.oper_bot, metrics.paren_bot, 0.8),
        );

        vec![
            (
                "numberSign.upright".to_string(),
                self.glyph(normal.0, normal.1, false),
            ),
            (
                "numberSign.uprightOpen".to_string(),
                self.glyph(normal.0, normal.1, true),
            ),
            (
                "numberSign.uprightTall".to_string(),
                self.glyph(tall.0, tall.1, false),
            ),
            (
                "numberSign.uprightOpenTall".to_string(),
                self.glyph(tall.0, tall.1, true),
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
