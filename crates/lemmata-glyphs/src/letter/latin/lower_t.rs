//! The lowercase t from `letter/latin/lower-t.ptl`.
//!
//! Ported: the bent-hook form at the ascender height, which the default sans
//! plan selects. The narrow, diagonal-tailed, flat-hook and hookless forms are
//! not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    control::{Item, UserControlKnot},
    letter_model::DivFrame,
    metrics::Metrics,
    shapes::{self, Design, HookArgs, HookParams},
    spiro_kit::Kit,
};

/// The lowercase t forms this module builds.
pub struct LowerT<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerT<'_> {
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

    /// `BentHook.Body` for the balanced symmetry and the ascender height.
    fn bent_hook(&self) -> Geometry {
        let metrics = self.metrics;
        let df = DivFrame::new(metrics.clone(), Some(1.0), None, None);
        let adws = df.adws();
        let top = metrics.ascender;
        let stroke = metrics.stroke;
        let hsw = |value: f64| value * metrics.hv_contrast;
        let bar_left = df.middle() - adws * (adws * metrics.t_balance) - hsw(metrics.half_stroke);
        let cross_ref = bar_left + adws * metrics.t_balance2 + hsw(metrics.half_stroke);
        let x_hook = bar_left
            + ((df.right_sb() - df.left_sb()) * 0.75 + hsw(metrics.quarter_stroke))
                .max(hsw(2.25 * stroke));
        let mid = (bar_left + x_hook) / 2.0;
        let adb = (1.2 * (mid + metrics.correction_o_mid_x * stroke - bar_left))
            .clamp(metrics.hook, metrics.arch_depth_b);
        let bar = shapes::h_cross_bar_top(
            self.kit,
            cross_ref - metrics.long_jut * adws.sqrt(),
            cross_ref + metrics.long_jut * adws.sqrt(),
            metrics.xh,
            None,
        )
        .geometry(kurbo::Affine::IDENTITY);

        let before = UserControlKnot {
            ty: KnotType::Right,
            x: bar_left,
            y: adb,
            af: Vec::new(),
        };
        let after = UserControlKnot {
            ty: KnotType::G4,
            x: x_hook,
            y: metrics.hook,
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
                bar_left,
                top,
                self.kit.heading(shapes::downward(metrics.hv_contrast)),
            ),
            self.kit.knot(before.ty, before.x, before.y, before.af),
        ];
        items.extend(hook);
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
        vec![("t.bentHook".to_string(), self.bent_hook())]
    }
}
