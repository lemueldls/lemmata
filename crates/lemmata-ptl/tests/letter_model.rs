//! Compares the ported per-glyph frame against the build's own.
//!
//! The capture writes `<path>.frames.json`: the frames the build built at
//! several divisions, plus the values they derive. Skips without a capture.

#![allow(clippy::cast_precision_loss)]

use std::fs;

use lemmata_capture::{dump_path, metrics_from_capture};
use lemmata_ptl::{letter_model::DivFrame, metrics::Metrics};
use serde_json::Value;

/// `atanh` and `tanh` differ between V8 and Rust in the last bits.
const TOLERANCE: f64 = 1e-9;

/// The frames the capture built, in the same order.
fn build_frames(metrics: &Metrics) -> Vec<(String, DivFrame)> {
    let make = |adws: Option<f64>, h_pack: Option<f64>, sb_mul: Option<f64>| {
        let name =
            |value: Option<f64>| value.map_or_else(|| "undefined".to_string(), |v| v.to_string());
        (
            format!("DivFrame({},{},{})", name(adws), name(h_pack), name(sb_mul)),
            DivFrame::new(metrics.clone(), adws, h_pack, sb_mul),
        )
    };
    let mut frames = vec![
        make(Some(1.0), None, None),
        make(Some(1.0), Some(2.0), None),
        make(Some(2.0), None, None),
        make(Some(1.0), Some(3.0), None),
        make(Some(1.0), None, Some(0.5)),
        make(Some(0.5), Some(1.0), None),
    ];
    let base = DivFrame::new(metrics.clone(), Some(1.0), None, None);
    frames.push((
        "slice(3,2)".to_string(),
        base.slice(Some(3.0), Some(2.0), None),
    ));
    frames.push((
        "slice(4,3)".to_string(),
        base.slice(Some(4.0), Some(3.0), None),
    ));
    frames.push((
        "sliceFine(3,2,0.5)".to_string(),
        base.slice_fine(3.0, 2.0, 0.5, 0.0, 0.0),
    ));
    let sub = base.slice(Some(3.0), Some(2.0), None);
    frames.push(("rest(slice(3,2))".to_string(), base.rest(&sub, None)));
    frames.push((
        "restCompact(0.1,slice(3,2))".to_string(),
        base.rest_compact(0.1, &sub, None),
    ));

    frames
}

/// Every value a frame exposes, under the name upstream uses.
fn fields(frame: &DivFrame) -> Vec<(&'static str, f64)> {
    vec![
        ("hPack", frame.h_pack),
        ("width", frame.width),
        ("sb", frame.sb),
        ("leftSB", frame.left_sb()),
        ("rightSB", frame.right_sb()),
        ("middle", frame.middle()),
        ("mvs", frame.mvs),
        ("shoulderFine", frame.shoulder_fine),
        ("ox", frame.ox),
        ("archDepth", frame.arch_depth),
        ("archDepthA", frame.arch_depth_a),
        ("archDepthB", frame.arch_depth_b),
        ("smallArchDepth", frame.small_arch_depth),
        ("smallArchDepthA", frame.small_arch_depth_a),
        ("smallArchDepthB", frame.small_arch_depth_b),
        ("adws", frame.adws()),
        ("widthNoOvershoot", frame.width_no_overshoot()),
        ("adwsNoOvershoot", frame.adws_no_overshoot()),
    ]
}

#[test]
fn frames_match_upstream() {
    let path = dump_path();
    let name = format!("{}.frames.json", path.display());
    let Ok(text) = fs::read_to_string(&name) else {
        eprintln!("skipping: no frames at {name}");
        return;
    };
    let Some(metrics) = metrics_from_capture() else {
        eprintln!("skipping: no metrics to build frames from");
        return;
    };
    let value: Value = serde_json::from_str(&text).expect("parse frames");
    let expected_frames = value["frames"].as_array().unwrap();
    let ours = build_frames(&metrics);
    assert_eq!(ours.len(), expected_frames.len(), "frame count");

    let xh = metrics.xh;
    let arch_depth = metrics.arch_depth;
    let mut worst = (0.0_f64, String::new());
    let mut mismatches = Vec::new();
    let mut compared = 0;
    for ((label, frame), expected) in ours.iter().zip(expected_frames) {
        assert_eq!(label, expected["label"].as_str().unwrap(), "frame order");
        for (name, actual) in fields(frame) {
            let Some(theirs) = expected["fields"].get(name).and_then(Value::as_f64) else {
                continue;
            };
            compared += 1;
            let deviation = (actual - theirs).abs();
            if deviation > worst.0 {
                worst = (deviation, format!("{label} {name}"));
            }

            if deviation > TOLERANCE {
                mismatches.push(format!("{label} {name}: ours {actual}, theirs {theirs}"));
            }
        }

        for (name, actual) in [
            ("adviceStroke", frame.advice_stroke(2.0, 1.0)),
            ("adviceThinnerStroke", frame.advice_thinner_stroke(2.0, 1.0)),
            ("adviceStroke2", frame.advice_stroke2(2.0, 3.0, xh, 1.0)),
            (
                "adviceThinnerStroke2",
                frame.advice_thinner_stroke2(2.0, 3.0, xh, 1.0),
            ),
            ("archDepthOf", frame.arch_depth_of(arch_depth, None)),
            ("archDepthAOf", frame.arch_depth_a_of(arch_depth, None)),
            ("archDepthBOf", frame.arch_depth_b_of(arch_depth, None)),
        ] {
            let theirs = expected[name].as_f64().unwrap();
            compared += 1;
            let deviation = (actual - theirs).abs();
            if deviation > worst.0 {
                worst = (deviation, format!("{label} {name}"));
            }

            if deviation > TOLERANCE {
                mismatches.push(format!("{label} {name}: ours {actual}, theirs {theirs}"));
            }
        }
    }
    eprintln!(
        "frames: {} frames, {compared} values, worst deviation {:.3e} on {}",
        ours.len(),
        worst.0,
        worst.1
    );
    assert!(
        mismatches.is_empty(),
        "{} of {compared} frame values differ:\n  {}",
        mismatches.len(),
        mismatches.join("\n  ")
    );
}
