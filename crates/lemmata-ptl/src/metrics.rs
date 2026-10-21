//! The metrics layer: plan parameters in, derived metrics out.
//!
//! Ported from `meta/aesthetics.ptl`'s `calculateMetrics` and
//! `font-kits/weight-control.mjs`. Every glyph definition reads this object:
//! the frame, the stroke width, the arch depths, the overshoots and the rest.
//!
//! The port takes the plan parameters as a plain struct. Upstream derives them
//! from the build plan through the `param` package, which is a separate layer.

#![allow(
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::suboptimal_flops,
    clippy::too_many_arguments
)]
use lemmata_geom::math;

/// The plan parameters `calculateMetrics` reads.
#[derive(Clone, Debug, Default)]
pub struct Para {
    pub width: f64,
    pub sb: f64,
    pub cap: f64,
    pub x_height: f64,
    pub ascender: f64,
    pub descender: Option<f64>,
    pub symbol_mid: f64,
    pub paren_size: f64,
    pub oper_size: f64,
    pub tack_size: f64,
    pub plus_size: f64,
    pub pict_size: f64,
    pub bgop_size: f64,
    pub bgtk_size: f64,
    pub contrast: Option<f64>,
    pub slope_angle: f64,
    pub is_quasi_proportional: bool,
    pub spacing: i32,
    pub overshoot: f64,
    pub overshootx: f64,
    pub oxhook: f64,
    pub hook: f64,
    pub ahook: f64,
    pub shook: f64,
    pub rhook: f64,
    pub jhook: f64,
    pub hookx: f64,
    pub tail_x: f64,
    pub tail_y: f64,
    pub diag_tail_angle: f64,
    pub diag_tail_depth: f64,
    pub unit_width_scalar: f64,
    pub arch_depth: f64,
    pub small_arch_depth: f64,
    pub stroke: f64,
    pub dot_size: Option<f64>,
    pub period_size: Option<f64>,
    pub jut: f64,
    pub long_jut: f64,
    pub vjut: f64,
    pub long_vjut: Option<f64>,
    pub accent_stack_offset: f64,
    pub accent_width: f64,
    pub accent_clearance: f64,
    pub accent_height: f64,
    pub cthin: Option<f64>,
    pub cthinb: Option<f64>,
    pub slab: f64,
    pub jbalance: Option<f64>,
    pub jbalance2: Option<f64>,
    pub tbalance: Option<f64>,
    pub tbalance2: Option<f64>,
    pub rbalance: Option<f64>,
    pub rbalance2: Option<f64>,
    pub fbalance: Option<f64>,
    pub onebalance: Option<f64>,
    pub ess_ratio: Option<f64>,
    pub ess_ratio_upper: Option<f64>,
    pub ess_ratio_lower: Option<f64>,
    pub ess_ratio_question: Option<f64>,
    pub shoulder_fine_min: f64,
    pub ref_stroke: f64,
    /// The reference jut a full serif is measured against.
    pub ref_jut: f64,
    pub canonical_stroke_width_min: f64,
    pub canonical_stroke_width_max: f64,
    pub leading: f64,
    pub descender_pad: Option<f64>,
    pub win_metric_ascender_pad: Option<f64>,
    pub win_metric_descender_pad: Option<f64>,
}

/// The constants `DesignParameters` carries.
pub mod design_parameters {
    pub const A_BAR_POS: f64 = 0.5304;
    pub const H_BAR_POS: f64 = 0.525;
    pub const OVERLAY_POS: f64 = 0.52;
    pub const A_BAR_STROKE_ADJ: f64 = -0.075;
    pub const SUPERNESS: f64 = 2.275;
    pub const SERIF_SHIFT_X: f64 = 0.60;
    pub const SQUARE_DOT_SCALAR: f64 = 0.95;
}

