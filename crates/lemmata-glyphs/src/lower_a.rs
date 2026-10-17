//! The double-storey `a` from `letter/latin/lower-a.ptl`.
//!
//! `DoubleStorey.Serifless` is the shape RefA-Regular's `a` uses: a union of
//! `HookAndBar` and `Arc`. This module carries those two bodies and the
//! variant's stroke advice, which is what the port needs before the rest of the
//! file's variant table can be filled in.
//!
//! The single-storey bodies and the `a`-derived glyphs (the turned, script and
//! accented forms) are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript. The names are
// upstream's, where `ada` and `adb` are the two arch depths.
#![allow(clippy::suboptimal_flops, clippy::similar_names)]

use crate::control::Item;
use crate::letter_model::DivFrame;
use crate::metrics::Metrics;
use crate::shapes::{
    ArchArgs, ArchParams, Design, HookArgs, HookParams, arch, arch_depth_b_clamped, downward,
    leftward, rightward, upward, ysmooth_mid_l,
};
use crate::spiro_kit::{Dispiro, Kit};
use kurbo::Vec2;
use lemmata_geom::KnotType;

/// The hook styles `DoubleStorey` takes.
pub const HOOK_SLAB_NONE: i32 = 0;
pub const HOOK_SLAB_CLASSICAL: i32 = 1;
pub const HOOK_SLAB_INWARD: i32 = 2;
pub const HOOK_HALF: i32 = 3;

