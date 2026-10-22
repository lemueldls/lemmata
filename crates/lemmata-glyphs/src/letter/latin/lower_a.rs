//! The lower-case `a` from `letter/latin/lower-a.ptl`.
//!
//! The file builds three families. `DoubleStorey` is the `a` the default
//! variant selects, a union of `HookAndBar` and `Arc`. `SingleStorey` is the
//! ring family the `a.*` table draws, one of five bodies over one of five bars.
//! `CrossingBody` is `a.crossing`.
//!
//! Two of the single-storey rings reach the glyph through two nested flips.
//! `OBarRight.earlessCorner` is `OBarLeft.toothlessCorner` flipped, and
//! `OBarLeft.toothlessCorner` is `OBarLeft.earlessCorner` flipped, so the
//! source reads as unflipped and is not. The capture settles it: it holds the
//! unflipped knots inside a `Transformed` node, so the port flips once, at the
//! point upstream does.
//!
//! `aSwash`, `aRetroflexHook`, the script forms, `mathbb/a` and the accented
//! compositions are built from these bodies plus shapes outside this file.
//! They are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript. The names are
// upstream's, where `ada` and `adb` are the two arch depths.
#![allow(clippy::suboptimal_flops, clippy::similar_names)]

use kurbo::Vec2;
use lemmata_geom::{Geometry, KnotType};
pub use lemmata_ptl::shapes::OBarRing;
use lemmata_ptl::{
    control::{Item, UserControlKnot},
    letter_model::DivFrame,
    metrics::Metrics,
    shapes,
    shapes::{
        ArchArgs, ArchParams, CenterAtArgs, Design, FlatSide, HSerifSide, HookArgs, HookParams,
        OBarContext, arch, arch_depth_b_clamped, center_at, downward, flatside_lu, leftward,
        rightward, upward, ysmooth_mid_l, ysmooth_mid_r,
    },
    spiro_kit::{Direction, Dispiro, Family, Kit, Position},
    util::mix,
};

/// The hook styles `DoubleStorey` takes.
pub const HOOK_SLAB_NONE: i32 = 0;
pub const HOOK_SLAB_CLASSICAL: i32 = 1;
pub const HOOK_SLAB_INWARD: i32 = 2;
pub const HOOK_HALF: i32 = 3;

/// The bars a single-storey body can carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SingleStoreyBar {
    Serifless,
    BottomSerifed,
    DoubleSerifed,
    Tailed,
    TailedSerifed,
}

/// The single-storey bodies, which differ in how the ring meets the bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SingleStoreyBody {
    /// `FullBarBody`: the whole flat-sided ring.
    FullBar,
    /// `TopCutBody`: the ring cut by the bar's top half.
    TopCut,
    /// `FlatTopBody`: the ring with the ear replaced by a corner.
    FlatTop,
    /// `EarlessCornerBody`: the ring cut short of the ear by `DToothlessRise`.
    EarlessCorner,
    /// `EarlessRoundedBody`: the ring with the ear rounded into the bar.
    EarlessRounded,
}

