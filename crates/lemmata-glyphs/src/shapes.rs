//! The shape helpers glyph definitions call: the hook and the arch.
//!
//! Ported from `common/shapes.ptl` (`HookShape`, `determineMixR`, the `Arch`
//! namespace) and the clamped arch depths in `meta/aesthetics.ptl`.
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

use kurbo::Vec2;
use lemmata_geom::KnotType;

use crate::control::{Af, Amendment, Item, UserControlKnot};
use crate::spiro_kit::{Direction, Family, Kit, Position};
use crate::util::{boole, boole_pn, mix};

/// The design constants these helpers read, from `DesignParameters` in
/// `meta/aesthetics.ptl`. They are fixed for the family rather than derived
/// from the plan.
#[derive(Clone, Copy, Debug)]
pub struct Design {
    pub superness: f64,
    pub a_bar_pos: f64,
    pub a_bar_stroke_adj: f64,
}

impl Default for Design {
    fn default() -> Self {
        Self {
            superness: 2.275,
            a_bar_pos: 0.5304,
            a_bar_stroke_adj: -0.075,
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
    1.0 - (1.0 - px.powf(superness)).powf(1.0 / superness)
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
        1.0 / ((1.0 - (1.0 - v / w).powf(superness)).powf(1.0 / superness) + 1.0)
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
    let s = (angle * std::f64::consts::PI / 180.0).sin();
    crate::util::js_sign(s) * s.abs().powf(2.0 / superness)
}

/// `superCos`.
fn super_cos(angle: f64, superness: f64) -> f64 {
    if angle == 0.0 {
        return 1.0;
    }
    let c = (angle * std::f64::consts::PI / 180.0).cos();
    crate::util::js_sign(c) * c.abs().powf(2.0 / superness)
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
    let r = 1.0 / ((1.0 - (1.0 - v / w).powf(superness)).powf(1.0 / superness) + 1.0);
    0.5 + (r - 0.5) * (v + w) / (u * 2.0)
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
    // to one when no post angle is given.
    if args.angle_pre.is_none() {
        items.push(kit.arcvh(None, None));
    }
    items.push(knot);
    if args.angle_post.is_none() {
        items.push(kit.archv(None, None));
    }
    items
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
