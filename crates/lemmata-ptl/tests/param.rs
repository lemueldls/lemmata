//! Runs `param` for the reference shape and compares every key against the
//! parameters the pinned build derived. Skips without a capture.

use std::collections::BTreeMap;

use lemmata_ptl::param::{Shape, init, load_data, toml::Value};

const PARAMETERS: &str = include_str!("../../../vendor/Iosevka/params/parameters.toml");
const WEIGHTS: &str = include_str!("../../../vendor/Iosevka/params/shape-weight.toml");
const WIDTHS: &str = include_str!("../../../vendor/Iosevka/params/shape-width.toml");
const SLOPES: &str = include_str!("../../../vendor/Iosevka/params/shape-slope.toml");

fn data() -> BTreeMap<String, Value> {
    load_data(PARAMETERS, WEIGHTS, WIDTHS, SLOPES).expect("load the parameter data")
}

fn reference_shape() -> Shape {
    Shape {
        serifs: Some("sans".to_string()),
        spacing: Some("normal".to_string()),
        weight: 400.0,
        width: 500.0,
        slope_angle: 0.0,
        slope: "Upright".to_string(),
    }
}

#[test]
fn reads_the_upstream_files() {
    let data = data();
    // The three shapes the walk depends on, found in the real files.
    assert!(data.contains_key("iosevka"));
    assert!(data.contains_key("shapeWeight"));
    assert!(data.contains_key("shapeWidth"));
    assert!(data.contains_key("shapeSlopeAngle"));

    let hive = init(&data, &reference_shape());
    // Straight from `shape-weight.toml`'s 400 grade and `parameters.toml`.
    assert_eq!(hive["stroke"].as_f64(), Some(70.0));
    assert_eq!(hive["cap"].as_f64(), Some(735.0));
    assert_eq!(hive["xHeight"].as_f64(), Some(520.0));
    assert_eq!(hive["width"].as_f64(), Some(500.0));
    // A width multiplies rather than blends.
    assert_eq!(hive["sb"].as_f64(), Some(64.0));
}

#[test]
fn matches_the_parameters_the_build_derived() {
    let Ok(reference) = std::panic::catch_unwind(|| lemmata_capture::read_side_data("metrics"))
    else {
        eprintln!("skipping: no capture");
        return;
    };
    let expected = reference["para"].as_object().expect("para");

    let hive = init(&data(), &reference_shape());
    let mut missing = Vec::new();
    let mut wrong = Vec::new();
    for (key, wanted) in expected {
        let Some(got) = hive.get(key) else {
            missing.push(key.clone());
            continue;
        };
        let same = match wanted {
            serde_json::Value::Number(number) => got
                .as_f64()
                .is_some_and(|value| (value - number.as_f64().unwrap()).abs() < 1e-9),
            serde_json::Value::String(text) => got.as_str() == Some(text.as_str()),
            serde_json::Value::Bool(flag) => got.as_bool() == Some(*flag),
            other => {
                wrong.push(format!("{key}: cannot compare {other}"));
                continue;
            }
        };
        if !same {
            wrong.push(format!("{key}: got {got:?}, wanted {wanted}"));
        }
    }
    eprintln!(
        "param: {} keys, {} missing, {} wrong",
        expected.len(),
        missing.len(),
        wrong.len()
    );
    for message in wrong.iter().take(8) {
        eprintln!("  {message}");
    }
    if !missing.is_empty() {
        eprintln!("  missing: {:?}", &missing[..missing.len().min(12)]);
    }
    assert!(
        wrong.is_empty() && missing.is_empty(),
        "see the report above"
    );
}