/// The context every `a` body reads.
pub struct LowerA<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerA<'_> {
    // `DoubleStorey`'s stroke advice.

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

        lemmata_ptl::shapes::arch_depth_a_clamped(
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

    // The shared arithmetic.

    const fn left_sb(&self) -> f64 {
        self.df.sb
    }

    fn right_sb(&self) -> f64 {
        self.df.width - self.df.sb
    }

    fn middle(&self) -> f64 {
        mix(self.left_sb(), self.right_sb(), 0.5)
    }

    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    /// `Arch.adjust-x.top`.
    fn adjust_x_top(&self, x: f64, sw: f64) -> f64 {
        x - self.metrics.correction_o_mid_x * sw
    }

    /// `Arch.adjust-x.bot`.
    fn adjust_x_bot(&self, x: f64, sw: f64) -> f64 {
        x + self.metrics.correction_o_mid_x * sw
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

    /// The `OBar` ring the single-storey bodies sit in.
    fn ring(&self, top: f64, sw: f64, ada: f64, adb: f64) -> OBarRing {
        OBarRing {
            top,
            bot: 0.0,
            left: self.left_sb(),
            right: self.right_sb(),
            sw,
            fine: self.metrics.shoulder_fine * (sw / self.metrics.stroke),
            ada,
            adb,
        }
    }

    // `DoubleStorey`.

    /// `HookAndBar`: the hook and the bar it sits on, plus the inward serif the
    /// inward hook style adds.
    ///
    /// Only the styles the variant table selects are built. `HOOK-SLAB-CLASSICAL`
    /// is a `TODO` upstream and never reaches a glyph.
    #[must_use]
    pub fn hook_and_bar(
        &self,
        hook_style: i32,
        y0: f64,
        stroke: Option<f64>,
        fine: Option<f64>,
    ) -> Vec<Dispiro> {
        let metrics = self.metrics;
        let sw = stroke.unwrap_or_else(|| self.sw());
        let fine_value = fine.unwrap_or(sw);

        let bar_right_x = self.right_sb() - self.hsw_to_v(sw - fine_value);
        let hook_left_x = match hook_style {
            HOOK_HALF => {
                (bar_right_x
                    - self.hsw_to_v(fine_value + 0.125 * sw)
                    - metrics.tiny
                    - (metrics.tan_slope * sw).abs())
                .min(
                    self.adjust_x_top(
                        bar_right_x - self.hsw_to_v(0.5 * fine_value) - metrics.hook_x,
                        mix(sw, fine_value, 0.5),
                    ) + metrics.ox_hook,
                )
            }
            _ => self.left_sb(),
        };

        let bar_y = (metrics.xh - self.smooth_b())
            .max(y0 + metrics.tiny + self.hsw_to_v((metrics.tan_slope * fine_value).abs()));
        let flat_af = fine.map_or_else(Vec::new, |f| self.kit.widths_rhs(Some(f)));

        let mut items = vec![Item::Af(self.kit.widths_rhs(Some(sw)))];
        match hook_style {
            HOOK_SLAB_INWARD => {
                items.extend(self.inward_slab_arc_start_ltr_rhs(
                    hook_left_x,
                    metrics.xh,
                    sw,
                    fine_value,
                    bar_right_x,
                    bar_y,
                ));
            }
            HOOK_HALF => {
                items.push(self.kit.pair(
                    Family::G4,
                    Direction::Right,
                    Position::Start,
                    hook_left_x,
                    metrics.xh - metrics.o,
                    Vec::new(),
                ));
                items.push(self.kit.archv(None, None));
            }
            _ => {
                let terminal = UserControlKnot {
                    ty: KnotType::G4,
                    x: hook_left_x,
                    y: metrics.xh - metrics.a_hook,
                    af: Vec::new(),
                };
                let straight = UserControlKnot {
                    ty: KnotType::Left,
                    x: bar_right_x,
                    y: bar_y,
                    af: Vec::new(),
                };
                let lemmata_ptl::shapes::HookOut {
                    items: hook_items,
                    terminal,
                } = lemmata_ptl::shapes::hook_start(
                    self.kit,
                    &terminal,
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
                items.push(
                    self.kit
                        .knot(terminal.ty, terminal.x, terminal.y, terminal.af),
                );
                items.extend(hook_items);
            }
        }

        items.push(self.kit.knot(KnotType::Left, bar_right_x, bar_y, flat_af));
        items.push(self.kit.knot(
            KnotType::Right,
            bar_right_x,
            y0,
            self.kit.heading(downward(metrics.hv_contrast)),
        ));

        let mut shapes = vec![self.kit.dispiro(items)];
        if hook_style == HOOK_SLAB_INWARD {
            shapes.push(self.arc_start_serif_inward_l(
                self.left_sb(),
                metrics.xh,
                sw,
                metrics.a_hook,
            ));
        }

        shapes
    }

    /// `InwardSlabArcStart.LtrRhs`: the hook's inward slab and the arch it
    /// turns into.
    fn inward_slab_arc_start_ltr_rhs(
        &self,
        ex: f64,
        cy: f64,
        sw: f64,
        sw_after: f64,
        bar_right_x: f64,
        bar_y: f64,
    ) -> Vec<Item> {
        let metrics = self.metrics;
        let slab = UserControlKnot {
            ty: KnotType::G2,
            x: ex,
            y: cy - metrics.d_toothless_rise,
            af: self.kit.widths_rhs(Some(sw)),
        };
        let straight = UserControlKnot {
            ty: KnotType::Left,
            x: bar_right_x,
            y: bar_y,
            af: Vec::new(),
        };
        let mut items = vec![self.kit.knot(slab.ty, slab.x, slab.y, slab.af.clone())];
        items.extend(arch(
            self.kit,
            &ArchArgs {
                y: cy,
                p: None,
                sw,
                compact: false,
                o: metrics.o,
                angle_pre: None,
                angle_post: None,
                blend_pre: Some(false),
                blend_post: None,
                sw_before: Some(sw),
                sw_after: Some(sw_after),
                lhs: false,
            },
            &slab,
            &straight,
            &self.arch_params(),
        ));

        items
    }

    /// `ArcStartSerif.InwardL`: the slab under the hook's inward serif.
    fn arc_start_serif_inward_l(&self, cx: f64, cy: f64, stroke: f64, hook: f64) -> Dispiro {
        let metrics = self.metrics;
        let depth = hook.max(metrics.d_toothless_rise + stroke);

        self.vbar_l(
            cx,
            cy - depth,
            cy - metrics.d_toothless_rise,
            shapes::arc_start_serif_width(stroke, metrics.v_jut_stroke),
        )
    }

    /// `Arc`: the shoulder on the left.
    ///
    /// Kind 0 is what the serifless and serifed variants use; kind 1 is
    /// `FlatBottom`, kind 2 `ToothlessRounded`.
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

        let bar_top =
            mix(0.0, metrics.xh, self.design.a_bar_pos) + sw * (0.5 + self.design.a_bar_stroke_adj);
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
            mix(self.right_sb(), self.middle(), 0.9),
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
        let arch_knot = UserControlKnot {
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
                let straight = UserControlKnot {
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
                        blend_pre: None,
                        blend_post: None,
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
            1 => {
                if rise == 0.0 {
                    items.push(self.kit.arcvh(None, None));
                    items.push(self.kit.knot(
                        KnotType::Left,
                        self.adjust_x_bot(self.middle(), sw),
                        0.0,
                        Vec::new(),
                    ));
                    items.push(self.kit.knot(
                        KnotType::Right,
                        self.right_sb() + metrics.o,
                        0.0,
                        self.kit.heading(rightward()),
                    ));
                } else {
                    let end = UserControlKnot {
                        ty: KnotType::G4,
                        x: self.right_sb(),
                        y: rise,
                        af: self.kit.widths_lhs(Some(sw)),
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
                            blend_pre: None,
                            blend_post: Some(false),
                            sw_before: Some(sw),
                            sw_after: Some(sw),
                            lhs: true,
                        },
                        &arch_knot,
                        &end,
                        &self.arch_params(),
                    ));
                    items.push(self.kit.knot(end.ty, end.x, end.y, end.af));
                }
            }
            2 => {
                let end = UserControlKnot {
                    ty: KnotType::Left,
                    x: self.right_sb(),
                    y: rise.min(ysmooth_mid_r(
                        bar_top,
                        0.0,
                        Some(ada2),
                        Some(adb2),
                        metrics.arch_depth_a,
                        metrics.arch_depth_b,
                    )),
                    af: Vec::new(),
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
                        blend_pre: None,
                        blend_post: None,
                        sw_before: Some(sw),
                        sw_after: Some(sw),
                        lhs: true,
                    },
                    &arch_knot,
                    &end,
                    &self.arch_params(),
                ));
                items.push(self.kit.knot(end.ty, end.x, end.y, end.af));
                items.push(self.kit.knot(
                    KnotType::Right,
                    self.right_sb(),
                    rise + metrics.tiny + self.hsw_to_v((sw * metrics.tan_slope).abs()),
                    self.kit.heading(upward(metrics.hv_contrast)),
                ));
            }
            other => panic!("arc kind {other} is not ported"),
        }

        self.kit.dispiro(items)
    }

    /// `Serifless`: the hook and bar unioned with the arc.
    #[must_use]
    pub fn serifless(&self, hook_style: i32, stroke: Option<f64>) -> Vec<Dispiro> {
        let mut shapes = self.hook_and_bar(hook_style, 0.0, stroke, None);
        shapes.push(self.arc(0, 0.0, stroke, None, None));

        shapes
    }

    /// `Serifed`: `Serifless` plus the bottom-right serif.
    #[must_use]
    pub fn serifed(&self, hook_style: i32, stroke: Option<f64>) -> Vec<Dispiro> {
        let mut shapes = self.serifless(hook_style, stroke);
        let sw_ref = stroke.unwrap_or_else(|| self.sw());
        shapes.push(self.hserif_rb(
            self.right_sb(),
            0.0,
            self.serif_side_jut(sw_ref),
            self.df.mvs,
        ));

        shapes
    }

    /// `Tailed`: the hook and bar, the arc, and the tail off the bar's foot.
    #[must_use]
    pub fn tailed(&self, hook_style: i32, stroke: Option<f64>) -> Vec<Dispiro> {
        let mut shapes = self.hook_and_bar(hook_style, self.smooth_a(), stroke, None);
        shapes.push(self.arc(0, 0.0, stroke, None, None));
        shapes.push(self.rightward_tailed_bar(
            self.right_sb(),
            0.0,
            self.metrics.xh - self.smooth_b(),
            stroke.unwrap_or_else(|| self.sw()),
        ));

        shapes
    }

    /// `FlatBottomSerifless`.
    #[must_use]
    pub fn flat_bottom_serifless(&self, hook_style: i32, stroke: Option<f64>) -> Vec<Dispiro> {
        let mut shapes = self.hook_and_bar(hook_style, 0.0, stroke, None);
        shapes.push(self.arc(1, 0.0, stroke, None, None));

        shapes
    }

    /// `FlatBottomSerifed`.
    #[must_use]
    pub fn flat_bottom_serifed(&self, hook_style: i32, stroke: Option<f64>) -> Vec<Dispiro> {
        let mut shapes = self.flat_bottom_serifless(hook_style, stroke);
        let sw_ref = stroke.unwrap_or_else(|| self.sw());
        shapes.push(self.hserif_rb(
            self.right_sb(),
            0.0,
            self.serif_side_jut(sw_ref),
            self.df.mvs,
        ));

        shapes
    }

    /// `ToothlessCorner`.
    #[must_use]
    pub fn toothless_corner(&self, hook_style: i32, stroke: Option<f64>) -> Vec<Dispiro> {
        let rise = self.metrics.d_toothless_rise;
        let mut shapes = self.hook_and_bar(hook_style, rise, stroke, None);
        shapes.push(self.arc(1, rise, stroke, None, None));

        shapes
    }

    /// `ToothlessRounded`.
    #[must_use]
    pub fn toothless_rounded(&self, hook_style: i32, stroke: Option<f64>) -> Vec<Dispiro> {
        let rise = self.smooth_a();
        let mut shapes = self.hook_and_bar(hook_style, rise, stroke, None);
        shapes.push(self.arc(2, rise, stroke, None, None));

        shapes
    }

    // `SingleStorey`.

    /// `VBar.r`.
    #[must_use]
    pub fn vbar_r(&self, x: f64, y_down: f64, y_up: f64, sw: f64) -> Dispiro {
        shapes::vbar_r(self.kit, self.metrics, x, y_down, y_up, sw)
    }

    /// `VBar.l`: the bar's ink to the right of `x`.
    #[must_use]
    pub fn vbar_l(&self, x: f64, y_down: f64, y_up: f64, sw: f64) -> Dispiro {
        shapes::vbar_l(self.kit, self.metrics, x, y_down, y_up, sw)
    }

    /// `HSerif.rb`: the outer serif on the bar's right foot.
    fn hserif_rb(&self, x: f64, y: f64, length: f64, sw: f64) -> Dispiro {
        shapes::hserif(
            self.kit,
            self.metrics,
            HSerifSide::RightBottom,
            x,
            y,
            length,
            sw,
            None,
        )
    }

    /// `HSerif.rt`: the outer serif on the bar's right shoulder.
    fn hserif_rt(&self, x: f64, y: f64, length: f64, sw: f64) -> Dispiro {
        shapes::hserif(
            self.kit,
            self.metrics,
            HSerifSide::RightTop,
            x,
            y,
            length,
            sw,
            None,
        )
    }

    /// `CSerifFrame`'s `sideJut`, which is `jut` less half the ink.
    fn serif_side_jut(&self, sw_ref: f64) -> f64 {
        let metrics = self.metrics;
        let ink = self.hsw_to_v(sw_ref);
        let co_ink = self.hsw_to_v(metrics.stroke - sw_ref);
        let h_split = 2.0_f64.max(self.df.h_pack);
        let jut = (metrics.jut - 0.5 * co_ink).min(mix(
            0.5 * ink,
            metrics.jut,
            (self.df.adws() * 2.25 / h_split).min(1.0),
        ));

        jut - 0.5 * ink
    }

    /// `RightwardTailedBar`: the bar's foot turned right.
    #[must_use]
    pub fn rightward_tailed_bar(&self, x: f64, low: f64, high: f64, sw: f64) -> Dispiro {
        shapes::rightward_tailed_bar(self.kit, self.metrics, self.df, x, low, high, sw)
    }

    /// The ring, as [`shapes::OBarContext`] sees it.
    const fn obar_context(&self) -> OBarContext<'_> {
        OBarContext {
            kit: self.kit,
            metrics: self.metrics,
            design: self.design,
        }
    }

    /// `OBarLeft.shape`, in [`shapes`].
    #[must_use]
    pub fn obar_left_shape(&self, ring: &OBarRing) -> Dispiro {
        shapes::obar_left_shape(&self.obar_context(), ring)
    }

    /// `OBarLeft.toothlessCorner`, in [`shapes`].
    #[must_use]
    pub fn obar_left_toothless_corner(&self, ring: &OBarRing, rise: f64, ystart: f64) -> Dispiro {
        shapes::obar_left_toothless_corner(&self.obar_context(), ring, rise, ystart)
    }

    /// `OBarLeft.toothlessRounded`, in [`shapes`].
    #[must_use]
    pub fn obar_left_toothless_rounded(
        &self,
        ring: &OBarRing,
        y_terminal: f64,
        ystart: f64,
    ) -> Dispiro {
        shapes::obar_left_toothless_rounded(&self.obar_context(), ring, y_terminal, ystart)
    }

    /// `OBarRight.shape`, in [`shapes`].
    #[must_use]
    pub fn obar_right_shape(&self, ring: &OBarRing) -> Geometry {
        shapes::obar_right_shape(&self.obar_context(), ring)
    }

    /// `OBarRight.earlessCorner`, in [`shapes`].
    #[must_use]
    pub fn obar_right_earless_corner(&self, ring: &OBarRing, rise: f64, yend: f64) -> Geometry {
        shapes::obar_right_earless_corner(&self.obar_context(), ring, rise, yend)
    }

    /// `OBarRight.earlessRounded`, in [`shapes`].
    #[must_use]
    pub fn obar_right_earless_rounded(
        &self,
        ring: &OBarRing,
        y_terminal: f64,
        yend: f64,
    ) -> Geometry {
        shapes::obar_right_earless_rounded(&self.obar_context(), ring, y_terminal, yend)
    }

    /// The bars a single-storey body carries.
    fn bar(&self, bar: SingleStoreyBar, top: f64, sw: f64) -> Vec<Geometry> {
        let metrics = self.metrics;
        let side_jut = self.serif_side_jut(sw);
        match bar {
            SingleStoreyBar::Serifless => {
                vec![
                    self.vbar_r(self.right_sb(), 0.0, top, sw)
                        .geometry(kurbo::Affine::IDENTITY),
                ]
            }
            SingleStoreyBar::BottomSerifed => {
                vec![
                    self.vbar_r(self.right_sb(), 0.0, top, sw)
                        .geometry(kurbo::Affine::IDENTITY),
                    self.hserif_rb(self.right_sb(), 0.0, side_jut, metrics.stroke)
                        .geometry(kurbo::Affine::IDENTITY),
                ]
            }
            SingleStoreyBar::DoubleSerifed => {
                vec![
                    self.vbar_r(self.right_sb(), 0.0, top, sw)
                        .geometry(kurbo::Affine::IDENTITY),
                    self.hserif_rt(self.right_sb(), top, side_jut, metrics.stroke)
                        .geometry(kurbo::Affine::IDENTITY),
                    self.hserif_rb(self.right_sb(), 0.0, side_jut, metrics.stroke)
                        .geometry(kurbo::Affine::IDENTITY),
                ]
            }
            SingleStoreyBar::Tailed => {
                vec![
                    self.rightward_tailed_bar(self.right_sb(), 0.0, top, sw)
                        .geometry(kurbo::Affine::IDENTITY),
                ]
            }
            SingleStoreyBar::TailedSerifed => {
                vec![
                    self.rightward_tailed_bar(self.right_sb(), 0.0, top, sw)
                        .geometry(kurbo::Affine::IDENTITY),
                    self.hserif_rt(self.right_sb(), top, side_jut, metrics.stroke)
                        .geometry(kurbo::Affine::IDENTITY),
                ]
            }
        }
    }

    /// `SingleStorey.FullBarBody`.
    #[must_use]
    pub fn full_bar_body(
        &self,
        top: f64,
        bar: SingleStoreyBar,
        stroke: Option<f64>,
        ada: Option<f64>,
        adb: Option<f64>,
    ) -> Vec<Geometry> {
        let (sw, ada, adb) = self.body_stroke(stroke, ada, adb);
        let ring = self.ring(top, sw, ada, adb);
        let mut shapes = vec![self.obar_right_shape(&ring)];
        shapes.extend(self.bar(bar, top, sw));

        shapes
    }

    /// `SingleStorey.TopCutBody`: the ring cut by the bar's top half.
    #[must_use]
    pub fn top_cut_body(
        &self,
        top: f64,
        bar: SingleStoreyBar,
        stroke: Option<f64>,
        ada: Option<f64>,
        adb: Option<f64>,
    ) -> Vec<Geometry> {
        let (sw, ada, adb) = self.body_stroke(stroke, ada, adb);
        let ring = self.ring(top, sw, ada, adb);
        let half_stroke = 0.5 * sw;
        let cut = self.kit.spiro_outline(vec![
            self.kit
                .knot(KnotType::Corner, self.right_sb(), top, Vec::new()),
            self.kit.knot(
                KnotType::Corner,
                self.right_sb(),
                top - half_stroke,
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Corner,
                self.right_sb() - self.hsw_to_v(sw),
                top - half_stroke,
                Vec::new(),
            ),
        ]);

        let mut shapes = vec![self.obar_right_shape(&ring)];
        shapes.extend(self.bar(bar, top - half_stroke, sw));
        shapes.push(Geometry::Spiro {
            gizmo: kurbo::Affine::IDENTITY,
            closed: cut.closed,
            knots: cut.knots,
        });

        shapes
    }

    /// `SingleStorey.FlatTopBody`.
    #[must_use]
    pub fn flat_top_body(
        &self,
        top: f64,
        bar: SingleStoreyBar,
        stroke: Option<f64>,
        ada: Option<f64>,
        adb: Option<f64>,
    ) -> Vec<Geometry> {
        let (sw, ada, adb) = self.body_stroke(stroke, ada, adb);
        let ring = self.ring(top, sw, ada, adb);
        let yend = ring.bot + ring.ada + self.metrics.tiny;
        let mut shapes = vec![self.obar_right_earless_corner(&ring, 0.0, yend)];
        shapes.extend(self.bar(bar, top, sw));

        shapes
    }

    /// `SingleStorey.EarlessCornerBody`.
    #[must_use]
    pub fn earless_corner_body(
        &self,
        top: f64,
        bar: SingleStoreyBar,
        stroke: Option<f64>,
        ada: Option<f64>,
        adb: Option<f64>,
    ) -> Vec<Geometry> {
        let (sw, ada, adb) = self.body_stroke(stroke, ada, adb);
        let ring = self.ring(top, sw, ada, adb);
        let yend = ring.bot + ring.ada + self.metrics.tiny;
        let mut shapes =
            vec![self.obar_right_earless_corner(&ring, self.metrics.d_toothless_rise, yend)];
        shapes.extend(self.bar(bar, top - self.metrics.d_toothless_rise, sw));

        shapes
    }

    /// `SingleStorey.EarlessRoundedBody`.
    #[must_use]
    pub fn earless_rounded_body(
        &self,
        top: f64,
        bar: SingleStoreyBar,
        stroke: Option<f64>,
        ada: Option<f64>,
        adb: Option<f64>,
    ) -> Vec<Geometry> {
        let (sw, ada, adb) = self.body_stroke(stroke, ada, adb);
        let ring = self.ring(top, sw, ada, adb);
        let yend = ring.bot + ring.ada + self.metrics.tiny;
        let mut shapes = vec![self.obar_right_earless_rounded(&ring, ring.ada, yend)];
        shapes.extend(self.bar(bar, top - ring.adb, sw));

        shapes
    }

    /// The fallbacks a body's stroke and arch depths take.
    fn body_stroke(
        &self,
        stroke: Option<f64>,
        ada: Option<f64>,
        adb: Option<f64>,
    ) -> (f64, f64, f64) {
        let sw = stroke.unwrap_or(self.df.mvs);
        let ada = ada.unwrap_or_else(|| {
            self.df
                .arch_depth_a_of(self.metrics.small_arch_depth, Some(sw))
        });
        let adb = adb.unwrap_or_else(|| {
            self.df
                .arch_depth_b_of(self.metrics.small_arch_depth, Some(sw))
        });

        (sw, ada, adb)
    }

    // `CrossingBody`.

    /// `CrossingBody`, which is `a.crossing`.
    #[must_use]
    pub fn crossing_body(&self, top: f64) -> Geometry {
        let metrics = self.metrics;
        let sw = self.df.mvs;
        let ada = self.df.arch_depth_a_of(metrics.small_arch_depth, Some(sw));
        let adb = self.df.arch_depth_b_of(metrics.small_arch_depth, Some(sw));
        let middle = mix(self.left_sb(), self.right_sb(), 0.42);
        let fine = self.df.advice_stroke(3.25, 1.0);
        let (k1, k2, k3, k4) = (0.25, 0.50, 0.35, 1.00);
        let params = self.arch_params();

        let mut items = vec![
            self.kit.knot(
                KnotType::G4,
                self.right_sb() - metrics.ox * 1.75,
                top,
                self.kit
                    .widths_rhs_heading(Some(fine), downward(metrics.hv_contrast)),
            ),
            self.kit.bez_controls(k1, k2, k3, k4, 6, Vec::new()),
            center_at(
                self.kit,
                &CenterAtArgs {
                    x: middle,
                    y: 0.0,
                    sw,
                    compact: false,
                    o: metrics.o,
                    sw_before: fine,
                    sw_after: sw,
                    lhs: false,
                    rtl: true,
                    at_bot: true,
                },
                &params,
            ),
            self.kit.archv(None, None),
        ];
        items.extend(flatside_lu(self.kit, &FlatSide {
            x: self.left_sb(),
            d: 0.0,
            u: top,
            ada,
            adb,
            ox: metrics.ox * 2.0,
            af: self.kit.widths_rhs(Some(sw)),
        }));
        items.push(self.kit.arcvh(None, None));
        items.push(center_at(
            self.kit,
            &CenterAtArgs {
                x: middle,
                y: top,
                sw,
                compact: false,
                o: metrics.o,
                sw_before: sw,
                sw_after: fine,
                lhs: false,
                rtl: false,
                at_bot: false,
            },
            &params,
        ));
        items.push(
            self.kit
                .bez_controls(1.0 - k3, 1.0 - k4, 1.0 - k1, 1.0 - k2, 6, Vec::new()),
        );
        items.push(
            self.kit.knot(
                KnotType::G4,
                self.right_sb() - metrics.ox * 2.0,
                0.0,
                self.kit
                    .widths_rhs_heading(Some(fine), downward(metrics.hv_contrast)),
            ),
        );

        self.kit.dispiro(items).geometry(kurbo::Affine::IDENTITY)
    }
}