/// The derived metrics, in the same names upstream uses.
#[derive(Clone, Debug, Default)]
pub struct Metrics {
    pub upm: f64,
    pub half_upm: f64,
    pub width: f64,
    pub sb: f64,
    pub cap: f64,
    pub xh: f64,
    pub ascender: f64,
    pub descender: f64,
    pub symbol_mid: f64,
    pub paren_top: f64,
    pub paren_bot: f64,
    pub oper_top: f64,
    pub oper_bot: f64,
    pub tack_top: f64,
    pub tack_bot: f64,
    pub plus_top: f64,
    pub plus_bot: f64,
    pub pict_top: f64,
    pub pict_bot: f64,
    pub bg_op_top: f64,
    pub bg_op_bot: f64,
    pub bg_tk_top: f64,
    pub bg_tk_bot: f64,
    pub contrast: f64,
    pub tan_slope: f64,
    pub hv_contrast: f64,
    pub vh_contrast: f64,
    pub o: f64,
    pub ox: f64,
    pub ox_hook: f64,
    pub hook: f64,
    pub a_hook: f64,
    pub s_hook: f64,
    pub r_hook: f64,
    pub j_hook: f64,
    pub hook_x: f64,
    pub tail_x: f64,
    pub tail_y: f64,
    pub diag_tail_angle: f64,
    pub diag_tail_depth: f64,
    pub unit_width_scalar: f64,
    pub arch_depth: f64,
    pub small_arch_depth: f64,
    pub stroke: f64,
    pub half_stroke: f64,
    pub quarter_stroke: f64,
    pub dot_size: f64,
    pub period_size: f64,
    pub square_dot_scalar: f64,
    pub h_bar_pos: f64,
    pub overlay_pos: f64,
    pub overlay_stroke: f64,
    pub jut: f64,
    pub long_jut: f64,
    pub mid_jut_side: f64,
    pub mid_jut_center: f64,
    pub v_jut: f64,
    pub long_v_jut: f64,
    pub v_jut_stroke: f64,
    pub accent_stack_offset: f64,
    pub accent_width: f64,
    pub accent_clearance: f64,
    pub accent_height: f64,
    pub c_thin: f64,
    pub c_thin_b: f64,
    pub slab: f64,
    pub j_balance: f64,
    pub j_balance2: f64,
    pub t_balance: f64,
    pub t_balance2: f64,
    pub r_balance: f64,
    pub r_balance2: f64,
    pub f_balance: f64,
    pub one_balance: f64,
    pub wide_width0: f64,
    pub wide_width1: f64,
    pub wide_width2: f64,
    pub wide_width3: f64,
    pub wide_width4: f64,
    pub ess_ratio_upper: f64,
    pub ess_ratio_lower: f64,
    pub ess_ratio_question: f64,
    pub ess_upper: f64,
    pub ess_lower: f64,
    pub ess_question: f64,
    pub right_sb: f64,
    pub middle: f64,
    pub dot_radius: f64,
    pub period_radius: f64,
    pub side_jut: f64,
    pub d_toothless_rise: f64,
    pub shoulder_fine: f64,
    pub tight_bend_inner_radius: f64,
    pub smooth_adjust: f64,
    pub arch_depth_a: f64,
    pub arch_depth_b: f64,
    pub small_arch_depth_a: f64,
    pub small_arch_depth_b: f64,
    pub correction_o_mid_x: f64,
    pub correction_o_mid_s: f64,
    pub serif_shift_x: f64,
    pub serif_shift_s: f64,
    pub operator_stroke: f64,
    pub geometry_stroke: f64,
    pub very_far: f64,
    pub tiny: f64,
    /// Carried from the plan so shape code can call `stroke_width_blend`.
    pub ref_stroke: f64,
    /// The reference jut a full serif is measured against.
    pub ref_jut: f64,
    pub canonical_stroke_width_min: f64,
    pub canonical_stroke_width_max: f64,
}

/// The stroke-width advice from `weight-control.mjs`.
#[derive(Clone, Copy, Debug)]
pub struct WeightControl {
    stroke: f64,
    width: f64,
    sb: f64,
    hv_contrast: f64,
    factor: f64,
}

impl WeightControl {
    #[must_use]
    pub fn new(stroke: f64, width: f64, sb: f64, hv_contrast: f64) -> Self {
        let base_fill_rate = (0.95_f64).min((2.0 * stroke * hv_contrast) / (width - sb * 2.0));
        // Through `libm` rather than the platform: the host and wasm32 have
        // different `tanh` and `atanh` in the last bit, and a stroke width that
        // moves by one unit in the last place moves the outline enough to
        // change how many points the quadifier emits. Byte-identical output
        // between the CLI and the browser needs one implementation, and this is
        // it. `r[wasm.parity-bytes]`
        let factor = math::atanh(base_fill_rate) / base_fill_rate;

        Self {
            stroke,
            width,
            sb,
            hv_contrast,
            factor,
        }
    }

    fn sigmoid(&self, x: f64) -> f64 {
        math::tanh(x * self.factor)
    }

