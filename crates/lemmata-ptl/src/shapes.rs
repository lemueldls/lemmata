//! The shape helpers glyph definitions call: the hook and the arch.
//!
//! Ported from `common/shapes.ptl` (`HookShape`, `determineMixR`, the `Arch`
//! namespace) and the clamped arch depths in `meta/aesthetics.ptl`. The `OBar`,
//! `VBar`, `HSerif`, `VerticalHook` and overlay helpers come from
//! `letter/shared.ptl`, where the letter blocks keep them.
//!
//! Both `HookStart` and `Arch.lhs`/`Arch.rhs` are `FunctionInterpolator`s
//! upstream: they read the knots either side of them when the flattener runs.
//! A definition places them between two knots it just wrote, so the port passes
//! those neighbours in directly and returns the resolved knots, with any arc
//! blends left as interpolators for the flattener.
//!
//! `HookShape` moves the terminal knot it is attached to and gives it a
//! heading. The port returns that knot rather than mutating a shared one, and
//! the caller substitutes it.

// `ada` and `adb` are the two arch depths, as upstream names them.
#![allow(
    clippy::similar_names,
    clippy::suboptimal_flops,
    clippy::cast_precision_loss
)]
use lemmata_geom::math;

use kurbo::Vec2;
use lemmata_geom::{Geometry, KnotType};

use crate::control::{Af, Amendment, Item, UserControlKnot};
use crate::metrics::Metrics;
use crate::spiro_kit::{Direction, Dispiro, Family, Kit, Position};
use crate::util::{boole, boole_pn, mix};

/// The design constants these helpers read, from `DesignParameters` in
/// `meta/aesthetics.ptl`. They are fixed for the family rather than derived
/// from the plan.
#[derive(Clone, Copy, Debug)]
pub struct Design {
    pub superness: f64,
    pub a_bar_pos: f64,
    pub a_bar_stroke_adj: f64,
    pub upper_e_bar_pos: f64,
}

impl Default for Design {
    fn default() -> Self {
        Self {
            superness: 2.275,
            a_bar_pos: 0.5304,
            a_bar_stroke_adj: -0.075,
            upper_e_bar_pos: 0.52,
        }
    }
}

// The geometry package's four heading constants, from `meta/aesthetics.ptl`.
// They point along the stroke, so `Upward` and `Downward` are horizontal.

/// `Upward`.
#[must_use]
pub const fn upward(hv_contrast: f64) -> Vec2 {
    Vec2::new(-hv_contrast, 0.0)
}

/// `Downward`.
#[must_use]
pub const fn downward(hv_contrast: f64) -> Vec2 {
    Vec2::new(hv_contrast, 0.0)
}

/// `Rightward`.
#[must_use]
pub const fn rightward() -> Vec2 {
    Vec2::new(0.0, 1.0)
}

/// `Leftward`.
#[must_use]
pub const fn leftward() -> Vec2 {
    Vec2::new(0.0, -1.0)
}

/// `archv.yFromX`.
#[must_use]
pub fn archv_y_from_x(px: f64, superness: f64) -> f64 {
    1.0 - math::pow(1.0 - math::pow(px, superness), 1.0 / superness)
}

/// `determineMixR`: where along the chord the key knot sits.
#[must_use]
pub fn determine_mix_r(w: f64, v: f64, u: f64, sw: f64, swash: bool, superness: f64) -> f64 {
    if !swash && w < v {
        return 1.0 - determine_mix_r(v, w, u, sw, swash, superness);
    }
    let mut r = if w <= v {
        0.5
    } else {
        1.0 / (math::pow(1.0 - math::pow(1.0 - v / w, superness), 1.0 / superness) + 1.0)
    };
    if swash {
        let idepth = w - sw;
        let iwidth = u * r - sw;

        if iwidth > 0.0 && idepth > 0.0 && iwidth / idepth >= 2.0 {
            let adjust = (1.0 - (iwidth / idepth - 2.0) * 0.0125).clamp(0.975, 1.0);
            r *= adjust;
        }
    } else {
        r = 0.5 + (r - 0.5) * (v + w) / (u * 2.0);
    }
    if r < 0.5 {
        r = 0.5;
    }

    r
}

/// `superSin`: the sine of an angle, raised to the superellipse power.
fn super_sin(angle: f64, superness: f64) -> f64 {
    if angle == 0.0 {
        return 0.0;
    }
    let s = math::sin(angle * std::f64::consts::PI / 180.0);

    crate::util::js_sign(s) * math::pow(s.abs(), 2.0 / superness)
}

/// `superCos`.
fn super_cos(angle: f64, superness: f64) -> f64 {
    if angle == 0.0 {
        return 1.0;
    }
    let c = math::cos(angle * std::f64::consts::PI / 180.0);

    crate::util::js_sign(c) * math::pow(c.abs(), 2.0 / superness)
}

/// `mixR`: the ratio where the arch's key knot sits.
fn mix_r(w: f64, ang_w: Option<f64>, v: f64, ang_v: Option<f64>, u: f64, superness: f64) -> f64 {
    if w < v {
        return 1.0 - mix_r(v, ang_v, w, ang_w, u, superness);
    }
    if let (Some(av), Some(aw)) = (ang_v, ang_w) {
        let rad_v = v / (1.0 - super_sin(av, superness));
        let rad_w = w / (1.0 - super_sin(aw, superness));
        let chr_v = rad_v * super_cos(av, superness);
        let chr_w = rad_w * super_cos(aw, superness);
        return chr_w / (chr_v + chr_w);
    }
    let r = 1.0 / (math::pow(1.0 - math::pow(1.0 - v / w, superness), 1.0 / superness) + 1.0);

    0.5 + (r - 0.5) * (v + w) / (u * 2.0)
}

/// What `flatside` takes: the stroke's x, the ring's bottom and top, the two
/// arch depths, the overshoot and any amendment for the control knots.
#[derive(Clone, Debug)]
pub struct FlatSide {
    pub x: f64,
    pub d: f64,
    pub u: f64,
    pub ada: f64,
    pub adb: f64,
    pub ox: f64,
    pub af: Af,
}

/// `flatside.ld`: the left side of a flat-sided ring.
#[must_use]
pub fn flatside_ld(kit: &Kit, side: &FlatSide) -> Vec<Item> {
    let FlatSide {
        x,
        d,
        u,
        ada,
        adb,
        ox,
        af,
    } = side;
    if u - d > ada + adb {
        vec![
            kit.knot(KnotType::Left, x + ox, u - ada, af.clone()),
            kit.knot(KnotType::Right, x + ox, d + adb, af.clone()),
        ]
    } else {
        vec![kit.knot(
            KnotType::G4,
            x + ox,
            ysmooth_mid_l(*u, *d, Some(*ada), Some(*adb), *ada, *adb),
            af.clone(),
        )]
    }
}

/// `flatside.ru`: the right side of a flat-sided ring.
#[must_use]
pub fn flatside_ru(kit: &Kit, side: &FlatSide) -> Vec<Item> {
    let FlatSide {
        x,
        d,
        u,
        ada,
        adb,
        ox,
        af,
    } = side;
    if u - d > ada + adb {
        vec![
            kit.knot(KnotType::Left, x - ox, d + ada, af.clone()),
            kit.knot(KnotType::Right, x - ox, u - adb, af.clone()),
        ]
    } else {
        vec![kit.knot(
            KnotType::G4,
            x - ox,
            ysmooth_mid_r(*u, *d, Some(*ada), Some(*adb), *ada, *adb),
            af.clone(),
        )]
    }
}

/// `flatside.lu`: the left side of a flat-sided ring, drawn upwards.
#[must_use]
pub fn flatside_lu(kit: &Kit, side: &FlatSide) -> Vec<Item> {
    let FlatSide {
        x,
        d,
        u,
        ada,
        adb,
        ox,
        af,
    } = side;
    if u - d > ada + adb {
        vec![
            kit.knot(KnotType::Left, x + ox, d + adb, af.clone()),
            kit.knot(KnotType::Right, x + ox, u - ada, af.clone()),
        ]
    } else {
        vec![kit.knot(
            KnotType::G4,
            x + ox,
            ysmooth_mid_l(*u, *d, Some(*ada), Some(*adb), *ada, *adb),
            af.clone(),
        )]
    }
}

/// The knot an item carries, which an interpolator next to it resolves against.
#[must_use]
pub fn knot_of(item: &Item) -> UserControlKnot {
    match item {
        Item::Control(crate::control::Control::Knot(knot)) => knot.clone(),
        Item::Control(crate::control::Control::Pair(pair)) => pair.center.clone(),
        other => panic!("expected a knot, found {other:?}"),
    }
}

