//! Shared readers for the tests that compare against a capture.

use std::fs;
use std::path::{Path, PathBuf};

use lemmata_glyphs::metrics::{Metrics, Para};
use serde_json::Value;

fn workspace_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

pub fn dump_path() -> PathBuf {
    std::env::var_os("LEMMATA_GLYPH_DUMP")
        .map_or_else(|| workspace_path("target/glyphs.bin"), PathBuf::from)
}

/// The metrics dump carries the parameters the build used.
pub fn metrics_from_capture() -> Option<Metrics> {
    let text = fs::read_to_string(format!("{}.metrics.json", dump_path().display())).ok()?;
    let value: Value = serde_json::from_str(&text).expect("parse metrics");
    let para = &value["para"];
    let number = |key: &str| para.get(key).and_then(Value::as_f64);
    let required = |key: &str| number(key).unwrap_or_else(|| panic!("para.{key}"));
    let para = Para {
        width: required("width"),
        sb: required("sb"),
        cap: required("cap"),
        x_height: required("xHeight"),
        ascender: required("ascender"),
        descender: number("descender"),
        symbol_mid: required("symbolMid"),
        paren_size: required("parenSize"),
        oper_size: required("operSize"),
        tack_size: required("tackSize"),
        plus_size: required("plusSize"),
        pict_size: required("pictSize"),
        bgop_size: required("bgopSize"),
        bgtk_size: required("bgtkSize"),
        contrast: number("contrast"),
        slope_angle: required("slopeAngle"),
        is_quasi_proportional: para
            .get("isQuasiProportional")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        spacing: i32::try_from(para.get("spacing").and_then(Value::as_i64).unwrap_or(0))
            .unwrap_or(0),
        overshoot: required("overshoot"),
        overshootx: required("overshootx"),
        oxhook: required("oxhook"),
        hook: required("hook"),
        ahook: required("ahook"),
        shook: required("shook"),
        rhook: required("rhook"),
        jhook: required("jhook"),
        hookx: required("hookx"),
        tail_x: required("tailX"),
        tail_y: required("tailY"),
        diag_tail_angle: required("diagTailAngle"),
        diag_tail_depth: required("diagTailDepth"),
        arch_depth: required("archDepth"),
        small_arch_depth: required("smallArchDepth"),
        stroke: required("stroke"),
        dot_size: number("dotSize"),
        period_size: number("periodSize"),
        jut: required("jut"),
        long_jut: required("longjut"),
        vjut: required("vjut"),
        long_vjut: number("longvjut"),
        accent_stack_offset: required("accentStackOffset"),
        accent_width: required("accentWidth"),
        accent_clearance: required("accentClearance"),
        accent_height: required("accentHeight"),
        cthin: number("cthin"),
        cthinb: number("cthinb"),
        slab: required("slab"),
        jbalance: number("jbalance"),
        jbalance2: number("jbalance2"),
        tbalance: number("tbalance"),
        tbalance2: number("tbalance2"),
        rbalance: number("rbalance"),
        rbalance2: number("rbalance2"),
        fbalance: number("fbalance"),
        onebalance: number("onebalance"),
        ess_ratio: number("essRatio"),
        ess_ratio_upper: number("essRatioUpper"),
        ess_ratio_lower: number("essRatioLower"),
        ess_ratio_question: number("essRatioQuestion"),
        shoulder_fine_min: required("shoulderFineMin"),
        ref_stroke: required("refStroke"),
        canonical_stroke_width_min: required("canonicalStrokeWidthMin"),
        canonical_stroke_width_max: required("canonicalStrokeWidthMax"),
        leading: required("leading"),
        descender_pad: number("descenderPad"),
        win_metric_ascender_pad: number("winMetricAscenderPad"),
        win_metric_descender_pad: number("winMetricDescenderPad"),
    };
    Some(Metrics::calculate(&para))
}