    #[must_use]
    pub fn advice_stroke_in_space(
        &self,
        avail_space: f64,
        crowdedness: f64,
        contrast: f64,
        mul: f64,
    ) -> f64 {
        let non_adjusted = (crowdedness * contrast * self.stroke) / avail_space;
        let adjusted = self.sigmoid(non_adjusted);
        let scalar = (mul * (adjusted / non_adjusted)).min(1.0);

        self.stroke * scalar
    }

    /// `AdviceStroke(crowdedness, adws, mul)`.
    #[must_use]
    pub fn advice_stroke(&self, crowdedness: f64, adws: f64, mul: f64) -> f64 {
        let space = self.width * adws - self.sb * 2.0;
        self.advice_stroke_in_space(space, crowdedness, self.hv_contrast, mul)
    }

    /// `AdviceStroke2(crwX, crwY, refH, adws)`.
    #[must_use]
    pub fn advice_stroke2(&self, crw_x: f64, crw_y: f64, ref_h: f64, adws: f64) -> f64 {
        let space_h = self.width * adws - self.sb * 2.0;
        self.advice_stroke_in_space(space_h, crw_x, self.hv_contrast, 1.0)
            .min(self.advice_stroke_in_space(ref_h, crw_y, 1.0, 1.0))
    }
}

impl Para {
    /// Builds a `Para` from the named parameters, in Iosevka's names.
    ///
    /// The names are the ones the parameter layer produces and the capture
    /// records, so the same mapping reads either. A key the caller cannot supply
    /// is an error rather than a zero: a missing `stroke` would divide by zero
    /// before anyone noticed.
    ///
    /// # Errors
    ///
    /// Returns the name of the first missing key.
    #[allow(clippy::cast_possible_truncation)]
    pub fn from_named(
        get: impl Fn(&str) -> Option<f64>,
        flag: impl Fn(&str) -> Option<bool>,
    ) -> Result<Self, String> {
        Ok(Para {
            width: require(&get, "width")?,
            sb: require(&get, "sb")?,
            cap: require(&get, "cap")?,
            x_height: require(&get, "xHeight")?,
            ascender: require(&get, "ascender")?,
            descender: get("descender"),
            symbol_mid: require(&get, "symbolMid")?,
            paren_size: require(&get, "parenSize")?,
            oper_size: require(&get, "operSize")?,
            tack_size: require(&get, "tackSize")?,
            plus_size: require(&get, "plusSize")?,
            pict_size: require(&get, "pictSize")?,
            bgop_size: require(&get, "bgopSize")?,
            bgtk_size: require(&get, "bgtkSize")?,
            contrast: get("contrast"),
            slope_angle: require(&get, "slopeAngle")?,
            is_quasi_proportional: flag("isQuasiProportional").unwrap_or(false),
            spacing: get("spacing").map_or(0, |value| value as i32),
            overshoot: require(&get, "overshoot")?,
            overshootx: require(&get, "overshootx")?,
            oxhook: require(&get, "oxhook")?,
            hook: require(&get, "hook")?,
            ahook: require(&get, "ahook")?,
            shook: require(&get, "shook")?,
            rhook: require(&get, "rhook")?,
            jhook: require(&get, "jhook")?,
            hookx: require(&get, "hookx")?,
            tail_x: require(&get, "tailX")?,
            tail_y: require(&get, "tailY")?,
            diag_tail_angle: require(&get, "diagTailAngle")?,
            diag_tail_depth: require(&get, "diagTailDepth")?,
            unit_width_scalar: require(&get, "unitWidthScalar")?,
            arch_depth: require(&get, "archDepth")?,
            small_arch_depth: require(&get, "smallArchDepth")?,
            stroke: require(&get, "stroke")?,
            dot_size: get("dotSize"),
            period_size: get("periodSize"),
            jut: require(&get, "jut")?,
            long_jut: require(&get, "longjut")?,
            vjut: require(&get, "vjut")?,
            long_vjut: get("longvjut"),
            accent_stack_offset: require(&get, "accentStackOffset")?,
            accent_width: require(&get, "accentWidth")?,
            accent_clearance: require(&get, "accentClearance")?,
            accent_height: require(&get, "accentHeight")?,
            cthin: get("cthin"),
            cthinb: get("cthinb"),
            slab: require(&get, "slab")?,
            jbalance: get("jbalance"),
            jbalance2: get("jbalance2"),
            tbalance: get("tbalance"),
            tbalance2: get("tbalance2"),
            rbalance: get("rbalance"),
            rbalance2: get("rbalance2"),
            fbalance: get("fbalance"),
            onebalance: get("onebalance"),
            ess_ratio: get("essRatio"),
            ess_ratio_upper: get("essRatioUpper"),
            ess_ratio_lower: get("essRatioLower"),
            ess_ratio_question: get("essRatioQuestion"),
            shoulder_fine_min: require(&get, "shoulderFineMin")?,
            ref_stroke: require(&get, "refStroke")?,
            ref_jut: require(&get, "refJut")?,
            canonical_stroke_width_min: require(&get, "canonicalStrokeWidthMin")?,
            canonical_stroke_width_max: require(&get, "canonicalStrokeWidthMax")?,
            leading: require(&get, "leading")?,
            descender_pad: get("descenderPad"),
            win_metric_ascender_pad: get("winMetricAscenderPad"),
            win_metric_descender_pad: get("winMetricDescenderPad"),
        })
    }
}