/// `YSmoothMidR`.
#[must_use]
pub fn ysmooth_mid_r(
    top: f64,
    bot: f64,
    ada: Option<f64>,
    adb: Option<f64>,
    arch_depth_a: f64,
    arch_depth_b: f64,
) -> f64 {
    if bot > top {
        return ysmooth_mid_r(bot, top, ada, adb, arch_depth_a, arch_depth_b);
    }
    let ada = ada.unwrap_or(arch_depth_a);
    let adb = adb.unwrap_or(arch_depth_b);
    let delta = (adb - ada) / 2.0;
    let sm_of_sma = ada + delta;
    let sm_of_smb = adb - delta;
    let y = mix(bot, top, sm_of_sma / (sm_of_sma + sm_of_smb));

    y - delta * ((top - bot) / (ada + adb)).min(1.0)
}

/// `YSmoothMidL`.
#[must_use]
pub fn ysmooth_mid_l(
    top: f64,
    bot: f64,
    ada: Option<f64>,
    adb: Option<f64>,
    arch_depth_a: f64,
    arch_depth_b: f64,
) -> f64 {
    if bot > top {
        return ysmooth_mid_l(bot, top, ada, adb, arch_depth_a, arch_depth_b);
    }
    let ada = ada.unwrap_or(arch_depth_a);
    let adb = adb.unwrap_or(arch_depth_b);
    let delta = (adb - ada) / 2.0;
    let sm_of_sma = ada + delta;
    let sm_of_smb = adb - delta;
    let y = mix(bot, top, sm_of_smb / (sm_of_sma + sm_of_smb));

    y + delta * ((top - bot) / (ada + adb)).min(1.0)
}

/// `ArchDepthAClamped`.
#[must_use]
pub fn arch_depth_a_clamped(
    top: f64,
    bot: f64,
    ada: Option<f64>,
    adb: Option<f64>,
    arch_depth_a: f64,
    arch_depth_b: f64,
) -> f64 {
    let height = (top - bot).abs();
    let ada = ada.unwrap_or(arch_depth_a);
    let adb = adb.unwrap_or(arch_depth_b);

    if ada + adb < height {
        ada
    } else {
        ysmooth_mid_r(
            height,
            0.0,
            Some(ada),
            Some(adb),
            arch_depth_a,
            arch_depth_b,
        )
    }
}

/// `ArchDepthBClamped`.
#[must_use]
pub fn arch_depth_b_clamped(
    top: f64,
    bot: f64,
    ada: Option<f64>,
    adb: Option<f64>,
    arch_depth_a: f64,
    arch_depth_b: f64,
) -> f64 {
    let height = (top - bot).abs();
    let ada = ada.unwrap_or(arch_depth_a);
    let adb = adb.unwrap_or(arch_depth_b);

    if ada + adb < height {
        adb
    } else {
        ysmooth_mid_l(
            height,
            0.0,
            Some(ada),
            Some(adb),
            arch_depth_a,
            arch_depth_b,
        )
    }
}

/// The metrics the arch and hook helpers read.
#[derive(Clone, Copy, Debug)]
pub struct ArchParams {
    pub superness: f64,
    pub tan_slope: f64,
    pub hv_contrast: f64,
    pub o: f64,
    pub correction_o_mid_x: f64,
}

/// One side of an arch: `Arch.lhs` or `Arch.rhs`.
#[derive(Clone, Copy, Debug)]
pub struct ArchArgs {
    pub y: f64,
    pub p: Option<f64>,
    pub sw: f64,
    pub compact: bool,
    pub o: f64,
    pub angle_pre: Option<f64>,
    pub angle_post: Option<f64>,
    /// `blendPre`. `None` follows `angle_pre`: an arc when no angle is given.
    pub blend_pre: Option<bool>,
    /// `blendPost`. `None` follows `angle_post`.
    pub blend_post: Option<bool>,
    pub sw_before: Option<f64>,
    pub sw_after: Option<f64>,
    pub lhs: bool,
}

/// `Arch.lhs` or `Arch.rhs`, resolved against the knots either side.
///
/// Returns the arc blend, the key knot, and the other arc blend, in the order
/// upstream's `archBlender` lists them.
#[must_use]
pub fn arch(
    kit: &Kit,
    args: &ArchArgs,
    before: &UserControlKnot,
    after: &UserControlKnot,
    params: &ArchParams,
) -> Vec<Item> {
    let do_adj = !args.compact;
    let sw_before = args.sw_before.unwrap_or(args.sw);
    let sw_after = args.sw_after.unwrap_or(args.sw);

    let u = (after.x - before.x).abs();
    let v = (after.y - args.y).abs();
    let w = (before.y - args.y).abs();
    let mix_ratio = mix_r(w, args.angle_pre, v, args.angle_post, u, params.superness);
    let x = mix(before.x, after.x, args.p.unwrap_or(mix_ratio));
    let rtl = before.x > after.x;
    let at_bot = args.y < before.y && args.y < after.y;

    let skew = (params.hv_contrast * 0.5 * (sw_after - sw_before)) / args.sw;
    let heading = Vec2::new(
        boole_pn(rtl) * (boole_pn(args.lhs) * skew + boole(do_adj) * params.tan_slope),
        boole_pn(args.lhs) * boole_pn(at_bot),
    );
    let italic_adj = boole(do_adj) * boole_pn(at_bot) * params.correction_o_mid_x * args.sw;
    let overshoot = boole_pn(at_bot) * args.o;
    let af = if args.lhs {
        kit.widths_lhs_heading(Some(args.sw), heading)
    } else {
        kit.widths_rhs_heading(Some(args.sw), heading)
    };
    let knot = kit.pair(
        if args.compact { Family::G4 } else { Family::G2 },
        if rtl {
            Direction::Left
        } else {
            Direction::Right
        },
        Position::Mid,
        x + italic_adj,
        args.y + overshoot,
        af,
    );

    let mut items = Vec::new();
    // `blendPre` defaults to an arc when no pre angle is given, and `blendPost`
    // to one when no post angle is given. A caller can suppress either.
    if args.blend_pre.unwrap_or_else(|| args.angle_pre.is_none()) {
        items.push(kit.arcvh(None, None));
    }
    items.push(knot);
    if args.blend_post.unwrap_or_else(|| args.angle_post.is_none()) {
        items.push(kit.archv(None, None));
    }

    items
}

/// What `Arch.lhs.centerAt` / `Arch.rhs.centerAt` takes.
///
/// The booleans are the four switches upstream's `args` object carries.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug)]
pub struct CenterAtArgs {
    pub x: f64,
    pub y: f64,
    pub sw: f64,
    pub compact: bool,
    pub o: f64,
    pub sw_before: f64,
    pub sw_after: f64,
    pub lhs: bool,
    pub rtl: bool,
    pub at_bot: bool,
}

/// `Arch.lhs.centerAt` / `Arch.rhs.centerAt`.
///
/// Where [`arch`] resolves its knot against the neighbours it sits between,
/// `centerAt` takes the position and the direction as given and emits one
/// directed pair. No blend sits either side of it.
#[must_use]
pub fn center_at(kit: &Kit, args: &CenterAtArgs, params: &ArchParams) -> Item {
    let do_adj = !args.compact;
    let skew = (params.hv_contrast * 0.5 * (args.sw_after - args.sw_before)) / args.sw;
    let heading = Vec2::new(
        boole_pn(args.rtl) * (boole_pn(args.lhs) * skew + boole(do_adj) * params.tan_slope),
        boole_pn(args.lhs) * boole_pn(args.at_bot),
    );
    let italic_adj = boole(do_adj) * boole_pn(args.at_bot) * params.correction_o_mid_x * args.sw;
    let overshoot = boole_pn(args.at_bot) * args.o;
    let af = if args.lhs {
        kit.widths_lhs_heading(Some(args.sw), heading)
    } else {
        kit.widths_rhs_heading(Some(args.sw), heading)
    };

    kit.pair(
        Family::G2,
        if args.rtl {
            Direction::Left
        } else {
            Direction::Right
        },
        Position::Mid,
        args.x + italic_adj,
        args.y + overshoot,
        af,
    )
}

/// The metrics the hook helpers read.
#[derive(Clone, Copy, Debug)]
pub struct HookParams {
    pub superness: f64,
    pub tan_slope: f64,
    pub contrast: f64,
    pub correction_o_mid_x: f64,
    pub ox_hook: f64,
    pub hook: f64,
    pub slope_angle: f64,
    pub is_italic: bool,
}

/// `HookStart`'s arguments.
#[derive(Clone, Copy, Debug)]
pub struct HookArgs {
    pub y_ref: f64,
    pub sw: f64,
    pub sw_terminal: f64,
    pub is_tail: bool,
    pub suppress_swash: bool,
    pub overshoot: f64,
}

/// The knots a hook resolves to, and the terminal knot it moved.
#[derive(Clone, Debug)]
pub struct HookOut {
    pub items: Vec<Item>,
    pub terminal: UserControlKnot,
}

/// `HookStart`, resolved against the knots either side.
#[must_use]
pub fn hook_start(
    kit: &Kit,
    before: &UserControlKnot,
    after: &UserControlKnot,
    args: &HookArgs,
    params: &HookParams,
) -> HookOut {
    hook_shape(kit, after, before, true, args, params)
}

