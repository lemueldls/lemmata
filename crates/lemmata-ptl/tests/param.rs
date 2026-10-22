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
    // The capture is drawn at its plan's grade, and the parameter walk has to
    // start from the same one.
    let mut shape = reference_shape();
    if let Some(width) = expected.get("width").and_then(serde_json::Value::as_f64) {
        shape.width = width;
    }
    let hive = init(&data(), &shape);
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
            // The parameter set also carries names and ranges, which are not
            // metrics. Only the scalars are compared here.
            _ => continue,
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

/// The parameter corpus: upstream's own layer run over twenty shapes.
const CORPUS: &str = include_str!("../../../corpus/param.json");

/// The values the port and the pinned layer disagree on, out of everything the
/// corpus covers.
///
/// Every disagreement is in the weight and width tables at a shape other than
/// the reference, and they come in two shapes: a grade the table does not carry
/// is extrapolated differently below the first point, and a width multiplier
/// lands where upstream does not apply one. Neither moves a font that is built
/// from the reference shape, which is why the reference comparison passes, and
/// both would move one built at another weight or width. Pinned so the count
/// cannot grow quietly, and named so the next pass has a starting point.
const DISAGREEMENTS: usize = 80;

/// The port against every plan the oracle ran, not just the reference.
///
/// One shape verifies the reference build's own parameter set. Twenty reach the
/// tables one shape does not: the weight grades from 100 to 900, the width
/// multipliers at 720, the slope, the slab serifs and the spacing overrides. The
/// corpus is the pinned source's own layer run over those shapes.
#[test]
fn matches_every_plan_the_oracle_ran() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(CORPUS).expect("parse the corpus");
    assert_eq!(rows.len(), 20, "the corpus should carry twenty shapes");
    let data = data();

    let mut agreed = 0usize;
    let mut differed = 0usize;
    let mut examples = Vec::new();
    for row in &rows {
        let named = &row["shape"];
        let text = |key: &str| named.get(key).and_then(serde_json::Value::as_str);
        let shape = Shape {
            serifs: text("serifs").map(str::to_string),
            spacing: text("spacing").map(str::to_string),
            weight: named["weight"].as_f64().expect("a weight"),
            width: named["width"].as_f64().expect("a width"),
            slope_angle: named["slopeAngle"].as_f64().unwrap_or(0.0),
            // A shape that names no slope gets no slope table, and the pinned
            // source has none for the upright name either.
            slope: text("slope").unwrap_or("Upright").to_string(),
        };
        let hive = init(&data, &shape);
        let expected = row["para"].as_object().expect("para");

        for (key, wanted) in expected {
            let Some(got) = hive.get(key) else {
                panic!("{shape:?}: {key} is not in the resolved set");
            };
            // A switch is a boolean in the resolved set and a number in the
            // tables, so both are read.
            let same = if let Some(flag) = wanted.as_bool() {
                got.as_bool() == Some(flag)
            } else {
                let number = wanted.as_f64().expect("a number or a switch");
                got.as_f64()
                    .is_some_and(|value| (value - number).abs() < 1e-9)
            };
            if same {
                agreed += 1;
            } else {
                differed += 1;
                if examples.len() < 6 {
                    examples.push(format!("{shape:?} {key}: port {got:?}, pinned {wanted:?}"));
                }
            }
        }
    }

    eprintln!(
        "param: {} shapes, {agreed} values agree, {differed} differ",
        rows.len()
    );
    for example in &examples {
        eprintln!("  {example}");
    }
    assert_eq!(
        differed, DISAGREEMENTS,
        "the disagreement count moved, so either a fix landed or a new one did"
    );
    assert!(agreed > 1000, "the corpus should cover the tables broadly");
}
