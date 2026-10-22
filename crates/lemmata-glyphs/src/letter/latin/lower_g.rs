//! The lowercase g from `letter/latin/lower-g.ptl`.
//!
//! Ported: the single-storey bent-hook serifless form, which the default sans
//! plan selects. The double-storey, flat-hook, earless and serifed forms are
//! not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    control::{Item, UserControlKnot},
    letter_model::DivFrame,
    metrics::Metrics,
    shapes::{self, Design, HookArgs, HookParams, OBarContext, OBarRing},
    spiro_kit::Kit,
};

/// The lowercase g forms this module builds.
pub struct LowerG<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerG<'_> {
    const fn hook_params(&self) -> HookParams {
        HookParams {
            superness: self.design.superness,
            tan_slope: self.metrics.tan_slope,
            contrast: self.metrics.contrast,
            correction_o_mid_x: self.metrics.correction_o_mid_x,
            ox_hook: self.metrics.ox_hook,
            hook: self.metrics.hook,
            slope_angle: 0.0,
            is_italic: false,
        }
    }

    /// `SingleStorey.BentHook`.
    fn bent_hook(&self, df: &DivFrame, y0: f64) -> Geometry {
        let metrics = self.metrics;
        let sw = df.mvs;
        let before = UserControlKnot {
            ty: KnotType::Right,
            x: df.right_sb(),
            y: metrics.descender + df.small_arch_depth_a,
            af: Vec::new(),
        };
        let after = UserControlKnot {
            ty: KnotType::G4,
            x: df.left_sb(),
            y: metrics.descender + metrics.s_hook,
            af: Vec::new(),
        };
        let lemmata_ptl::shapes::HookOut {
            items: hook,
            terminal,
        } = shapes::hook_end(
            self.kit,
            &before,
            &after,
            &HookArgs {
                y_ref: metrics.descender,
                sw,
                sw_terminal: sw,
                is_tail: false,
                suppress_swash: false,
                overshoot: metrics.o,
            },
            &self.hook_params(),
        );

        let mut items = vec![
            Item::Af(self.kit.widths_rhs(Some(sw))),
            self.kit.knot(
                KnotType::Left,
                df.right_sb(),
                y0,
                self.kit.heading(shapes::downward(metrics.hv_contrast)),
            ),
            self.kit.knot(before.ty, before.x, before.y, before.af),
        ];
        items.extend(hook);
        items.push(
            self.kit
                .knot(terminal.ty, terminal.x, terminal.y, terminal.af),
        );

        self.kit.dispiro(items).geometry(kurbo::Affine::IDENTITY)
    }

    /// `SingleStorey.SeriflessBody`.
    fn body(&self, df: &DivFrame, top: f64) -> Geometry {
        let ring = OBarRing {
            top,
            bot: 0.0,
            left: df.left_sb(),
            right: df.right_sb(),
            sw: df.mvs,
            fine: df.shoulder_fine,
            ada: df.small_arch_depth_a,
            adb: df.small_arch_depth_b,
        };
        let ctx = OBarContext {
            kit: self.kit,
            metrics: self.metrics,
            design: self.design,
        };

        shapes::obar_right_shape(&ctx, &ring)
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let df = DivFrame::new(self.metrics.clone(), Some(1.0), None, None);
        let body = self.body(&df, self.metrics.xh);
        let hook = self.bent_hook(&df, self.metrics.xh);

        vec![(
            "g.singleStoreyBentHookSerifless".to_string(),
            Geometry::Combine(vec![body, hook]),
        )]
    }
}
