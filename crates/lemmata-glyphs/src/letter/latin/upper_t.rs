//! The capital `T` from `letter/latin/upper-t.ptl`.
//!
//! Ported: the three `T` configs, serifless, motion-serifed and serifed. The
//! hooked, Cyrillic, long-right-leg and turned forms are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::shapes::{self, Design, VSerifSide};
use lemmata_ptl::spiro_kit::Kit;

/// `advanceScaleT` is 1 for the reference plan.
const ADVANCE_SCALE_T: f64 = 1.0;

/// The `T` families this module builds.
pub struct UpperT<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl UpperT<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    /// `VSwToH`.
    fn vsw_to_h(&self, value: f64) -> f64 {
        value / self.metrics.hv_contrast
    }

    /// `TShape`.
    fn shape(&self, df: &DivFrame, top: f64, top_serifs: bool, bottom_serif: bool) -> Geometry {
        let metrics = self.metrics;
        let l = df.left_sb() + 0.75 * metrics.ox;
        let r = df.right_sb() - 0.75 * metrics.ox;
        let mut parts = vec![
            tagged(
                shapes::vbar_m(self.kit, metrics, df.middle(), 0.0, top, metrics.stroke)
                    .geometry(kurbo::Affine::IDENTITY),
            ),
            tagged(
                shapes::hbar_t(self.kit, metrics, l, df.middle(), top, Some(metrics.stroke))
                    .geometry(kurbo::Affine::IDENTITY),
            ),
            tagged(
                shapes::hbar_t(self.kit, metrics, df.middle(), r, top, Some(metrics.stroke))
                    .geometry(kurbo::Affine::IDENTITY),
            ),
        ];
        if top_serifs {
            let sw = metrics.v_jut_stroke.min(
                self.vsw_to_h(0.625 * ((df.middle() - self.hsw_to_v(metrics.half_stroke)) - l)),
            );
            parts.push(tagged(
                shapes::vserif(
                    self.kit,
                    metrics,
                    VSerifSide::DownRight,
                    r,
                    top,
                    metrics.v_jut,
                    Some(sw),
                )
                .geometry(kurbo::Affine::IDENTITY),
            ));
            parts.push(tagged(
                shapes::vserif(
                    self.kit,
                    metrics,
                    VSerifSide::DownLeft,
                    l,
                    top,
                    metrics.v_jut,
                    Some(sw),
                )
                .geometry(kurbo::Affine::IDENTITY),
            ));
        }
        if bottom_serif {
            parts.push(tagged(
                shapes::hserif_mb(
                    self.kit,
                    metrics,
                    df.middle(),
                    0.0,
                    metrics.mid_jut_center,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            ));
        }

        Geometry::Combine(parts)
    }

    /// One `T.<suffix>`.
    #[must_use]
    pub fn glyph(&self, adws: f64, top_serifs: bool, bottom_serif: bool) -> Geometry {
        let df = DivFrame::new(self.metrics.clone(), Some(adws), None, None);

        self.shape(&df, self.metrics.cap, top_serifs, bottom_serif)
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        CONFIGS
            .iter()
            .map(|(suffix, adws, top, bottom)| {
                (format!("T.{suffix}"), self.glyph(*adws, *top, *bottom))
            })
            .collect()
    }
}

/// The config table, in `Object.entries` order.
pub const CONFIGS: &[(&str, f64, bool, bool)] = &[
    ("serifless", 1.0, false, false),
    ("motionSerifed", ADVANCE_SCALE_T, true, false),
    ("serifed", ADVANCE_SCALE_T, true, true),
];

fn tagged(geometry: Geometry) -> Geometry {
    Geometry::Tagged(Box::new(geometry))
}