/// `HookEnd`, resolved against the knots either side.
#[must_use]
pub fn hook_end(
    kit: &Kit,
    before: &UserControlKnot,
    after: &UserControlKnot,
    args: &HookArgs,
    params: &HookParams,
) -> HookOut {
    hook_shape(kit, before, after, false, args, params)
}

/// `HookShape`: the arc of knots from the hook's tip to the terminal.
fn hook_shape(
    kit: &Kit,
    to_straight: &UserControlKnot,
    to_finish: &UserControlKnot,
    is_start: bool,
    args: &HookArgs,
    params: &HookParams,
) -> HookOut {
    const SEGMENTS: usize = 12;

    let at_bottom = to_straight.y > args.y_ref;
    let ltr = if is_start {
        to_finish.x < to_straight.x
    } else {
        to_finish.x > to_straight.x
    };
    let dtu = if is_start {
        args.y_ref > to_finish.y
    } else {
        args.y_ref < to_finish.y
    };
    let do_swash = !args.suppress_swash
        && !is_start
        && at_bottom
        && (params.is_italic || args.is_tail)
        && params.slope_angle != 0.0
        && (if params.slope_angle > 0.0 { ltr } else { !ltr });

    let y = args.y_ref + (if args.y_ref > to_finish.y { -1.0 } else { 1.0 }) * args.overshoot;

    let mut finish = to_finish.clone();
    finish.x +=
        params.ox_hook * (if ltr { -1.0 } else { 1.0 }) * (if is_start { -1.0 } else { 1.0 });
    if do_swash {
        let tail_adj_x = (params.hook * 0.5).max(1.0 * args.sw);
        let tail_adj_y = (params.hook * 0.5).max(1.25 * args.sw);
        finish.x += tail_adj_x * params.tan_slope;
        finish.y -= tail_adj_y * params.tan_slope.abs();
    }

    let w = (to_straight.y - y).abs();
    let v = (finish.y - y).abs();
    let u = (finish.x - to_straight.x).abs();
    let mix_ratio = determine_mix_r(w, v, u, args.sw, do_swash, params.superness);
    let mx = mix(to_straight.x, finish.x, mix_ratio)
        + (if at_bottom { 1.0 } else { -1.0 }) * 0.75 * params.correction_o_mid_x * args.sw;

    // The pair's direction offsets the two knots. The heading is one of the
    // geometry package's four constants, which point along the stroke rather
    // than along the axes.
    let key_dir = if ltr {
        Direction::Right
    } else {
        Direction::Left
    };
    let key_heading = rightward();
    let extra_slope = (if ltr { 1.0 } else { -1.0 }) * 0.5 * (args.sw - args.sw_terminal) / args.sw;
    let final_turn_slope =
        key_heading.x + extra_slope - (if ltr { 1.0 } else { -1.0 }) * 0.5 * params.tan_slope;
    let key_knot = kit.pair(
        Family::G4,
        key_dir,
        Position::Mid,
        mx,
        y,
        kit.heading(Vec2::new(final_turn_slope, key_heading.y)),
    );

    let rad = w.min(mix_ratio * u);
    let skew0 = (w - v).clamp(0.0, w) / rad + ((mix_ratio * (u / w)).clamp(1.0, 1.5) - 1.0) * 0.5;
    let depth = v + skew0 * args.sw - args.sw;
    let shallow_limit = args.sw / 2.0;
    let skew = (skew0 + (shallow_limit - depth).clamp(0.0, shallow_limit) / rad).clamp(0.0, 0.5);
    let head_direction = if do_swash {
        let length = (1.0 + skew * skew).sqrt();
        Vec2::new(
            params.contrast / length * (if dtu { -1.0 } else { 1.0 }),
            skew / length * (if ltr { 1.0 } else { -1.0 }),
        )
    } else {
        Vec2::new(params.contrast * (if dtu { -1.0 } else { 1.0 }), 0.0)
    };
    finish.af.push(Amendment::Heading(head_direction));

    let mix_ratio_adjust = mix(0.5, mix_ratio, if do_swash { 1.0 } else { 0.125 }).max(0.5);
    let span = archv_y_from_x(1.0 / mix_ratio_adjust - 1.0, params.superness);
    let mut before_segment = Vec::new();
    let mut after_segment = Vec::new();
    for index in 1..SEGMENTS {
        let fraction = index as f64 / SEGMENTS as f64;
        let unimportant: Af = vec![Amendment::Unimportant];
        before_segment.push(kit.knot(
            KnotType::G4,
            mix(mx, to_straight.x, fraction),
            mix(y, to_straight.y, archv_y_from_x(fraction, params.superness)),
            unimportant.clone(),
        ));
        let far = fraction * (1.0 / mix_ratio_adjust - 1.0);
        after_segment.push(kit.knot(
            KnotType::G4,
            mix(mx, finish.x, fraction),
            mix(y, finish.y, archv_y_from_x(far, params.superness) / span),
            unimportant,
        ));
    }

    let mut items = Vec::new();
    if is_start {
        items.extend(after_segment.into_iter().rev());
        items.push(key_knot);
        items.extend(before_segment);
    } else {
        items.extend(before_segment.into_iter().rev());
        items.push(key_knot);
        items.extend(after_segment);
    }

    HookOut {
        items,
        terminal: finish,
    }
}

/// The ring an `OBar` side is drawn in: the box, the stroke, the fine width at
/// the shoulder, and the two arch depths.
///
/// `OBarLeft` and `OBarRight` are one ring seen from either side, and
/// `OBarRight` is `OBarLeft` rotated half a turn about the ring's centre.
#[derive(Clone, Copy, Debug)]
pub struct OBarRing {
    pub top: f64,
    pub bot: f64,
    pub left: f64,
    pub right: f64,
    pub sw: f64,
    pub fine: f64,
    pub ada: f64,
    pub adb: f64,
}

/// What an `OBar` side reads besides its ring.
#[derive(Clone, Copy)]
pub struct OBarContext<'a> {
    pub kit: &'a Kit,
    pub metrics: &'a Metrics,
    pub design: &'a Design,
}

impl OBarContext<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    fn adjust_x_bot(&self, x: f64, sw: f64) -> f64 {
        x + self.metrics.correction_o_mid_x * sw
    }

    fn adjust_x_top(&self, x: f64, sw: f64) -> f64 {
        x - self.metrics.correction_o_mid_x * sw
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
}

/// Rotate a shape half a turn about the ring's centre.
fn flip(shape: &Dispiro, ring: &OBarRing) -> Geometry {
    let mid_x = mix(ring.left, ring.right, 0.5);
    let mid_y = mix(ring.bot, ring.top, 0.5);
    let tf = kurbo::Affine::translate((mid_x, mid_y))
        * kurbo::Affine::scale_non_uniform(-1.0, -1.0)
        * kurbo::Affine::translate((-mid_x, -mid_y));

    Geometry::Transformed {
        tf,
        geom: Box::new(shape.geometry(kurbo::Affine::IDENTITY)),
    }
}

/// `OBarLeft.shape`: the flat-sided ring's left side.
#[must_use]
pub fn obar_left_shape(ctx: &OBarContext<'_>, ring: &OBarRing) -> Dispiro {
    let metrics = ctx.metrics;
    let params = ctx.arch_params();
    let OBarRing {
        top,
        bot,
        left,
        right,
        sw,
        fine,
        ada,
        adb,
    } = *ring;
    let x = left + ctx.hsw_to_v(sw - fine);

    let flat_side = |x: f64, ox: f64, af: Af| FlatSide {
        x,
        d: bot,
        u: top,
        ada,
        adb,
        ox,
        af,
    };
    let start = flatside_ld(ctx.kit, &flat_side(x, 0.0, Vec::new()));
    let middle = flatside_ru(ctx.kit, &flat_side(right, metrics.ox, Vec::new()));
    let end = flatside_ld(ctx.kit, &flat_side(x, 0.0, ctx.kit.widths_lhs(Some(fine))));

    let arch_bottom = ArchArgs {
        y: bot,
        p: None,
        sw,
        compact: false,
        o: metrics.o,
        angle_pre: None,
        angle_post: None,
        blend_pre: None,
        blend_post: None,
        sw_before: Some(fine),
        sw_after: Some(sw),
        lhs: true,
    };
    let arch_top = ArchArgs {
        y: top,
        sw_before: Some(sw),
        sw_after: Some(fine),
        ..arch_bottom
    };

    let mut items = vec![Item::Af(ctx.kit.widths_lhs(Some(fine)))];
    items.extend(start.iter().cloned());
    items.extend(arch(
        ctx.kit,
        &arch_bottom,
        &knot_of(start.last().expect("flatside item")),
        &knot_of(&middle[0]),
        &params,
    ));
    items.extend(middle.iter().cloned());
    items.extend(arch(
        ctx.kit,
        &arch_top,
        &knot_of(middle.last().expect("flatside item")),
        &knot_of(&end[0]),
        &params,
    ));
    items.extend(end);

    ctx.kit.dispiro(items)
}