fn require(get: &impl Fn(&str) -> Option<f64>, key: &str) -> Result<f64, String> {
    get(key).ok_or_else(|| format!("the parameter {key} is missing"))
}

impl Metrics {
    /// `calculateMetrics`.
    #[must_use]
    pub fn calculate(para: &Para) -> Self {
        use design_parameters as dp;
        let upm = 1000.0;
        let half_upm = upm / 2.0;
        let width = para.width.round();
        let sb = para.sb.round();
        let cap = para.cap.round();
        let xh = para.x_height.round();
        let ascender = para.ascender.round();
        let descender = para.descender.unwrap_or(xh - ascender).round();

        let symbol_mid = para.symbol_mid.round();
        let half_paren = (para.paren_size / 2.0).ceil();
        let symbol_span = width - sb * 2.0;

        let contrast = para.contrast.unwrap_or(1.0);
        let slope = math::tan(para.slope_angle / 180.0 * std::f64::consts::PI);
        let sin_slope = math::sin(para.slope_angle / 180.0 * std::f64::consts::PI);
        let cos_slope = math::cos(para.slope_angle / 180.0 * std::f64::consts::PI);
        let tan_slope = slope;
        let hv_contrast = contrast * cos_slope + sin_slope * tan_slope;
        let vh_contrast = 1.0 / hv_contrast;
        let hsw_to_v = |sw: f64| sw * hv_contrast;

        let weight = WeightControl::new(para.stroke, width, sb, hv_contrast);
        let stroke = weight.advice_stroke(2.0, 1.0, 1.0);
        let half_stroke = stroke / 2.0;
        let quarter_stroke = stroke / 4.0;

        let dot_size = para.dot_size.unwrap_or(stroke);
        let period_size = para.period_size.unwrap_or(dot_size);
        let h_bar_pos = dp::H_BAR_POS - 0.09 * stroke / cap;
        let overlay_stroke = weight.advice_stroke(3.75, 1.0, 1.0);

        let jut = para.jut;
        let long_jut = para.long_jut;
        let mid_jut_side = jut.max(mix(hsw_to_v(half_stroke), long_jut, 0.5));
        let mid_jut_center = jut.max(mix(hsw_to_v(half_stroke), long_jut, 0.6));
        let long_v_jut = para.long_vjut.unwrap_or(long_jut);

        let j_balance = para.jbalance.unwrap_or(0.0);
        let j_balance2 = para.jbalance2.unwrap_or(long_jut * 0.04 + quarter_stroke);
        let t_balance = para.tbalance.unwrap_or(j_balance);
        let t_balance2 = para.tbalance2.unwrap_or(t_balance);
        let r_balance = para.rbalance.unwrap_or(j_balance * 0.3);
        let r_balance2 = para.rbalance2.unwrap_or(0.0);
        let f_balance = para.fbalance.unwrap_or(0.0);
        let one_balance = para.onebalance.unwrap_or(0.0);

        let wide_width0 = if para.is_quasi_proportional {
            upm
        } else {
            width * 2.0
        };
        let wide = |level: i32| {
            if para.spacing >= level {
                wide_width0
            } else {
                width
            }
        };

        let ess_ratio_upper = para.ess_ratio_upper.or(para.ess_ratio).unwrap_or(contrast);
        let ess_ratio_lower = para.ess_ratio_lower.or(para.ess_ratio).unwrap_or(contrast);
        let ess_ratio_question = para
            .ess_ratio_question
            .or(para.ess_ratio)
            .unwrap_or(contrast);

        let right_sb = width - sb;
        let middle = width / 2.0;
        let side_jut = jut - hsw_to_v(half_stroke);
        let d_toothless_rise = para.hook / 4.0 + stroke / 16.0;
        let shoulder_fine =
            (stroke * para.shoulder_fine_min).min(weight.advice_stroke(24.0, 1.0, 1.0));
        let tight_bend_inner_diameter = (xh / 32.0).max(weight.advice_stroke2(24.0, 32.0, xh, 1.0));

        let stroke_width_blend = |lo: f64, hi: f64, sw: Option<f64>| {
            let sw_ref = (sw.unwrap_or(stroke) / stroke) * para.ref_stroke;
            linreg(
                para.canonical_stroke_width_min,
                lo,
                para.canonical_stroke_width_max,
                hi,
                sw_ref,
            )
        };
        let smooth_adjust = stroke_width_blend(80.0, 144.0, None);
        let arch_depth_a = arch_depth_a_of(para.arch_depth, smooth_adjust, tan_slope, width, None);
        let arch_depth_b = arch_depth_b_of(para.arch_depth, smooth_adjust, tan_slope, width, None);

        Self {
            upm,
            half_upm,
            width,
            sb,
            cap,
            xh,
            ascender,
            descender,
            symbol_mid,
            paren_top: symbol_mid + half_paren,
            paren_bot: symbol_mid - half_paren,
            oper_top: symbol_mid + para.oper_size * symbol_span,
            oper_bot: symbol_mid - para.oper_size * symbol_span,
            tack_top: symbol_mid + para.tack_size * symbol_span,
            tack_bot: symbol_mid - para.tack_size * symbol_span,
            plus_top: symbol_mid + para.plus_size * symbol_span,
            plus_bot: symbol_mid - para.plus_size * symbol_span,
            pict_top: symbol_mid + para.pict_size * symbol_span,
            pict_bot: symbol_mid - para.pict_size * symbol_span,
            bg_op_top: symbol_mid + para.bgop_size * symbol_span,
            bg_op_bot: symbol_mid - para.bgop_size * symbol_span,
            bg_tk_top: symbol_mid + para.bgtk_size * symbol_span,
            bg_tk_bot: symbol_mid - para.bgtk_size * symbol_span,
            contrast,
            tan_slope,
            hv_contrast,
            vh_contrast,
            o: para.overshoot,
            ox: para.overshootx,
            ox_hook: para.oxhook,
            hook: para.hook,
            a_hook: para.ahook,
            s_hook: para.shook,
            r_hook: para.rhook,
            j_hook: para.jhook,
            hook_x: para.hookx,
            tail_x: para.tail_x,
            tail_y: para.tail_y,
            diag_tail_angle: para.diag_tail_angle,
            diag_tail_depth: para.diag_tail_depth,
            unit_width_scalar: para.unit_width_scalar,
            arch_depth: para.arch_depth,
            small_arch_depth: para.small_arch_depth,
            stroke,
            half_stroke,
            quarter_stroke,
            dot_size,
            period_size,
            square_dot_scalar: dp::SQUARE_DOT_SCALAR,
            h_bar_pos,
            overlay_pos: dp::OVERLAY_POS,
            overlay_stroke,
            jut,
            long_jut,
            mid_jut_side,
            mid_jut_center,
            v_jut: para.vjut,
            long_v_jut,
            v_jut_stroke: weight.advice_stroke(3.5, 1.0, 1.0),
            accent_stack_offset: para.accent_stack_offset,
            accent_width: para.accent_width,
            accent_clearance: para.accent_clearance,
            accent_height: para.accent_height,
            c_thin: para.cthin.unwrap_or(0.75),
            c_thin_b: para.cthinb.unwrap_or(0.50),
            slab: para.slab,
            j_balance,
            j_balance2,
            t_balance,
            t_balance2,
            r_balance,
            r_balance2,
            f_balance,
            one_balance,
            wide_width0,
            wide_width1: wide(1),
            wide_width2: wide(2),
            wide_width3: wide(3),
            wide_width4: wide(4),
            ess_ratio_upper,
            ess_ratio_lower,
            ess_ratio_question,
            ess_upper: stroke * ess_ratio_upper,
            ess_lower: stroke * ess_ratio_lower,
            ess_question: stroke * ess_ratio_question,
            right_sb,
            middle,
            dot_radius: dot_size / 2.0,
            period_radius: period_size / 2.0,
            side_jut,
            d_toothless_rise,
            shoulder_fine,
            tight_bend_inner_radius: tight_bend_inner_diameter / 2.0,
            smooth_adjust,
            arch_depth_a,
            arch_depth_b,
            small_arch_depth_a: arch_depth_a_of(
                para.small_arch_depth,
                smooth_adjust,
                tan_slope,
                width,
                None,
            ),
            small_arch_depth_b: arch_depth_b_of(
                para.small_arch_depth,
                smooth_adjust,
                tan_slope,
                width,
                None,
            ),
            correction_o_mid_x: tan_slope * 0.9 * stroke_width_blend(1.3, 0.9, None),
            correction_o_mid_s: stroke * (tan_slope * 0.9 * stroke_width_blend(1.3, 0.9, None)),
            serif_shift_x: tan_slope * dp::SERIF_SHIFT_X,
            serif_shift_s: stroke * (tan_slope * dp::SERIF_SHIFT_X),
            operator_stroke: weight.advice_stroke(2.75, 1.0, 1.0),
            geometry_stroke: weight.advice_stroke(4.0, 1.0, 1.0),
            very_far: upm * 16.0,
            ref_stroke: para.ref_stroke,
            ref_jut: para.ref_jut,
            canonical_stroke_width_min: para.canonical_stroke_width_min,
            canonical_stroke_width_max: para.canonical_stroke_width_max,
            tiny: 1.0 / 128.0,
        }
    }
}

