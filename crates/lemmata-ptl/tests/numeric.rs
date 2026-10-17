//! Compares the ported numeric helpers against Iosevka's.
//!
//! `golden/util.json` carries a grid of inputs and the results upstream's
//! helpers produced. Every result is compared exactly, including the halves
//! that `quantize` rounds and the zeroes the waveforms treat specially.

#![allow(clippy::cast_precision_loss)]
// The comparison is exact on purpose: these helpers must reproduce JavaScript
// bit for bit, halves and zeroes included.
#![allow(clippy::float_cmp)]

use std::fs;
use std::path::{Path, PathBuf};

use lemmata_ptl::util::{
    Waveform, bar_mix_l, bez2, bez3, boole, boole_pn, clamp, dist_p, fallback, linreg, mix,
    quantize, sl_x, sl_y, stroke_offset,
};
use serde_json::Value;

fn workspace_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// Runs one case, returning the values it produced.
fn dispatch(name: &str, args: &[Value]) -> Option<Vec<f64>> {
    let n = |index: usize| args[index].as_f64().expect("number");
    let b = |index: usize| args[index].as_bool().expect("boolean");
    let single = |value: f64| Some(vec![value]);
    match name {
        "mix" => single(mix(n(0), n(1), n(2))),
        "barMixL" => single(bar_mix_l(n(0), n(1), n(2), n(3))),
        "linreg" => single(linreg(n(0), n(1), n(2), n(3), n(4))),
        "clamp" => single(clamp(n(0), n(1), n(2))),
        "quantize" => single(quantize(n(0), n(1))),
        "fallback" => {
            let values: Vec<Option<f64>> = args.iter().map(Value::as_f64).collect();
            fallback(&values).map(|value| vec![value])
        }
        "bez2" => single(bez2(n(0), n(1), n(2), n(3))),
        "bez3" => single(bez3(n(0), n(1), n(2), n(3), n(4))),
        "boole" => single(boole(b(0))),
        "boolePn" => single(boole_pn(b(0))),
        "strokeOffset" => {
            let (x, y) = stroke_offset(n(0), n(1), n(2), n(3), n(4), n(5));
            Some(vec![x, y])
        }
        "slY" => single(sl_y(n(0), n(1), n(2), n(3))),
        "slX" => single(sl_x(n(0), n(1), n(2), n(3))),
        "distP" => single(dist_p(n(0), n(1), n(2), n(3), n(4))),
        other => other.strip_prefix("waveform.").map(|kind| {
            let waveform = match kind {
                "sine" => Waveform::Sine,
                "square" => Waveform::Square,
                "triangle" => Waveform::Triangle,
                "sawtooth" => Waveform::Sawtooth,
                other => panic!("unknown waveform {other}"),
            };
            vec![waveform.eval(n(0))]
        }),
    }
}

#[test]
fn util_matches_upstream() {
    let path = workspace_path("golden/util.json");
    let golden: Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("golden util")).unwrap();
    let cases = golden["cases"].as_array().unwrap();

    let mut compared = 0;
    let mut mismatches = Vec::new();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let args = case["args"].as_array().unwrap();
        let expected = &case["result"];
        let ours = dispatch(name, args);
        let theirs: Option<Vec<f64>> = match expected {
            Value::Array(values) => Some(values.iter().map(|v| v.as_f64().unwrap()).collect()),
            Value::Number(value) => Some(vec![value.as_f64().unwrap()]),
            Value::Null => None,
            other => panic!("{name}: unexpected result {other}"),
        };
        compared += 1;
        match (ours, theirs) {
            (Some(ours), Some(theirs)) => {
                assert_eq!(ours.len(), theirs.len(), "{name}: value count");
                for (index, (actual, expected)) in ours.iter().zip(&theirs).enumerate() {
                    if actual != expected {
                        mismatches.push(format!(
                            "{name} {args:?} value {index}: ours {actual}, theirs {expected}"
                        ));
                    }
                }
            }
            (None, None) => {}
            (ours, theirs) => {
                mismatches.push(format!("{name} {args:?}: ours {ours:?}, theirs {theirs:?}"));
            }
        }
    }
    eprintln!("util: {compared} cases compared");
    assert!(
        mismatches.is_empty(),
        "{} of {compared} cases differ:\n  {}",
        mismatches.len(),
        mismatches.join("\n  ")
    );
}
