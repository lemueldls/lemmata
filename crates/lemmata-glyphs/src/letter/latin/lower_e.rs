//! The lowercase e from `letter/latin/lower-e.ptl`.
//!
//! Ported: the flat-crossbar form, which the default sans plan selects. The
//! rounded, serifed and reversed forms are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    control::{Item, UserControlKnot},
    letter_model::DivFrame,
    metrics::Metrics,
    shapes::{self, ArchArgs, ArchParams, Design, FlatSide, HookArgs, HookParams},
    spiro_kit::Kit,
    util::mix,
};

/// The lowercase e forms this module builds.
pub struct LowerE<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerE<'_> {
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

    /// `SmallEShape` for the flat crossbar.
    fn flat_crossbar(&self) -> Geometry {
        let metrics = self.metrics;
        let design = self.design;
        let df = DivFrame::new(metrics.clone(), Some(1.0), None, None);
        let top = metrics.xh;
        let stroke = metrics.advice_stroke2(2.0, 3.0, top, 1.0);
        let hook = metrics.a_hook;
        let ada = df.small_arch_depth_a;
        let adb = df.small_arch_depth_b;
        let y_bar_bottom = mix(0.0, top, design.e_bar_pos) - stroke / 2.0;
        let x_outer = df.right_sb() - metrics.ox;
        let tiny = metrics.tiny;
        let params = ArchParams {
            superness: design.superness,
            tan_slope: metrics.tan_slope,
            hv_contrast: metrics.hv_contrast,
            o: metrics.o,
            correction_o_mid_x: metrics.correction_o_mid_x,
        };

        let bar = shapes::hbar_b(
            self.kit,
            metrics,
            df.left_sb() + metrics.ox + stroke / 2.0,
            df.right_sb() - metrics.ox - stroke / 2.0,
            y_bar_bottom,
            Some(stroke),
        )
        .geometry(kurbo::Affine::IDENTITY);

        let side = FlatSide {
            x: df.left_sb(),
            d: 0.0,
            u: top,
            ada,
            adb,
            ox: metrics.ox,
            af: Vec::new(),
        };
        let side_items = shapes::flatside_ld(self.kit, &side);
        let arch_before = UserControlKnot {
            ty: KnotType::Right,
            x: x_outer,
            y: (top - adb).max(y_bar_bottom + tiny),
            af: Vec::new(),
        };
        let arch_after = shapes::knot_of(&side_items[0]);
        let end_before = shapes::knot_of(side_items.last().expect("the side has knots"));
        let small_e_hook = hook.min(mix(
            stroke,
            mix(0.0, top, design.e_bar_pos) - stroke / 2.0,
            0.554,
        ));
        let end_after = UserControlKnot {
            ty: KnotType::G4,
            x: df.right_sb() - 0.5 * metrics.ox,
            y: small_e_hook,
            af: Vec::new(),
        };
        let lemmata_ptl::shapes::HookOut {
            items: end_hook,
            terminal,
        } = shapes::hook_end(
            self.kit,
            &end_before,
            &end_after,
            &HookArgs {
                y_ref: 0.0,
                sw: stroke,
                sw_terminal: stroke,
                is_tail: false,
                suppress_swash: false,
                overshoot: metrics.o,
            },
            &self.hook_params(),
        );

        let mut items = vec![
            Item::Af(self.kit.widths_lhs(Some(stroke))),
            self.kit.knot(
                KnotType::Left,
                x_outer,
                y_bar_bottom,
                self.kit.heading(shapes::upward(metrics.hv_contrast)),
            ),
            self.kit.knot(
                arch_before.ty,
                arch_before.x,
                arch_before.y,
                arch_before.af.clone(),
            ),
        ];
        items.extend(shapes::arch(
            self.kit,
            &ArchArgs {
                y: top,
                p: None,
                sw: stroke,
                compact: false,
                o: metrics.o,
                angle_pre: None,
                angle_post: None,
                blend_pre: None,
                blend_post: None,
                sw_before: None,
                sw_after: None,
                lhs: true,
            },
            &arch_before,
            &arch_after,
            &params,
        ));
        items.extend(side_items);
        items.extend(end_hook);
        items.push(
            self.kit
                .knot(terminal.ty, terminal.x, terminal.y, terminal.af),
        );

        Geometry::Combine(vec![
            bar,
            self.kit.dispiro(items).geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        vec![("e.flatCrossbar".to_string(), self.flat_crossbar())]
    }
}