/// `OBarLeft.toothlessCorner`: the left side cut at `ystart` with the bar's
/// foot replaced by a corner at `rise`.
#[must_use]
pub fn obar_left_toothless_corner(
    ctx: &OBarContext<'_>,
    ring: &OBarRing,
    rise: f64,
    ystart: f64,
) -> Dispiro {
    let metrics = ctx.metrics;
    let OBarRing {
        top,
        bot,
        left,
        right,
        sw,
        fine,
        ada,
        adb,
    } = *ring;
    let x = left + ctx.hsw_to_v(sw - fine);
    let flat_y = (top - ada).max(ystart + metrics.tiny);
    let flat = UserControlKnot {
        ty: KnotType::Left,
        x,
        y: flat_y,
        af: ctx.kit.widths_lhs(Some(fine)),
    };
    let right_side = flatside_ru(
        ctx.kit,
        &FlatSide {
            x: right,
            d: bot,
            u: top,
            ada,
            adb,
            ox: metrics.ox,
            af: Vec::new(),
        },
    );

    let mut items = vec![Item::Af(ctx.kit.widths_lhs(Some(sw)))];
    if rise == 0.0 {
        items.push(ctx.kit.knot(
            KnotType::Left,
            left + metrics.tiny,
            bot,
            ctx.kit.heading(rightward()),
        ));
        items.push(ctx.kit.knot(
            KnotType::Right,
            ctx.adjust_x_bot(mix(left, right, 0.5), sw),
            bot,
            Vec::new(),
        ));
        items.push(ctx.kit.archv(None, None));
    } else {
        let corner = UserControlKnot {
            ty: KnotType::G4,
            x: left,
            y: bot + rise,
            af: Vec::new(),
        };
        items.push(
            ctx.kit
                .knot(corner.ty, corner.x, corner.y, corner.af.clone()),
        );
        items.extend(arch(
            ctx.kit,
            &ArchArgs {
                y: bot,
                p: None,
                sw,
                compact: false,
                o: metrics.o,
                angle_pre: None,
                angle_post: None,
                blend_pre: Some(false),
                blend_post: None,
                sw_before: Some(sw),
                sw_after: Some(sw),
                lhs: true,
            },
            &corner,
            &knot_of(&right_side[0]),
            &ctx.arch_params(),
        ));
    }

    items.extend(right_side.iter().cloned());
    items.extend(arch(
        ctx.kit,
        &ArchArgs {
            y: top,
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
        &knot_of(right_side.last().expect("flatside item")),
        &flat,
        &ctx.arch_params(),
    ));
    items.push(ctx.kit.knot(flat.ty, flat.x, flat.y, flat.af));
    items.push(
        ctx.kit
            .knot(KnotType::Right, x, ystart, ctx.kit.widths_lhs(Some(fine))),
    );

    ctx.kit.dispiro(items)
}

/// `OBarLeft.toothlessRounded`: the left side cut at `ystart` with the bar's
/// foot rounded into the bar at `yTerminal`.
#[must_use]
pub fn obar_left_toothless_rounded(
    ctx: &OBarContext<'_>,
    ring: &OBarRing,
    y_terminal: f64,
    ystart: f64,
) -> Dispiro {
    let metrics = ctx.metrics;
    let OBarRing {
        top,
        bot,
        left,
        right,
        sw,
        fine,
        ada,
        adb,
    } = *ring;
    let x = left + ctx.hsw_to_v(sw - fine);
    let flat_y = (top - ada).max(ystart + metrics.tiny);
    let flat = UserControlKnot {
        ty: KnotType::Left,
        x,
        y: flat_y,
        af: ctx.kit.widths_lhs(Some(fine)),
    };
    let right_side = flatside_ru(
        ctx.kit,
        &FlatSide {
            x: right,
            d: bot,
            u: top,
            ada,
            adb,
            ox: metrics.ox,
            af: Vec::new(),
        },
    );

    let mut items = vec![
        Item::Af(ctx.kit.widths_lhs(Some(sw))),
        ctx.kit.knot(
            KnotType::Left,
            left,
            y_terminal,
            ctx.kit
                .widths_lhs_heading(Some(sw), downward(metrics.hv_contrast)),
        ),
        ctx.kit.knot(
            KnotType::Right,
            left,
            (bot + adb).min(y_terminal - metrics.tiny),
            Vec::new(),
        ),
    ];
    items.extend(arch(
        ctx.kit,
        &ArchArgs {
            y: bot,
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
        &knot_of(items.last().expect("knot")),
        &knot_of(&right_side[0]),
        &ctx.arch_params(),
    ));
    items.extend(right_side.iter().cloned());
    items.extend(arch(
        ctx.kit,
        &ArchArgs {
            y: top,
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
        &knot_of(right_side.last().expect("flatside item")),
        &flat,
        &ctx.arch_params(),
    ));
    items.push(ctx.kit.knot(flat.ty, flat.x, flat.y, flat.af));
    items.push(
        ctx.kit
            .knot(KnotType::Right, x, ystart, ctx.kit.widths_lhs(Some(fine))),
    );

    ctx.kit.dispiro(items)
}

/// `OBarRight.shape`: the left shape rotated half a turn about the ring.
#[must_use]
pub fn obar_right_shape(ctx: &OBarContext<'_>, ring: &OBarRing) -> Geometry {
    flip(&obar_left_shape(ctx, ring), ring)
}

/// `OBarRight.earlessCorner`, which is `OBarLeft.toothlessCorner` flipped.
#[must_use]
pub fn obar_right_earless_corner(
    ctx: &OBarContext<'_>,
    ring: &OBarRing,
    rise: f64,
    yend: f64,
) -> Geometry {
    flip(
        &obar_left_toothless_corner(ctx, ring, rise, ring.top - yend),
        ring,
    )
}

/// `OBarRight.earlessRounded`, which is `OBarLeft.toothlessRounded` flipped.
#[must_use]
pub fn obar_right_earless_rounded(
    ctx: &OBarContext<'_>,
    ring: &OBarRing,
    y_terminal: f64,
    yend: f64,
) -> Geometry {
    flip(
        &obar_left_toothless_rounded(ctx, ring, ring.top - y_terminal, ring.top - yend),
        ring,
    )
}

/// Which side of a serif the ink is on.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HSerifSide {
    /// `lt`: the left shoulder, ink on the left.
    LeftTop,
    /// `lb`: the left foot, ink on the right.
    LeftBottom,
    /// `rt`: the right shoulder, ink on the right.
    RightTop,
    /// `rb`: the right foot, ink on the left.
    RightBottom,
}

/// `HSerif`: a horizontal serif at `x`, `length` long, pointing away from the
/// bar.
///
/// `sw` is the stroke the serif is drawn with and `sw_ref` the stroke the flat
/// is placed against. A definition that omits either falls back to the stroke
/// width and to `sw`, which is what `None` means.
// The signature is upstream's, and the side is a parameter rather than four
// near-identical functions.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn hserif(
    kit: &Kit,
    metrics: &Metrics,
    side: HSerifSide,
    x: f64,
    y: f64,
    length: f64,
    sw: f64,
    sw_ref: Option<f64>,
) -> Dispiro {
    let sw_ref = sw_ref.unwrap_or(sw);
    let half = 0.5 * sw_ref * metrics.hv_contrast;
    let shift = sw * metrics.serif_shift_x;
    let (widths, flat_x, heading, end_x) = match side {
        HSerifSide::LeftTop => (
            kit.widths_lhs(Some(sw)),
            x + half,
            leftward(),
            x - length - shift,
        ),
        HSerifSide::LeftBottom => (
            kit.widths_rhs(Some(sw)),
            x + half,
            leftward(),
            x - length + shift,
        ),
        HSerifSide::RightTop => (
            kit.widths_rhs(Some(sw)),
            x - half,
            rightward(),
            x + length - shift,
        ),
        HSerifSide::RightBottom => (
            kit.widths_lhs(Some(sw)),
            x - half,
            rightward(),
            x + length + shift,
        ),
    };
    let items = vec![
        Item::Af(widths),
        kit.knot(KnotType::Left, flat_x, y, kit.heading(heading)),
        kit.knot(KnotType::Right, end_x, y, Vec::new()),
    ];

    kit.dispiro(items)
}

/// `VBar.l` and `VBar.r`: a vertical bar, its ink on one side of `x`.
#[must_use]
pub fn vbar_l(kit: &Kit, metrics: &Metrics, x: f64, y_down: f64, y_up: f64, sw: f64) -> Dispiro {
    vbar(
        kit,
        metrics,
        x + 0.5 * sw * metrics.hv_contrast,
        y_down,
        y_up,
        sw,
    )
}

/// `VBar.r`, the right-hand twin of [`vbar_l`].
#[must_use]
pub fn vbar_r(kit: &Kit, metrics: &Metrics, x: f64, y_down: f64, y_up: f64, sw: f64) -> Dispiro {
    vbar(
        kit,
        metrics,
        x - 0.5 * sw * metrics.hv_contrast,
        y_down,
        y_up,
        sw,
    )
}

fn vbar(kit: &Kit, metrics: &Metrics, x: f64, y_down: f64, y_up: f64, sw: f64) -> Dispiro {
    let heading = if y_down < y_up {
        upward(metrics.hv_contrast)
    } else {
        downward(metrics.hv_contrast)
    };
    let items = vec![
        Item::Af(kit.widths_center(Some(sw))),
        kit.knot(KnotType::Left, x, y_down, kit.heading(heading)),
        kit.knot(KnotType::Right, x, y_up, kit.heading(heading)),
    ];

    kit.dispiro(items)
}

/// `HBar.m`: a horizontal bar from `x_left` to `x_right` at `y`, centred on it.
#[must_use]
pub fn hbar_m(
    kit: &Kit,
    metrics: &Metrics,
    x_left: f64,
    x_right: f64,
    y: f64,
    fine: Option<f64>,
) -> Dispiro {
    let width = fine.unwrap_or(metrics.stroke);
    let items = vec![
        Item::Af(kit.widths_center(Some(width))),
        kit.knot(KnotType::Left, x_left, y, kit.heading(rightward())),
        kit.knot(KnotType::Right, x_right, y, kit.heading(rightward())),
    ];

    kit.dispiro(items)
}

/// `HBar.t`: the bar's top edge at `y`.
#[must_use]
pub fn hbar_t(
    kit: &Kit,
    metrics: &Metrics,
    x_left: f64,
    x_right: f64,
    y: f64,
    fine: Option<f64>,
) -> Dispiro {
    let width = fine.unwrap_or(metrics.stroke);

    hbar_m(kit, metrics, x_left, x_right, y - width / 2.0, fine)
}

/// `HBar.b`: the bar's bottom edge at `y`.
#[must_use]
pub fn hbar_b(
    kit: &Kit,
    metrics: &Metrics,
    x_left: f64,
    x_right: f64,
    y: f64,
    fine: Option<f64>,
) -> Dispiro {
    let width = fine.unwrap_or(metrics.stroke);

    hbar_m(kit, metrics, x_left, x_right, y + width / 2.0, fine)
}

/// Which end of a vertical serif the ink sits on.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VSerifSide {
    /// `dr`: below `y`, ink on the right.
    DownRight,
    /// `dl`: below `y`, ink on the left.
    DownLeft,
    /// `ur`: above `y`, ink on the left.
    UpRight,
    /// `ul`: above `y`, ink on the right.
    UpLeft,
}

