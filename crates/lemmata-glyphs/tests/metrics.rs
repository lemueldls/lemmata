//! Compares the ported metrics layer against the build's own.
//!
//! The capture writes `<path>.metrics.json`: the plan parameters the build used
//! and the metrics `calculateMetrics` derived from them. Every field is
//! compared. Skips when no capture is present.

#![allow(clippy::cast_precision_loss)]

use std::fs;

mod util;

use lemmata_glyphs::metrics::Metrics;
use serde_json::Value;

use util::dump_path;

/// Metrics go through `tan`, `atanh` and `tanh`, whose implementations differ
/// between V8 and Rust in the last bits.
const TOLERANCE: f64 = 1e-9;

/// Every metric, under the name upstream uses.
fn fields(metrics: &Metrics) -> Vec<(&'static str, f64)> {
    vec![
        ("UPM", metrics.upm),
        ("HalfUPM", metrics.half_upm),
        ("Width", metrics.width),
        ("SB", metrics.sb),
        ("CAP", metrics.cap),
        ("XH", metrics.xh),
        ("Ascender", metrics.ascender),
        ("Descender", metrics.descender),
        ("SymbolMid", metrics.symbol_mid),
        ("ParenTop", metrics.paren_top),
        ("ParenBot", metrics.paren_bot),
        ("OperTop", metrics.oper_top),
        ("OperBot", metrics.oper_bot),
        ("TackTop", metrics.tack_top),
        ("TackBot", metrics.tack_bot),
        ("PlusTop", metrics.plus_top),
        ("PlusBot", metrics.plus_bot),
        ("PictTop", metrics.pict_top),
        ("PictBot", metrics.pict_bot),
        ("BgOpTop", metrics.bg_op_top),
        ("BgOpBot", metrics.bg_op_bot),
        ("BgTkTop", metrics.bg_tk_top),
        ("BgTkBot", metrics.bg_tk_bot),
        ("Contrast", metrics.contrast),
        ("TanSlope", metrics.tan_slope),
        ("HVContrast", metrics.hv_contrast),
        ("VHContrast", metrics.vh_contrast),
        ("O", metrics.o),
        ("OX", metrics.ox),
        ("OXHook", metrics.ox_hook),
        ("Hook", metrics.hook),
        ("AHook", metrics.a_hook),
        ("SHook", metrics.s_hook),
        ("RHook", metrics.r_hook),
        ("JHook", metrics.j_hook),
        ("HookX", metrics.hook_x),
        ("TailX", metrics.tail_x),
        ("TailY", metrics.tail_y),
        ("DiagTailAngle", metrics.diag_tail_angle),
        ("DiagTailDepth", metrics.diag_tail_depth),
        ("ArchDepth", metrics.arch_depth),
        ("SmallArchDepth", metrics.small_arch_depth),
        ("Stroke", metrics.stroke),
        ("HalfStroke", metrics.half_stroke),
        ("QuarterStroke", metrics.quarter_stroke),
        ("DotSize", metrics.dot_size),
        ("PeriodSize", metrics.period_size),
        ("SquareDotScalar", metrics.square_dot_scalar),
        ("HBarPos", metrics.h_bar_pos),
        ("OverlayPos", metrics.overlay_pos),
        ("OverlayStroke", metrics.overlay_stroke),
        ("Jut", metrics.jut),
        ("LongJut", metrics.long_jut),
        ("MidJutSide", metrics.mid_jut_side),
        ("MidJutCenter", metrics.mid_jut_center),
        ("VJut", metrics.v_jut),
        ("LongVJut", metrics.long_v_jut),
        ("VJutStroke", metrics.v_jut_stroke),
        ("AccentStackOffset", metrics.accent_stack_offset),
        ("AccentWidth", metrics.accent_width),
        ("AccentClearance", metrics.accent_clearance),
        ("AccentHeight", metrics.accent_height),
        ("CThin", metrics.c_thin),
        ("CThinB", metrics.c_thin_b),
        ("SLAB", metrics.slab),
        ("JBalance", metrics.j_balance),
        ("JBalance2", metrics.j_balance2),
        ("TBalance", metrics.t_balance),
        ("TBalance2", metrics.t_balance2),
        ("RBalance", metrics.r_balance),
        ("RBalance2", metrics.r_balance2),
        ("FBalance", metrics.f_balance),
        ("OneBalance", metrics.one_balance),
        ("WideWidth0", metrics.wide_width0),
        ("WideWidth1", metrics.wide_width1),
        ("WideWidth2", metrics.wide_width2),
        ("WideWidth3", metrics.wide_width3),
        ("WideWidth4", metrics.wide_width4),
        ("EssRatioUpper", metrics.ess_ratio_upper),
        ("EssRatioLower", metrics.ess_ratio_lower),
        ("EssRatioQuestion", metrics.ess_ratio_question),
        ("EssUpper", metrics.ess_upper),
        ("EssLower", metrics.ess_lower),
        ("EssQuestion", metrics.ess_question),
        ("RightSB", metrics.right_sb),
        ("Middle", metrics.middle),
        ("DotRadius", metrics.dot_radius),
        ("PeriodRadius", metrics.period_radius),
        ("SideJut", metrics.side_jut),
        ("DToothlessRise", metrics.d_toothless_rise),
        ("ShoulderFine", metrics.shoulder_fine),
        ("TightBendInnerRadius", metrics.tight_bend_inner_radius),
        ("SmoothAdjust", metrics.smooth_adjust),
        ("ArchDepthA", metrics.arch_depth_a),
        ("ArchDepthB", metrics.arch_depth_b),
        ("SmallArchDepthA", metrics.small_arch_depth_a),
        ("SmallArchDepthB", metrics.small_arch_depth_b),
        ("CorrectionOMidX", metrics.correction_o_mid_x),
        ("CorrectionOMidS", metrics.correction_o_mid_s),
        ("SerifShiftX", metrics.serif_shift_x),
        ("SerifShiftS", metrics.serif_shift_s),
        ("OperatorStroke", metrics.operator_stroke),
        ("GeometryStroke", metrics.geometry_stroke),
        ("VERY-FAR", metrics.very_far),
        ("TINY", metrics.tiny),
    ]
}

#[test]
fn metrics_match_upstream() {
    let path = dump_path();
    let name = format!("{}.metrics.json", path.display());
    let Ok(text) = fs::read_to_string(&name) else {
        eprintln!("skipping: no metrics at {name}");
        return;
    };
    let value: Value = serde_json::from_str(&text).expect("parse metrics");
    let ours = util::metrics_from_capture().expect("metrics");
    let expected = &value["metrics"];

    let mut worst = (0.0_f64, String::new());
    let mut mismatches = Vec::new();
    let mut compared = 0;
    for (name, actual) in fields(&ours) {
        let Some(theirs) = expected.get(name).and_then(Value::as_f64) else {
            continue;
        };
        compared += 1;
        let deviation = (actual - theirs).abs();
        if deviation > worst.0 {
            worst = (deviation, name.to_string());
        }
        if deviation > TOLERANCE {
            mismatches.push(format!("{name}: ours {actual}, theirs {theirs}"));
        }
    }
    eprintln!(
        "metrics: {compared} fields compared, worst deviation {:.3e} on {}",
        worst.0, worst.1
    );
    assert!(
        mismatches.is_empty(),
        "{} of {compared} metrics differ:\n  {}",
        mismatches.len(),
        mismatches.join("\n  ")
    );
}
