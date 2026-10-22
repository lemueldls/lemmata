//! The slashes from `symbol/punctuation/slashes-and-number-sign.ptl`.
//!
//! Ported: `slash`, `backslash`, the joining halves and `doubleSlash`. The
//! number sign, the big forms, the overbars and the reverse halves are not
//! ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{metrics::Metrics, shapes::Design, spiro_kit::Kit, util::Waveform};

/// The slash glyphs this module builds.
pub struct Slashes<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl Slashes<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    /// `SlashShape`.
    fn slash_shape(&self, l: f64, r: f64, t: f64, b: f64, w: f64) -> Geometry {
        let a = Waveform::Sawtooth.eval((r - l - w) / (t - b));
        let cor = 1.0 / (1.0 - a * a).sqrt();
        let half = self.hsw_to_v(0.5 * cor * w);

        spiro_geometry(self.kit.spiro_outline(vec![
            self.kit.knot(KnotType::Corner, r - half, t, Vec::new()),
            self.kit.knot(KnotType::Corner, r + half, t, Vec::new()),
            self.kit.knot(KnotType::Corner, l + half, b, Vec::new()),
            self.kit.knot(KnotType::Corner, l - half, b, Vec::new()),
        ]))
    }

    /// `BackslashShape`.
    fn backslash_shape(&self, l: f64, r: f64, t: f64, b: f64, w: f64) -> Geometry {
        let a = Waveform::Sawtooth.eval((r - l - w) / (t - b));
        let cor = 1.0 / (1.0 - a * a).sqrt();
        let half = self.hsw_to_v(0.5 * cor * w);

        spiro_geometry(self.kit.spiro_outline(vec![
            self.kit.knot(KnotType::Corner, l - half, t, Vec::new()),
            self.kit.knot(KnotType::Corner, l + half, t, Vec::new()),
            self.kit.knot(KnotType::Corner, r + half, b, Vec::new()),
            self.kit.knot(KnotType::Corner, r - half, b, Vec::new()),
        ]))
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let metrics = self.metrics;
        let left = metrics.sb + metrics.half_stroke;
        let right = metrics.width - metrics.sb - metrics.half_stroke;
        let top = metrics.paren_top;
        let bottom = metrics.paren_bot;
        let stroke = metrics.stroke;
        let double = metrics
            .advice_stroke(3.0, 1.0, 1.0)
            .max(metrics.width * 0.15);

        vec![
            (
                "slash".to_string(),
                self.slash_shape(left, right, top, bottom, stroke),
            ),
            (
                "backslash".to_string(),
                self.backslash_shape(left, right, top, bottom, stroke),
            ),
            (
                "slash.left".to_string(),
                self.slash_shape(left, metrics.width, top, bottom, stroke),
            ),
            (
                "slash.right".to_string(),
                self.slash_shape(0.0, right, top, bottom, stroke),
            ),
            (
                "doubleSlash".to_string(),
                Geometry::Combine(vec![
                    self.slash_shape(left - double, right - double, top, bottom, double),
                    self.slash_shape(left + double, right + double, top, bottom, double),
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