/// `VSerif`: a vertical serif at `x`, `length` long, pointing away from the bar.
#[must_use]
pub fn vserif(
    kit: &Kit,
    metrics: &Metrics,
    side: VSerifSide,
    x: f64,
    y: f64,
    length: f64,
    sw: Option<f64>,
) -> Dispiro {
    let width = sw.unwrap_or(metrics.v_jut_stroke);
    let (widths, down, heading) = match side {
        VSerifSide::DownRight => (
            kit.widths_rhs(Some(width)),
            true,
            downward(metrics.hv_contrast),
        ),
        VSerifSide::DownLeft => (
            kit.widths_lhs(Some(width)),
            true,
            downward(metrics.hv_contrast),
        ),
        VSerifSide::UpRight => (
            kit.widths_lhs(Some(width)),
            false,
            upward(metrics.hv_contrast),
        ),
        VSerifSide::UpLeft => (
            kit.widths_rhs(Some(width)),
            false,
            upward(metrics.hv_contrast),
        ),
    };
    let end = if down { y - length } else { y + length };
    let items = vec![
        Item::Af(widths),
        kit.knot(KnotType::Left, x, y, kit.heading(heading)),
        kit.knot(KnotType::Right, x, end, kit.heading(heading)),
    ];

    kit.dispiro(items)
}

/// `FlatHookDepth`: how far a flat hook reaches along each axis.
#[derive(Clone, Copy, Debug)]
pub struct FlatHookDepth {
    pub x: f64,
    pub y: f64,
}

/// `FlatHookDepthImpl`: one axis of the flat hook's depth.
fn flat_hook_depth_impl(metrics: &Metrics, hook_scalar: f64, stroke_scalar: f64) -> f64 {
    const K: f64 = 15.0 / 16.0;
    let hook = K * hook_scalar * metrics.hook;
    let stroke = stroke_scalar * metrics.stroke;

    hook.max(stroke * 1.375).max(hook.min(stroke * 1.625))
}

/// `FlatHookDepth`, read from the frame's width.
// `sqrt` is not const.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
pub fn flat_hook_depth(metrics: &Metrics, adws: f64) -> FlatHookDepth {
    let hook_scalar = adws.sqrt().min(1.0);

    FlatHookDepth {
        x: flat_hook_depth_impl(metrics, hook_scalar, metrics.hv_contrast),
        y: flat_hook_depth_impl(metrics, hook_scalar, 1.0),
    }
}

/// `VerticalHook.m`: a hook leaving a vertical stem.
// The signature is upstream's: the stem, the two depths, the stroke and the extension.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn vertical_hook_m(
    kit: &Kit,
    metrics: &Metrics,
    x: f64,
    y: f64,
    x_depth: f64,
    y_depth: f64,
    sw: f64,
    y_extension: f64,
) -> Dispiro {
    let hd = flat_hook_depth(metrics, 1.0);
    let y2 = if y_depth > 0.0 {
        (y - y_depth + hd.y - 0.25 * sw).min(y - metrics.tiny)
    } else {
        (y - y_depth - hd.y + 0.25 * sw).max(y + metrics.tiny)
    };
    let x3 = if x_depth > 0.0 {
        (x + x_depth - metrics.tiny).min(x + hd.x - 0.25 * sw * metrics.hv_contrast)
    } else {
        (x + x_depth + metrics.tiny).max(x - hd.x + 0.25 * sw * metrics.hv_contrast)
    };
    let heading = if y_depth > 0.0 {
        downward(metrics.hv_contrast)
    } else {
        upward(metrics.hv_contrast)
    };
    let items = vec![
        Item::Af(kit.widths_center(Some(sw))),
        kit.knot(KnotType::Left, x, y + y_extension, kit.heading(heading)),
        kit.knot(KnotType::Right, x, y2, kit.heading(heading)),
        kit.arcvh(None, None),
        kit.knot(KnotType::Left, x3, y - y_depth, Vec::new()),
        kit.knot(KnotType::Right, x + x_depth, y - y_depth, Vec::new()),
    ];

    kit.dispiro(items)
}

/// `VerticalHook.l`: the same hook with its ink on the left of `x`.
// The signature is upstream's: the stem, the two depths, the stroke and the extension.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn vertical_hook_l(
    kit: &Kit,
    metrics: &Metrics,
    x: f64,
    y: f64,
    x_depth: f64,
    y_depth: f64,
    sw: f64,
    y_extension: f64,
) -> Dispiro {
    vertical_hook_m(
        kit,
        metrics,
        x + 0.5 * sw * metrics.hv_contrast,
        y,
        x_depth,
        y_depth,
        sw,
        y_extension,
    )
}

/// `TopHook.toRight.lBarInner`: a stem whose top turns right and down into a
/// hook. `sign` is `+1` for a rightward hook and `-1` for a leftward one.
#[must_use]
pub fn top_hook_l_bar_inner(
    kit: &Kit,
    metrics: &Metrics,
    x: f64,
    y_bot: f64,
    y_top: f64,
    sw: f64,
    sign: f64,
) -> Vec<Geometry> {
    let y_hook_start = y_top - metrics.o - metrics.tail_y - 0.5 * sw;
    let stem = vbar_l(kit, metrics, x, y_bot, y_hook_start, sw).geometry(kurbo::Affine::IDENTITY);
    let hook = vertical_hook_l(
        kit,
        metrics,
        x,
        y_hook_start,
        sign * metrics.tail_x,
        -metrics.tail_y,
        sw,
        0.0,
    )
    .geometry(kurbo::Affine::IDENTITY);

    vec![stem, hook]
}

/// `HOverlayBar`: a horizontal overlay bar.
#[must_use]
pub fn h_overlay_bar(
    kit: &Kit,
    metrics: &Metrics,
    x_left: f64,
    x_right: f64,
    y: f64,
    sw: Option<f64>,
) -> Dispiro {
    let width = sw.unwrap_or(metrics.overlay_stroke);
    let items = vec![
        Item::Af(kit.widths_center(Some(width))),
        kit.knot(KnotType::Left, x_left, y, Vec::new()),
        kit.knot(KnotType::Right, x_right, y, Vec::new()),
    ];

    kit.dispiro(items)
}