/// `FlipAround` about the ring's centre, as a node the tree carries.
/// The double-storey bodies `DoubleStoreyConfig` selects.
#[derive(Clone, Copy)]
enum DoubleBody {
    Serifless,
    Serifed,
    Tailed,
    FlatBottomSerifless,
    FlatBottomSerifed,
    ToothlessCorner,
    ToothlessRounded,
}

impl DoubleBody {
    fn build(self, body: &LowerA, hook: i32) -> Vec<Dispiro> {
        match self {
            Self::Serifless => body.serifless(hook, None),
            Self::Serifed => body.serifed(hook, None),
            Self::Tailed => body.tailed(hook, None),
            Self::FlatBottomSerifless => body.flat_bottom_serifless(hook, None),
            Self::FlatBottomSerifed => body.flat_bottom_serifed(hook, None),
            Self::ToothlessCorner => body.toothless_corner(hook, None),
            Self::ToothlessRounded => body.toothless_rounded(hook, None),
        }
    }
}

/// The double-storey suffixes, in `DoubleStoreyConfig`'s order.
const DOUBLE: &[(&str, DoubleBody)] = &[
    ("doubleStoreySerifless", DoubleBody::Serifless),
    ("doubleStoreySerifed", DoubleBody::Serifed),
    ("doubleStoreyTailed", DoubleBody::Tailed),
    (
        "doubleStoreyFlatBottomSerifless",
        DoubleBody::FlatBottomSerifless,
    ),
    (
        "doubleStoreyFlatBottomSerifed",
        DoubleBody::FlatBottomSerifed,
    ),
    ("doubleStoreyToothlessCorner", DoubleBody::ToothlessCorner),
    ("doubleStoreyToothlessRounded", DoubleBody::ToothlessRounded),
];