/// `ArchDepthAOf`, as a free function so `calculate` can use it before the
/// struct exists.
fn arch_depth_a_of(
    arch_depth: f64,
    smooth_adjust: f64,
    tan_slope: f64,
    width: f64,
    width_opt: Option<f64>,
) -> f64 {
    arch_depth - tan_slope * smooth_adjust * (width_opt.unwrap_or(width) / width)
}

fn arch_depth_b_of(
    arch_depth: f64,
    smooth_adjust: f64,
    tan_slope: f64,
    width: f64,
    width_opt: Option<f64>,
) -> f64 {
    arch_depth + tan_slope * smooth_adjust * (width_opt.unwrap_or(width) / width)
}

impl Metrics {
    /// `AdviceStroke(crowdedness, adws, mul)`.
    #[must_use]
    pub fn advice_stroke(&self, crowdedness: f64, adws: f64, mul: f64) -> f64 {
        WeightControl::new(self.stroke, self.width, self.sb, self.hv_contrast).advice_stroke(
            crowdedness,
            adws,
            mul,
        )
    }

    /// `AdviceStroke2(crwX, crwY, refH, adws)`.
    #[must_use]
    pub fn advice_stroke2(&self, crw_x: f64, crw_y: f64, ref_h: f64, adws: f64) -> f64 {
        WeightControl::new(self.stroke, self.width, self.sb, self.hv_contrast)
            .advice_stroke2(crw_x, crw_y, ref_h, adws)
    }