/// `LetterBarOverlay`: a bar across the letter, at a fraction `py` of the way
/// from `bot` to `top`.
///
/// `p_x_in_sw` places the reference point: 0 at the left edge, 1 at the right.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn letter_bar_overlay_in(
    kit: &Kit,
    metrics: &Metrics,
    x: f64,
    bot: f64,
    top: f64,
    py: f64,
    space: (f64, f64),
    ref_sw: f64,
    sw: f64,
    p_x_in_sw: f64,
) -> Dispiro {
    let (x_left_space, x_right_space) = space;
    let hsw = |value: f64| value * metrics.hv_contrast;
    let x_left_edge = x - p_x_in_sw * hsw(ref_sw);
    let x_right_edge = x + (1.0 - p_x_in_sw) * hsw(ref_sw);
    let p = (x_left_edge - x_left_space)
        / ((x_right_space - x_right_edge) + (x_left_edge - x_left_space));
    let x_c = mix(x_left_edge, x_right_edge, mix(0.5, 1.0 - p, 0.5));
    let jut = 0.75 * metrics.long_jut;
    let left =
        (x_left_edge - metrics.side_jut).min((x_c - jut).max(mix(x_left_space, x_left_edge, 0.3)));
    let right = (x_right_edge + metrics.side_jut).max((x_c + jut).min(mix(
        x_right_space,
        x_right_edge,
        0.3,
    )));
    let y = mix(bot, top, py);
    let sw = sw
        .min(1.25 * (top - bot) * py)
        .min(1.25 * (top - bot) * (1.0 - py));

    h_overlay_bar(kit, metrics, left, right, y, Some(sw))
}

/// `LetterBarOverlay.l.in`.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn letter_bar_overlay_l_in(
    kit: &Kit,
    metrics: &Metrics,
    x: f64,
    bot: f64,
    top: f64,
    py: f64,
    space: (f64, f64),
    ref_sw: f64,
    sw: f64,
) -> Dispiro {
    letter_bar_overlay_in(kit, metrics, x, bot, top, py, space, ref_sw, sw, 0.0)
}

/// `LetterBarOverlay.r.in`.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn letter_bar_overlay_r_in(
    kit: &Kit,
    metrics: &Metrics,
    x: f64,
    bot: f64,
    top: f64,
    py: f64,
    space: (f64, f64),
    ref_sw: f64,
    sw: f64,
) -> Dispiro {
    letter_bar_overlay_in(kit, metrics, x, bot, top, py, space, ref_sw, sw, 1.0)
}

/// The arch parameters a definition reads from the design and the metrics.
const fn design_arch_params(metrics: &Metrics, design: &Design) -> ArchParams {
    ArchParams {
        superness: design.superness,
        tan_slope: metrics.tan_slope,
        hv_contrast: metrics.hv_contrast,
        o: metrics.o,
        correction_o_mid_x: metrics.correction_o_mid_x,
    }
}

/// `OShapeOutline`: a whole ring as a closed spiro outline.
///
/// The two `centerAt` knots close the top and bottom midsections, and the flat
/// sides carry the two arch depths. `OShapeOutline.NoOvershoot` is this with
/// the box pushed out by `O` and `OX`.
// The signature is upstream's: the box, the width and the two arch depths.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn oshape_outline(
    kit: &Kit,
    metrics: &Metrics,
    design: &Design,
    u: f64,
    d: f64,
    l: f64,
    r: f64,
    width: Option<f64>,
    ada: Option<f64>,
    adb: Option<f64>,
) -> crate::spiro_kit::SpiroOutline {
    let width = width.unwrap_or(metrics.stroke);
    let ada = ada.unwrap_or(metrics.small_arch_depth_a);
    let adb = adb.unwrap_or(metrics.small_arch_depth_b);
    let middle = mix(l, r, 0.5);
    let params = design_arch_params(metrics, design);
    let end = |x: f64, y: f64, rtl: bool, at_bot: bool| {
        center_at(
            kit,
            &CenterAtArgs {
                x,
                y,
                sw: width,
                compact: false,
                o: metrics.o,
                sw_before: width,
                sw_after: width,
                lhs: true,
                rtl,
                at_bot,
            },
            &params,
        )
    };
    let side = |x: f64| FlatSide {
        x,
        d,
        u,
        ada,
        adb,
        ox: metrics.ox,
        af: Vec::new(),
    };
    let mut items = vec![
        Item::Af(kit.widths(width, 0.0)),
        end(middle, u, true, false),
        kit.archv(None, None),
    ];
    items.extend(flatside_ld(kit, &side(l)));
    items.push(kit.arcvh(None, None));
    items.push(end(middle, d, false, true));
    items.push(kit.archv(None, None));
    items.extend(flatside_ru(kit, &side(r)));
    items.push(kit.arcvh(None, None));
    items.push(kit.close());

    kit.spiro_outline(items)
}

/// `OShapeOutline.NoOvershoot`: the ring's box pushed out by the overshoot.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn oshape_outline_no_overshoot(
    kit: &Kit,
    metrics: &Metrics,
    design: &Design,
    u: f64,
    d: f64,
    l: f64,
    r: f64,
    width: Option<f64>,
    ada: Option<f64>,
    adb: Option<f64>,
) -> crate::spiro_kit::SpiroOutline {
    oshape_outline(
        kit,
        metrics,
        design,
        u + metrics.o,
        d - metrics.o,
        l - metrics.ox,
        r + metrics.ox,
        width,
        ada,
        adb,
    )
}

/// `BBS`: the stroke a blackboard bar is drawn with.
#[must_use]
pub fn bb_stroke(df: &crate::letter_model::DivFrame) -> f64 {
    df.advice_stroke(6.0, 1.0)
}

/// `BBD`: the distance between a blackboard bar's two strokes.
#[must_use]
pub fn bb_distance(metrics: &Metrics, bbs: f64) -> f64 {
    crate::util::clamp(metrics.width / 5.5, metrics.width / 4.0, bbs * 2.0)
}

/// `BBBarLeft`: a blackboard bar on the left of `x`.
#[must_use]
pub fn bb_bar_left(
    kit: &Kit,
    metrics: &Metrics,
    x: f64,
    y_bot: f64,
    y_top: f64,
    bbs: f64,
    bbd: f64,
) -> Vec<Geometry> {
    vec![
        vbar_l(kit, metrics, x, y_bot, y_top, bbs).geometry(kurbo::Affine::IDENTITY),
        vbar_l(kit, metrics, x + bbd, y_bot, y_top, bbs).geometry(kurbo::Affine::IDENTITY),
        hbar_t(kit, metrics, x, x + bbd, y_top, Some(bbs)).geometry(kurbo::Affine::IDENTITY),
        hbar_b(kit, metrics, x, x + bbd, y_bot, Some(bbs)).geometry(kurbo::Affine::IDENTITY),
    ]
}

/// `RightwardTailedBar`: a vertical bar whose foot turns right.
#[must_use]
pub fn rightward_tailed_bar(
    kit: &Kit,
    metrics: &Metrics,
    _df: &crate::letter_model::DivFrame,
    x: f64,
    low: f64,
    high: f64,
    sw: f64,
) -> Dispiro {
    // `Math.max` takes three arguments upstream: the indented `AdviceStroke 8`
    // and `0.625 * SB` are siblings of `SideJut` rather than one being an
    // argument of the other.
    // `AdviceStroke` and `SB` are the build's, not the frame's: a definition
    // that draws the bar in a narrower frame still reads the global advice.
    let weight = crate::metrics::WeightControl::new(
        metrics.stroke,
        metrics.width,
        metrics.sb,
        metrics.hv_contrast,
    );
    let hook_depth = metrics
        .side_jut
        .max(weight.advice_stroke(8.0, 1.0, 1.0))
        .max(0.625 * metrics.sb);
    let advice16 = weight.advice_stroke(16.0, 1.0, 1.0);
    let hook_turn = advice16.max(hook_depth - advice16);
    let items = vec![
        Item::Af(kit.widths_lhs(Some(sw))),
        kit.knot(
            KnotType::Left,
            x - sw * metrics.hv_contrast,
            high,
            kit.heading(downward(metrics.hv_contrast)),
        ),
        kit.knot(
            KnotType::Right,
            x - sw * metrics.hv_contrast,
            low + metrics.o + sw + hook_turn,
            kit.heading(downward(metrics.hv_contrast)),
        ),
        kit.arcvh(None, None),
        kit.knot(
            KnotType::Left,
            x + hook_turn,
            low + metrics.o,
            kit.heading(rightward()),
        ),
        kit.knot(
            KnotType::Right,
            x + hook_depth + 0.0_f64.max(sw * metrics.tan_slope),
            low + metrics.o,
            Vec::new(),
        ),
    ];

    kit.dispiro(items)
}

