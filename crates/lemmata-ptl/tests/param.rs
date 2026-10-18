//! Runs `param` for the reference shape and compares every key against the
//! parameters the pinned build derived. Skips without a capture.

use std::collections::BTreeMap;

use lemmata_ptl::param::{Shape, init, load_data, metric_override, toml::Value};

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

#[test]
fn metric_overrides_read_expressions() {
    let data = data();
    let shape = reference_shape();
    let mut overrides = BTreeMap::new();
    overrides.insert(
        "cap".to_string(),
        Value::Str("700 + weight / 4".to_string()),
    );
    overrides.insert("xHeight".to_string(), Value::Int(545));
    overrides.insert(
        "symbolMid".to_string(),
        Value::Str("blend(weight, [400, 340], [700, 360])".to_string()),
    );
    overrides.insert("notAField".to_string(), Value::Float(1.0));

    let mut hive = init(&data, &shape);
    let skipped = metric_override::apply(&mut hive, &overrides, &shape).expect("apply");

    // 700 + 400 / 4
    assert_eq!(hive["cap"].as_f64(), Some(800.0));
    // A number is taken as it stands.
    assert_eq!(hive["xHeight"].as_f64(), Some(545.0));
    // A control point of the blend, so the interpolant is not involved.
    assert_eq!(hive["symbolMid"].as_f64(), Some(340.0));
    // Upstream reports the field and carries on rather than failing the build.
    assert_eq!(skipped, vec!["notAField".to_string()]);
}

#[test]
fn metric_overrides_report_bad_expressions() {
    let data = data();
    let shape = reference_shape();
    let mut overrides = BTreeMap::new();
    overrides.insert("cap".to_string(), Value::Str("700 +".to_string()));
    let mut hive = init(&data, &shape);
    assert!(metric_override::apply(&mut hive, &overrides, &shape).is_err());

    let mut overrides = BTreeMap::new();
    overrides.insert("cap".to_string(), Value::Str("nope(1)".to_string()));
    assert!(metric_override::apply(&mut hive, &overrides, &shape).is_err());
}

#[test]
fn metric_overrides_read_the_defaults() {
    let data = data();
    let shape = reference_shape();
    let mut overrides = BTreeMap::new();
    overrides.insert(
        "ascender".to_string(),
        Value::Str("default_cap + 230".to_string()),
    );
    let mut hive = init(&data, &shape);
    metric_override::apply(&mut hive, &overrides, &shape).expect("apply");
    // cap is 735 in `parameters.toml`.
    assert_eq!(hive["ascender"].as_f64(), Some(965.0));
}