/// The single-storey suffixes and the body and bar each selects.
const SINGLE: &[(&str, SingleStoreyBody, SingleStoreyBar)] = &[
    (
        "singleStoreySerifless",
        SingleStoreyBody::FullBar,
        SingleStoreyBar::Serifless,
    ),
    (
        "singleStoreyTopCutSerifless",
        SingleStoreyBody::TopCut,
        SingleStoreyBar::Serifless,
    ),
    (
        "singleStoreyFlatTopSerifless",
        SingleStoreyBody::FlatTop,
        SingleStoreyBar::Serifless,
    ),
    (
        "singleStoreyEarlessCornerSerifless",
        SingleStoreyBody::EarlessCorner,
        SingleStoreyBar::Serifless,
    ),
    (
        "singleStoreyEarlessRoundedSerifless",
        SingleStoreyBody::EarlessRounded,
        SingleStoreyBar::Serifless,
    ),
    (
        "singleStoreySerifed",
        SingleStoreyBody::FullBar,
        SingleStoreyBar::BottomSerifed,
    ),
    (
        "singleStoreyTopCutSerifed",
        SingleStoreyBody::TopCut,
        SingleStoreyBar::BottomSerifed,
    ),
    (
        "singleStoreyFlatTopSerifed",
        SingleStoreyBody::FlatTop,
        SingleStoreyBar::BottomSerifed,
    ),
    (
        "singleStoreyEarlessCornerSerifed",
        SingleStoreyBody::EarlessCorner,
        SingleStoreyBar::BottomSerifed,
    ),
    (
        "singleStoreyEarlessRoundedSerifed",
        SingleStoreyBody::EarlessRounded,
        SingleStoreyBar::BottomSerifed,
    ),
    (
        "singleStoreyDoubleSerifed",
        SingleStoreyBody::FullBar,
        SingleStoreyBar::DoubleSerifed,
    ),
    (
        "singleStoreyFlatTopDoubleSerifed",
        SingleStoreyBody::FlatTop,
        SingleStoreyBar::DoubleSerifed,
    ),
    (
        "singleStoreyTailed",
        SingleStoreyBody::FullBar,
        SingleStoreyBar::Tailed,
    ),
    (
        "singleStoreyTopCutTailed",
        SingleStoreyBody::TopCut,
        SingleStoreyBar::Tailed,
    ),
    (
        "singleStoreyFlatTopTailed",
        SingleStoreyBody::FlatTop,
        SingleStoreyBar::Tailed,
    ),
    (
        "singleStoreyEarlessCornerTailed",
        SingleStoreyBody::EarlessCorner,
        SingleStoreyBar::Tailed,
    ),
    (
        "singleStoreyEarlessRoundedTailed",
        SingleStoreyBody::EarlessRounded,
        SingleStoreyBar::Tailed,
    ),
    (
        "singleStoreyTailedSerifed",
        SingleStoreyBody::FullBar,
        SingleStoreyBar::TailedSerifed,
    ),
    (
        "singleStoreyFlatTopTailedSerifed",
        SingleStoreyBody::FlatTop,
        SingleStoreyBar::TailedSerifed,
    ),
];