/// `OBarLeft.earlessCorner`: the left side cut at `yend` with the bar's ear
/// replaced by a corner at `rise`.
#[must_use]
pub fn obar_left_earless_corner(
    ctx: &OBarContext<'_>,
    ring: &OBarRing,
    rise: f64,
    yend: f64,
) -> Dispiro {
    let metrics = ctx.metrics;
    let OBarRing {
        top,
        bot,
        left,
        right,
        sw,
        fine,
        ada,
        adb,
    } = *ring;
    let x = left + ctx.hsw_to_v(sw - fine);
    let right_side = flatside_ru(
        ctx.kit,
        &FlatSide {
            x: right,
            d: bot,
            u: top,
            ada,
            adb,
            ox: metrics.ox,
            af: Vec::new(),
        },
    );
    let start = UserControlKnot {
        ty: KnotType::Right,
        x,
        y: (bot + adb).min(yend - metrics.tiny),
        af: ctx.kit.widths_lhs(Some(fine)),
    };
    let mut items = vec![
        Item::Af(ctx.kit.widths_lhs(Some(fine))),
        ctx.kit
            .knot(KnotType::Left, x, yend, ctx.kit.widths_lhs(Some(fine))),
        ctx.kit.knot(start.ty, start.x, start.y, start.af.clone()),
    ];
    items.extend(arch(
        ctx.kit,
        &ArchArgs {
            y: bot,
            p: None,
            sw,
            compact: false,
            o: metrics.o,
            angle_pre: None,
            angle_post: None,
            blend_pre: None,
            blend_post: None,
            sw_before: Some(fine),
            sw_after: Some(sw),
            lhs: true,
        },
        &start,
        &knot_of(&right_side[0]),
        &ctx.arch_params(),
    ));
    items.extend(right_side.iter().cloned());
    if rise == 0.0 {
        items.push(ctx.kit.arcvh(None, None));
        items.push(ctx.kit.knot(
            KnotType::Left,
            ctx.adjust_x_top(mix(left, right, 0.5), sw),
            top,
            Vec::new(),
        ));
        items.push(ctx.kit.knot(
            KnotType::Right,
            left + metrics.tiny,
            top,
            ctx.kit.heading(leftward()),
        ));
    } else {
        let corner = UserControlKnot {
            ty: KnotType::G4,
            x: left,
            y: top - rise,
            af: ctx.kit.widths_lhs(Some(sw)),
        };
        items.extend(arch(
            ctx.kit,
            &ArchArgs {
                y: top,
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
            &knot_of(right_side.last().expect("flatside item")),
            &corner,
            &ctx.arch_params(),
        ));
        items.push(ctx.kit.knot(corner.ty, corner.x, corner.y, corner.af));
    }

    ctx.kit.dispiro(items)
}

/// `OBarLeft.earlessRounded`: the left side cut at `yend` with the bar's ear
/// rounded away at `yTerminal`.
#[must_use]
pub fn obar_left_earless_rounded(
    ctx: &OBarContext<'_>,
    ring: &OBarRing,
    y_terminal: f64,
    yend: f64,
) -> Dispiro {
    let metrics = ctx.metrics;
    let OBarRing {
        top,
        bot,
        left,
        right,
        sw,
        fine,
        ada,
        adb,
    } = *ring;
    let x = left + ctx.hsw_to_v(sw - fine);
    let right_side = flatside_ru(
        ctx.kit,
        &FlatSide {
            x: right,
            d: bot,
            u: top,
            ada,
            adb,
            ox: metrics.ox,
            af: Vec::new(),
        },
    );
    let start = UserControlKnot {
        ty: KnotType::Right,
        x,
        y: (bot + adb).min(yend - metrics.tiny),
        af: ctx.kit.widths_lhs(Some(fine)),
    };
    let mut items = vec![
        Item::Af(ctx.kit.widths_lhs(Some(fine))),
        ctx.kit
            .knot(KnotType::Left, x, yend, ctx.kit.widths_lhs(Some(fine))),
        ctx.kit.knot(start.ty, start.x, start.y, start.af.clone()),
    ];
    items.extend(arch(
        ctx.kit,
        &ArchArgs {
            y: bot,
            p: None,
            sw,
            compact: false,
            o: metrics.o,
            angle_pre: None,
            angle_post: None,
            blend_pre: None,
            blend_post: None,
            sw_before: Some(fine),
            sw_after: Some(sw),
            lhs: true,
        },
        &start,
        &knot_of(&right_side[0]),
        &ctx.arch_params(),
    ));
    items.extend(right_side.iter().cloned());
    let terminal = UserControlKnot {
        ty: KnotType::Left,
        x: left,
        y: (top - ada).max(y_terminal + metrics.tiny),
        af: Vec::new(),
    };
    items.extend(arch(
        ctx.kit,
        &ArchArgs {
            y: top,
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
        &knot_of(right_side.last().expect("flatside item")),
        &terminal,
        &ctx.arch_params(),
    ));
    items.push(
        ctx.kit
            .knot(terminal.ty, terminal.x, terminal.y, Vec::new()),
    );
    items.push(
        ctx.kit.knot(
            KnotType::Right,
            left,
            y_terminal,
            ctx.kit
                .widths_lhs_heading(Some(sw), downward(metrics.hv_contrast)),
        ),
    );

    ctx.kit.dispiro(items)
}

/// `OBarRight.toothlessCorner`: `OBarLeft.earlessCorner` rotated half a turn.
#[must_use]
pub fn obar_right_toothless_corner(
    ctx: &OBarContext<'_>,
    ring: &OBarRing,
    rise: f64,
    ystart: f64,
) -> Geometry {
    flip(
        &obar_left_earless_corner(ctx, ring, rise, ring.top - ystart),
        ring,
    )
}

/// `OBarRight.toothlessRounded`: `OBarLeft.earlessRounded` rotated half a turn.
#[must_use]
pub fn obar_right_toothless_rounded(
    ctx: &OBarContext<'_>,
    ring: &OBarRing,
    y_terminal: f64,
    ystart: f64,
) -> Geometry {
    flip(
        &obar_left_earless_rounded(ctx, ring, ring.top - y_terminal, ring.top - ystart),
        ring,
    )
}

/// `TopHook.rBarInner`: the hook at a bar's right edge.
#[must_use]
pub fn top_hook_r_bar_inner(
    kit: &Kit,
    metrics: &Metrics,
    x: f64,
    y_bot: f64,
    y_top: f64,
    sw: f64,
    sign: f64,
) -> Vec<Geometry> {
    top_hook_l_bar_inner(
        kit,
        metrics,
        x - sw * metrics.hv_contrast,
        y_bot,
        y_top,
        sw,
        sign,
    )
}

/// `BBBarRight`: a blackboard bar on the right of `x`.
#[must_use]
pub fn bb_bar_right(
    kit: &Kit,
    metrics: &Metrics,
    x: f64,
    y_bot: f64,
    y_top: f64,
    bbs: f64,
    bbd: f64,
) -> Vec<Geometry> {
    vec![
        vbar_r(kit, metrics, x, y_bot, y_top, bbs).geometry(kurbo::Affine::IDENTITY),
        vbar_r(kit, metrics, x - bbd, y_bot, y_top, bbs).geometry(kurbo::Affine::IDENTITY),
        hbar_t(kit, metrics, x, x - bbd, y_top, Some(bbs)).geometry(kurbo::Affine::IDENTITY),
        hbar_b(kit, metrics, x, x - bbd, y_bot, Some(bbs)).geometry(kurbo::Affine::IDENTITY),
    ]
}

/// `Rect`: an axis-aligned closed outline.
#[must_use]
pub fn rect(kit: &Kit, u: f64, d: f64, l: f64, r: f64) -> crate::spiro_kit::SpiroOutline {
    kit.spiro_outline(vec![
        kit.knot(KnotType::Corner, l, d, Vec::new()),
        kit.knot(KnotType::Corner, l, u, Vec::new()),
        kit.knot(KnotType::Corner, r, u, Vec::new()),
        kit.knot(KnotType::Corner, r, d, Vec::new()),
    ])
}

/// `VSerif.mr`: a centered vertical serif at `x`, `length` each way.
#[must_use]
pub fn vserif_mr(
    kit: &Kit,
    metrics: &Metrics,
    x: f64,
    y: f64,
    length: f64,
    sw: Option<f64>,
) -> Dispiro {
    let width = sw.unwrap_or(metrics.v_jut_stroke);
    let heading = upward(metrics.hv_contrast);
    let items = vec![
        Item::Af(kit.widths_lhs(Some(width))),
        kit.knot(KnotType::Left, x, y - length, kit.heading(heading)),
        kit.knot(KnotType::Right, x, y + length, kit.heading(heading)),
    ];

    kit.dispiro(items)
}

/// `EFMidBarShrinkK`: how far the crossbar's right end is pulled in.
#[must_use]
pub fn ef_mid_bar_shrink_k(
    df: &crate::letter_model::DivFrame,
    serif_rt: bool,
    sw_ref: Option<f64>,
) -> f64 {
    let sw_ref = sw_ref.unwrap_or(df.mvs);
    1.0 + ((0.15 * (df.width - 2.0 * df.sb)).max(0.5 * sw_ref)
        + if serif_rt { 0.25 } else { 0.0 } * sw_ref)
        / df.sb
}

/// `EFMidBarPosY`: where the crossbar sits.
#[must_use]
pub fn ef_mid_bar_pos_y(top: f64, bot: f64, py_bar: Option<f64>, design: &Design) -> f64 {
    mix(bot, top, py_bar.unwrap_or(design.upper_e_bar_pos))
}

/// `EFVJutLength`: the three jut lengths around the crossbar.
// The signature is upstream's: the height, the bar's fraction, the stroke and
// the two overrides.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn ef_v_jut_length(
    metrics: &Metrics,
    design: &Design,
    top: f64,
    bot: f64,
    py_bar: Option<f64>,
    stroke: Option<f64>,
    jut_top: Option<f64>,
    jut_bot: Option<f64>,
) -> (f64, f64, f64) {
    let height = top - bot;
    let bar_pos = ef_mid_bar_pos_y(height, 0.0, py_bar, design);
    let stroke = stroke.unwrap_or(metrics.stroke);
    let jut_top = jut_top.unwrap_or_else(|| {
        metrics
            .v_jut
            .min(mix(stroke, height - (bar_pos + stroke / 2.0), 0.5))
    });
    let jut_bot =
        jut_bot.unwrap_or_else(|| metrics.v_jut.min(mix(stroke, bar_pos - stroke / 2.0, 0.5)));
    let co_top = (height - (bar_pos - stroke / 2.0)) - jut_top;
    let co_bot = (bar_pos + stroke / 2.0) - jut_bot;
    let jut_mid = mix(
        stroke / 2.0,
        metrics.v_jut.min(co_top).min(co_bot) - stroke / 2.0,
        0.5,
    );

    (jut_top, jut_bot, jut_mid)
}

/// `VBar.m`: a vertical bar centred on `x`.
#[must_use]
pub fn vbar_m(kit: &Kit, metrics: &Metrics, x: f64, y_down: f64, y_up: f64, sw: f64) -> Dispiro {
    vbar(kit, metrics, x, y_down, y_up, sw)
}

/// `HSerif.mt`: a horizontal serif centred on `x` at the top.
#[must_use]
pub fn hserif_mt(
    kit: &Kit,
    metrics: &Metrics,
    x: f64,
    y: f64,
    length: f64,
    sw: Option<f64>,
) -> Dispiro {
    let sw = sw.unwrap_or(metrics.stroke);
    let shift = sw * metrics.serif_shift_x;
    let items = vec![
        Item::Af(kit.widths_lhs(Some(sw))),
        kit.knot(KnotType::Left, x + length - shift, y, Vec::new()),
        kit.knot(KnotType::Right, x - length - shift, y, Vec::new()),
    ];

    kit.dispiro(items)
}

/// `HSerif.mb`: a horizontal serif centred on `x` at the bottom.
#[must_use]
pub fn hserif_mb(
    kit: &Kit,
    metrics: &Metrics,
    x: f64,
    y: f64,
    length: f64,
    sw: Option<f64>,
) -> Dispiro {
    let sw = sw.unwrap_or(metrics.stroke);
    let shift = sw * metrics.serif_shift_x;
    let items = vec![
        Item::Af(kit.widths_rhs(Some(sw))),
        kit.knot(KnotType::Left, x + length + shift, y, Vec::new()),
        kit.knot(KnotType::Right, x - length + shift, y, Vec::new()),
    ];

    kit.dispiro(items)
}

/// `VSerif.ml`: a centered vertical serif with its ink on the right.
#[must_use]
pub fn vserif_ml(
    kit: &Kit,
    metrics: &Metrics,
    x: f64,
    y: f64,
    length: f64,
    sw: Option<f64>,
) -> Dispiro {
    let width = sw.unwrap_or(metrics.v_jut_stroke);
    let heading = upward(metrics.hv_contrast);
    let items = vec![
        Item::Af(kit.widths_rhs(Some(width))),
        kit.knot(KnotType::Left, x, y - length, kit.heading(heading)),
        kit.knot(KnotType::Right, x, y + length, kit.heading(heading)),
    ];

    kit.dispiro(items)
}

/// `BBBarCenter`: a centered blackboard bar.
#[must_use]
pub fn bb_bar_center(
    kit: &Kit,
    metrics: &Metrics,
    x: f64,
    y_bot: f64,
    y_top: f64,
    bbs: f64,
    bbd: f64,
) -> Vec<Geometry> {
    vec![
        vbar_m(kit, metrics, x - bbd / 2.0, y_bot, y_top, bbs).geometry(kurbo::Affine::IDENTITY),
        vbar_m(kit, metrics, x + bbd / 2.0, y_bot, y_top, bbs).geometry(kurbo::Affine::IDENTITY),
        hbar_t(kit, metrics, x - bbd / 2.0, x + bbd / 2.0, y_top, Some(bbs))
            .geometry(kurbo::Affine::IDENTITY),
        hbar_b(kit, metrics, x - bbd / 2.0, x + bbd / 2.0, y_bot, Some(bbs))
            .geometry(kurbo::Affine::IDENTITY),
    ]
}

/// `flatside.rd`: the right side of a ring, from top to bottom.
#[must_use]
pub fn flatside_rd(kit: &Kit, side: &FlatSide) -> Vec<Item> {
    let FlatSide {
        x,
        d,
        u,
        ada,
        adb,
        ox,
        af,
    } = side;
    if u - d > ada + adb {
        vec![
            kit.knot(KnotType::Left, x - ox, u - adb, af.clone()),
            kit.knot(KnotType::Right, x - ox, d + ada, af.clone()),
        ]
    } else {
        vec![kit.knot(
            KnotType::G4,
            x - ox,
            ysmooth_mid_r(*u, *d, Some(*ada), Some(*adb), *ada, *adb),
            af.clone(),
        )]
    }
}

/// `HSerif.mtAsymmetric`: a top serif extending `l` left and `r` right.
#[must_use]
pub fn hserif_mt_asymmetric(
    kit: &Kit,
    metrics: &Metrics,
    x: f64,
    y: f64,
    l: f64,
    r: f64,
    sw: Option<f64>,
) -> Dispiro {
    let sw = sw.unwrap_or(metrics.stroke);
    let shift = sw * metrics.serif_shift_x;
    let items = vec![
        Item::Af(kit.widths_lhs(Some(sw))),
        kit.knot(KnotType::Left, x + r - shift, y, Vec::new()),
        kit.knot(KnotType::Right, x - l - shift, y, Vec::new()),
    ];

    kit.dispiro(items)
}

/// `HSerif.mbAsymmetric`: a bottom serif extending `l` left and `r` right.
#[must_use]
pub fn hserif_mb_asymmetric(
    kit: &Kit,
    metrics: &Metrics,
    x: f64,
    y: f64,
    l: f64,
    r: f64,
    sw: Option<f64>,
) -> Dispiro {
    let sw = sw.unwrap_or(metrics.stroke);
    let shift = sw * metrics.serif_shift_x;
    let items = vec![
        Item::Af(kit.widths_rhs(Some(sw))),
        kit.knot(KnotType::Left, x + r + shift, y, Vec::new()),
        kit.knot(KnotType::Right, x - l + shift, y, Vec::new()),
    ];

    kit.dispiro(items)
}

/// `JutIn`: how far a serif reaches when the two sides share the space.
#[must_use]
pub fn jut_in(
    metrics: &Metrics,
    left: f64,
    right: f64,
    jut: f64,
    sw_ref: f64,
    h_split: f64,
) -> f64 {
    let ink = sw_ref * metrics.hv_contrast;
    let gap = (right - left - h_split * ink) / (h_split - 1.0);

    jut.min(0.5 * ink + (metrics.stroke * metrics.tan_slope).max(0.375 * gap))
}

/// `VerticalHook.r`: the same hook with its ink on the right of `x`.
// The signature is upstream's: the stem, the two depths, the stroke and the
// extension.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn vertical_hook_r(
    kit: &Kit,
    metrics: &Metrics,
    x: f64,
    y: f64,
    x_depth: f64,
    y_depth: f64,
    sw: f64,
    y_extension: f64,
) -> Dispiro {
    vertical_hook_m(
        kit,
        metrics,
        x - 0.5 * sw * metrics.hv_contrast,
        y,
        x_depth,
        y_depth,
        sw,
        y_extension,
    )
}

/// `EngHook`: the hook under an eng.
#[must_use]
pub fn eng_hook(
    kit: &Kit,
    metrics: &Metrics,
    x: f64,
    y_start: f64,
    y_end: f64,
    sw: f64,
) -> Dispiro {
    let y = y_end + metrics.hook + 0.5 * sw;

    vertical_hook_r(
        kit,
        metrics,
        x,
        y,
        -metrics.hook_x,
        metrics.hook,
        sw,
        0.0_f64.max(y_start - y),
    )
}