    /// `ArchDepthAOf`.
    #[must_use]
    pub fn arch_depth_a_of(&self, arch_depth: f64, width: Option<f64>) -> f64 {
        arch_depth_a_of(
            arch_depth,
            self.smooth_adjust,
            self.tan_slope,
            self.width,
            width,
        )
    }

    /// `StrokeWidthBlend`: a value interpolated across the canonical stroke
    /// range, scaled by the stroke relative to the reference.
    #[must_use]
    pub fn stroke_width_blend(&self, lo: f64, hi: f64, sw: Option<f64>) -> f64 {
        let sw_ref = (sw.unwrap_or(self.stroke) / self.stroke) * self.ref_stroke;
        crate::util::linreg(
            self.canonical_stroke_width_min,
            lo,
            self.canonical_stroke_width_max,
            hi,
            sw_ref,
        )
    }

    /// `ArchDepthBOf`.
    #[must_use]
    pub fn arch_depth_b_of(&self, arch_depth: f64, width: Option<f64>) -> f64 {
        arch_depth_b_of(
            arch_depth,
            self.smooth_adjust,
            self.tan_slope,
            self.width,
            width,
        )
    }
}

/// `linreg(a, b, c, d, e)`.
fn linreg(a: f64, b: f64, c: f64, d: f64, e: f64) -> f64 {
    b + (d - b) * (e - a) / (c - a)
}

fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