fn single_storey(
    body: &LowerA,
    kind: SingleStoreyBody,
    top: f64,
    bar: SingleStoreyBar,
    ada: Option<f64>,
    adb: Option<f64>,
) -> Vec<Geometry> {
    match kind {
        SingleStoreyBody::FullBar => body.full_bar_body(top, bar, None, ada, adb),
        SingleStoreyBody::TopCut => body.top_cut_body(top, bar, None, ada, adb),
        SingleStoreyBody::FlatTop => body.flat_top_body(top, bar, None, ada, adb),
        SingleStoreyBody::EarlessCorner => body.earless_corner_body(top, bar, None, ada, adb),
        SingleStoreyBody::EarlessRounded => body.earless_rounded_body(top, bar, None, ada, adb),
    }
}

impl LowerA<'_> {
    /// Every glyph this definition builds, by name.
    ///
    /// The order is the variant tables' and the names are the ones the build
    /// gives them, so a caller can file them in a store and a reader can compare
    /// them with a capture.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let metrics = self.metrics;
        let combine = |operands: Vec<Geometry>| Geometry::Combine(operands);
        let from_dispiros = |dispiros: Vec<Dispiro>| {
            combine(
                dispiros
                    .iter()
                    .map(|dispiro| dispiro.geometry(kurbo::Affine::IDENTITY))
                    .collect(),
            )
        };
        let mut out = Vec::new();
        for (suffix, kind) in DOUBLE {
            out.push((
                format!("a.{suffix}"),
                from_dispiros(kind.build(self, HOOK_SLAB_NONE)),
            ));
            out.push((
                format!("aHalf.{suffix}"),
                from_dispiros(kind.build(self, HOOK_HALF)),
            ));
            out.push((
                format!(
                    "a.doubleStoreyHookInwardSerifed{}",
                    &suffix["doubleStorey".len()..]
                ),
                from_dispiros(kind.build(self, HOOK_SLAB_INWARD)),
            ));
        }
        out.push((
            "a.crossing".to_string(),
            combine(vec![self.crossing_body(metrics.xh)]),
        ));
        for (suffix, kind, bar) in SINGLE {
            out.push((
                format!("a.{suffix}"),
                combine(single_storey(self, *kind, metrics.xh, *bar, None, None)),
            ));
            out.push((
                format!("aCap.{suffix}"),
                combine(single_storey(
                    self,
                    *kind,
                    metrics.cap,
                    *bar,
                    Some(metrics.arch_depth_a),
                    Some(metrics.arch_depth_b),
                )),
            ));
        }

        out
    }
}
