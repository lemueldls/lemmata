//! The lowercase q from `letter/latin/lower-q.ptl`.
//!
//! Ported: the eared body with the normal terminal and no serifs, which the
//! default sans plan selects. The cut and earless bodies, the tailed terminals
//! and the serifs are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::{
    letter_model::DivFrame,
    metrics::Metrics,
    shapes::{self, Design, OBarContext, OBarRing},
    spiro_kit::Kit,
};

/// The lowercase q forms this module builds.
pub struct LowerQ<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerQ<'_> {
    /// `EaredBody` with `TERMINAL-NORMAL`.
    fn straight_serifless(&self) -> Geometry {
        let metrics = self.metrics;
        let df = DivFrame::new(metrics.clone(), Some(1.0), None, None);
        let ctx = OBarContext {
            kit: self.kit,
            metrics,
            design: self.design,
        };
        let body = shapes::obar_right_shape(&ctx, &OBarRing {
            top: metrics.xh,
            bot: 0.0,
            left: df.left_sb(),
            right: df.right_sb(),
            sw: df.mvs,
            fine: df.shoulder_fine,
            ada: df.small_arch_depth_a,
            adb: df.small_arch_depth_b,
        });
        let tail = shapes::vbar_r(
            self.kit,
            metrics,
            df.right_sb(),
            metrics.descender,
            metrics.xh,
            df.mvs,
        )
        .geometry(kurbo::Affine::IDENTITY);

        Geometry::Combine(vec![body, tail])
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        vec![("q.straightSerifless".to_string(), self.straight_serifless())]
    }
}