/// The double-storey `a`'s two bodies.
pub struct DoubleStorey<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl DoubleStorey<'_> {
    /// `Sw`: the stroke advice this variant uses.
    #[must_use]
    pub fn sw(&self) -> f64 {
        self.df.advice_stroke2(2.0, 3.0, self.metrics.xh, 1.0)
    }

    /// `SmoothA`.
    #[must_use]
    pub fn smooth_a(&self) -> f64 {
        let sw = self.sw();
        let ad = self.metrics.arch_depth * self.metrics.stroke_width_blend(0.9, 0.81, Some(sw));
        crate::shapes::arch_depth_a_clamped(
            self.metrics.xh,
            0.0,
            Some(self.df.arch_depth_a_of(ad, Some(sw))),
            Some(self.df.arch_depth_b_of(ad, Some(sw))),
            self.metrics.arch_depth_a,
            self.metrics.arch_depth_b,
        )
    }

    /// `SmoothB`.
    #[must_use]
    pub fn smooth_b(&self) -> f64 {
        let sw = self.sw();
        let ad = self.metrics.arch_depth * self.metrics.stroke_width_blend(0.9, 0.81, Some(sw));
        arch_depth_b_clamped(
            self.metrics.xh,
            0.0,
            Some(self.df.arch_depth_a_of(ad, Some(sw))),
            Some(self.df.arch_depth_b_of(ad, Some(sw))),
            self.metrics.arch_depth_a,
            self.metrics.arch_depth_b,
        )
    }

    const fn left_sb(&self) -> f64 {
        self.df.sb
    }

    const fn right_sb(&self) -> f64 {
        self.df.width - self.df.sb
    }

    const fn middle(&self) -> f64 {
        0.5 * self.df.width
    }

    const fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

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

    const fn arch_params(&self) -> ArchParams {
        ArchParams {
            superness: self.design.superness,
            tan_slope: self.metrics.tan_slope,
            hv_contrast: self.metrics.hv_contrast,
            o: self.metrics.o,
            correction_o_mid_x: self.metrics.correction_o_mid_x,
        }
    }

    /// `HookAndBar`, the hook and the bar it sits on.
    ///
    /// Only `HOOK-SLAB-NONE` is ported; the other hook styles add serifs or a
    /// half-height hook.
    #[must_use]
    pub fn hook_and_bar(
        &self,
        hook_style: i32,
        y0: f64,
        stroke: Option<f64>,
        fine: Option<f64>,
    ) -> Dispiro {
        let metrics = self.metrics;
        let sw = stroke.unwrap_or_else(|| self.sw());
        let fine = fine.unwrap_or(sw);

        let bar_right_x = self.right_sb() - self.hsw_to_v(sw - fine);
        let hook_left_x = self.left_sb();
        let bar_y = (metrics.xh - self.smooth_b())
            .max(y0 + metrics.tiny + self.hsw_to_v((metrics.tan_slope * fine).abs()));

        let mut items = vec![Item::Af(self.kit.widths_rhs(Some(sw)))];

        match hook_style {
            HOOK_SLAB_NONE => {
                // The g4 knot the hook's terminal replaces, and the flat knot
                // the hook resolves against.
                let terminal_knot = crate::control::UserControlKnot {
                    ty: KnotType::G4,
                    x: hook_left_x,
                    y: metrics.xh - metrics.a_hook,
                    af: Vec::new(),
                };
                let straight = crate::control::UserControlKnot {
                    ty: KnotType::Left,
                    x: bar_right_x,
                    y: bar_y,
                    af: Vec::new(),
                };
                let hook = crate::shapes::hook_start(
                    self.kit,
                    &terminal_knot,
                    &straight,
                    &HookArgs {
                        y_ref: metrics.xh,
                        sw,
                        sw_terminal: sw,
                        is_tail: false,
                        suppress_swash: false,
                        overshoot: metrics.o,
                    },
                    &self.hook_params(),
                );
                let crate::shapes::HookOut {
                    items: hook_items,
                    terminal,
                } = hook;
                items.push(
                    self.kit
                        .knot(terminal.ty, terminal.x, terminal.y, terminal.af),
                );
                items.extend(hook_items);
            }
            other => panic!("hook style {other} is not ported"),
        }

        items.push(
            self.kit
                .knot(KnotType::Left, bar_right_x, bar_y, Vec::new()),
        );
        items.push(self.kit.knot(
            KnotType::Right,
            bar_right_x,
            y0,
            self.kit.heading(downward(metrics.hv_contrast)),
        ));
        self.kit.dispiro(items)
    }

    /// `Arc`, the shoulder on the left.
    ///
    /// Only kind 0 is ported, which is what the serifless variant uses.
    #[must_use]
    pub fn arc(
        &self,
        kind: i32,
        rise: f64,
        stroke: Option<f64>,
        fine: Option<f64>,
        bbd: Option<f64>,
    ) -> Dispiro {
        let metrics = self.metrics;
        let sw = stroke.unwrap_or_else(|| self.sw());
        let fine = fine.unwrap_or(self.df.shoulder_fine);
        let bbd = bbd.unwrap_or(0.0);

        let bar_top = crate::util::mix(0.0, metrics.xh, self.design.a_bar_pos)
            + sw * (0.5 + self.design.a_bar_stroke_adj);
        let ada = self
            .df
            .arch_depth_a_of(metrics.small_arch_depth * 0.9, Some(sw));
        // Computed for parity with upstream; only `ada` reaches kind 0.
        let _adb = self
            .df
            .arch_depth_b_of(metrics.small_arch_depth * 0.9, Some(sw));
        let ada2 = metrics.arch_depth_a_of(metrics.small_arch_depth * self.design.a_bar_pos, None);
        let adb2 = metrics.arch_depth_b_of(metrics.small_arch_depth * self.design.a_bar_pos, None);
        let left_slope_s = 0.1 * (self.df.width / metrics.half_upm);
        let left_slope = left_slope_s - metrics.tan_slope;

        let mut items = vec![Item::Af(self.kit.widths_lhs(Some(sw)))];
        items.push(self.kit.knot(
            KnotType::Left,
            self.right_sb() + metrics.o,
            bar_top,
            self.kit.heading(leftward()),
        ));
        items.push(self.kit.knot(
            KnotType::Right,
            crate::util::mix(self.right_sb(), self.middle(), 0.9),
            bar_top,
            Vec::new(),
        ));
        items.push(self.kit.archv(None, None));

        let arch_x = self.left_sb() + metrics.ox;
        let arch_y = ysmooth_mid_l(
            bar_top,
            0.0,
            Some(ada2),
            Some(adb2),
            metrics.arch_depth_a,
            metrics.arch_depth_b,
        ) - self.hsw_to_v(left_slope_s) * metrics.smooth_adjust;
        let arch_knot = crate::control::UserControlKnot {
            ty: KnotType::G4,
            x: arch_x,
            y: arch_y,
            af: self.kit.heading(Vec2::new(metrics.hv_contrast, left_slope)),
        };
        items.push(
            self.kit
                .knot(arch_knot.ty, arch_knot.x, arch_knot.y, arch_knot.af.clone()),
        );

        match kind {
            0 => {
                let flat_x = self.right_sb() - bbd - self.hsw_to_v(sw - fine);
                let flat_y =
                    (bar_top - metrics.tiny - self.hsw_to_v((fine * metrics.tan_slope).abs()))
                        .min(0.0 + ada);
                let straight = crate::control::UserControlKnot {
                    ty: KnotType::Left,
                    x: flat_x,
                    y: flat_y,
                    af: self.kit.widths_lhs(Some(fine)),
                };
                items.extend(arch(
                    self.kit,
                    &ArchArgs {
                        y: 0.0,
                        p: None,
                        sw,
                        compact: false,
                        o: metrics.o,
                        angle_pre: None,
                        angle_post: None,
                        sw_before: Some(sw),
                        sw_after: Some(fine),
                        lhs: true,
                    },
                    &arch_knot,
                    &straight,
                    &self.arch_params(),
                ));
                items.push(
                    self.kit
                        .knot(straight.ty, straight.x, straight.y, straight.af),
                );
                items.push(self.kit.knot(
                    KnotType::Right,
                    flat_x,
                    bar_top,
                    self.kit.heading(upward(metrics.hv_contrast)),
                ));
            }
            other => panic!("arc kind {other} is not ported"),
        }

        let _ = rise;
        self.kit.dispiro(items)
    }

    /// `DoubleStorey.Serifless`: the union's two operands.
    #[must_use]
    pub fn serifless(&self, hook_style: i32, stroke: Option<f64>) -> Vec<Dispiro> {
        vec![
            self.hook_and_bar(hook_style, 0.0, stroke, None),
            self.arc(0, 0.0, stroke, None, None),
        ]
    }

    /// `DoubleStorey.Serifed`: the serifless union plus the bottom-right serif.
    #[must_use]
    pub fn serifed(&self, hook_style: i32, stroke: Option<f64>) -> Vec<Dispiro> {
        let mut operands = self.serifless(hook_style, stroke);
        operands.push(self.bottom_right_serif());
        operands
    }

    /// `SerifFrame.fromDf ... .rb.outer`, which is `HSerif.rb`.
    ///
    /// Only the bottom-right outer serif is ported: it is the one the serifed
    /// double-storey variants use, and it needs the frame's `jut` and nothing
    /// else from the serif frame.
    #[must_use]
    pub fn bottom_right_serif(&self) -> Dispiro {
        let metrics = self.metrics;
        let sw_ref = self.sw();
        let sw_serif = self.df.mvs;
        let left = self.left_sb();
        let right = self.right_sb();

        // `CSerifFrame`'s constructor, reduced to `jut` and `sideJut`.
        let ink = self.hsw_to_v(sw_ref);
        let co_ink = self.hsw_to_v(metrics.stroke - sw_ref);
        let h_split = 2.0_f64.max(self.df.h_pack);
        let jut = (metrics.jut - 0.5 * co_ink).min(crate::util::mix(
            0.5 * ink,
            metrics.jut,
            (self.df.adws() * 2.25 / h_split).min(1.0),
        ));
        let side_jut = jut - 0.5 * ink;
        let _ = (left, right, h_split);

        // `HSerif.rb right bot sideJut swSerif`.
        let x = right;
        let y = 0.0;
        let sw = sw_serif;
        let items = vec![
            Item::Af(self.kit.widths_lhs(Some(sw))),
            self.kit.knot(
                KnotType::Left,
                x - self.hsw_to_v(0.5 * sw),
                y,
                self.kit.heading(rightward()),
            ),
            self.kit.knot(
                KnotType::Right,
                x + side_jut + sw * metrics.serif_shift_x,
                y,
                Vec::new(),
            ),
        ];
        self.kit.dispiro(items)
    }
}
