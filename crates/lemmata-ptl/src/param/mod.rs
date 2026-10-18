//! The parameter layer, ported from `@iosevka/param`.
//!
//! A build plan names a shape: a serif style, a spacing, a weight, a width, a
//! slope. Upstream turns that into the parameter set a glyph definition reads
//! by walking a tree of named styles, where each style can inherit from others,
//! multiply, add, append, remove, or interpolate a value across a range of
//! grades. This is that walk.
//!
//! The tree is data, and the data is upstream's: `params/parameters.toml` for
//! the base, `shape-weight.toml`, `shape-width.toml` and `shape-slope.toml` for
//! the per-shape hives. They are read verbatim, which is what
//! `r[plan.iosevka-keys]` asks for.
//!
//! Not ported: the variant and ligature data (`params/variants.toml` is 13,540
//! lines and names glyphs rather than measuring them) and the subset filter.
//! Those belong to the layout work.

pub mod interpolate;
pub mod metric_override;
pub mod toml;

use std::collections::BTreeMap;

use interpolate::monotonic_interpolate;
use toml::Value;

/// One style's parameters, before the walk flattens them.
pub type Hive = BTreeMap<String, Value>;

/// The style tree.
pub type Data = BTreeMap<String, Value>;

/// The shape a plan asks for.
#[derive(Clone, Debug)]
pub struct Shape {
    pub serifs: Option<String>,
    pub spacing: Option<String>,
    pub weight: f64,
    pub width: f64,
    pub slope_angle: f64,
    /// The slope's name, which selects an alternates style rather than a blend.
    pub slope: String,
}

impl Default for Shape {
    fn default() -> Self {
        Self {
            serifs: None,
            spacing: None,
            weight: 400.0,
            width: 500.0,
            slope_angle: 0.0,
            slope: "Upright".to_string(),
        }
    }
}

/// `init`: the parameter set for one shape.
#[must_use]
pub fn init(data: &Data, shape: &Shape) -> Hive {
    let mut sink = Hive::new();
    apply(&mut sink, data, &["iosevka".to_string()], &BTreeMap::new());
    if let Some(serifs) = &shape.serifs {
        apply(
            &mut sink,
            data,
            &[format!("serifs-{serifs}")],
            &BTreeMap::new(),
        );
    }
    if let Some(spacing) = &shape.spacing {
        apply(
            &mut sink,
            data,
            &[format!("spacing-{spacing}")],
            &BTreeMap::new(),
        );
    }

    let variants: Vec<(&str, f64)> = vec![
        ("shapeWeight", shape.weight),
        ("shapeWidth", shape.width),
        ("shapeSlopeAngle", shape.slope_angle),
    ];
    for (key, value) in variants {
        let mut keys = vec![key.to_string()];
        if let Some(serifs) = &shape.serifs {
            keys.push(format!("{key}-serifs-{serifs}"));
        }
        if let Some(spacing) = &shape.spacing {
            keys.push(format!("{key}-spacing-{spacing}"));
        }
        for style in keys {
            let mut blend = BTreeMap::new();
            blend.insert(style.clone(), value);
            apply(&mut sink, data, &[style], &blend);
        }
    }

    let base = format!("slope-{}", shape.slope);
    let mut keys = vec![base.clone()];
    if let Some(serifs) = &shape.serifs {
        keys.push(format!("{base}-serifs-{serifs}"));
    }
    if let Some(spacing) = &shape.spacing {
        keys.push(format!("{base}-spacing-{spacing}"));
    }
    apply(&mut sink, data, &keys, &BTreeMap::new());

    sink
}

/// `apply`: walks each named style into the sink, in order.
pub fn apply(sink: &mut Hive, source: &Data, styles: &[String], blend: &BTreeMap<String, f64>) {
    for style in styles {
        intro(source, style, blend, sink);
    }
}

/// `intro`: one style, its inheritance, its operators, then its own keys.
fn intro(source: &Data, style: &str, blend: &BTreeMap<String, f64>, sink: &mut Hive) {
    let Some(entry) = source.get(style).and_then(Value::as_table) else {
        return;
    };
    let mut hive = entry.clone();

    if let Some(Value::Array(names)) = hive.remove("inherits") {
        for name in names {
            if let Some(name) = name.as_str() {
                intro(source, name, blend, sink);
            }
        }
    }

    for (operator, combine) in [("multiplies", Combine::Multiply), ("adds", Combine::Add)] {
        if let Some(block) = hive.remove(operator) {
            let value = blend.get(style).copied();
            let blended = hive_blend(as_hive(&block), value);
            for (key, value) in blended {
                let Some(factor) = value.as_f64() else {
                    continue;
                };
                let current = sink.get(&key).and_then(Value::as_f64).unwrap_or(0.0);
                let result = match combine {
                    Combine::Multiply => current * factor,
                    Combine::Add => current + factor,
                };
                sink.insert(key, Value::Float(result));
            }
        }
    }

    if let Some(Value::Table(appends)) = hive.remove("appends") {
        for (key, value) in appends {
            let target = sink.entry(key).or_insert_with(|| Value::Array(Vec::new()));
            if let (Value::Array(into), Value::Array(from)) = (target, &value) {
                into.extend(from.iter().cloned());
            }
        }
    }

    if let Some(Value::Table(removes)) = hive.remove("removes") {
        for (key, value) in removes {
            let Value::Array(removed) = value else {
                continue;
            };
            if let Some(Value::Array(current)) = sink.get_mut(&key) {
                current.retain(|item| !removed.contains(item));
            }
        }
    }

    let hive = hive_blend(hive, blend.get(style).copied());
    for (key, value) in hive {
        sink.insert(key, value);
    }
}

enum Combine {
    Multiply,
    Add,
}

fn as_hive(value: &Value) -> Hive {
    value.as_table().cloned().unwrap_or_default()
}

/// `hiveBlend`: a hive with a `blend` block becomes a hive whose blending keys
/// are interpolated to `value`. Without a value the block is left alone, which
/// is what upstream does.
fn hive_blend(mut hive: Hive, value: Option<f64>) -> Hive {
    let Some(block) = hive.remove("blend") else {
        return hive;
    };
    let Some(value) = value else {
        hive.insert("blend".to_string(), block);
        return hive;
    };
    let Value::Table(grades) = block else {
        return hive;
    };

    let mut keys: Vec<String> = Vec::new();
    for (grade, entry) in &grades {
        if grade.parse::<f64>().is_err() {
            continue;
        }
        if let Value::Table(fields) = entry {
            for key in fields.keys() {
                if !keys.contains(key) {
                    keys.push(key.clone());
                }
            }
        }
    }

    for key in keys {
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        for (grade, entry) in &grades {
            let Ok(grade) = grade.parse::<f64>() else {
                continue;
            };
            let Some(entry) = entry.as_table() else {
                continue;
            };
            let Some(value) = entry.get(&key).and_then(Value::as_f64) else {
                continue;
            };
            xs.push(grade);
            ys.push(value);
        }
        if xs.is_empty() {
            continue;
        }
        hive.insert(key, Value::Float(monotonic_interpolate(&xs, &ys)(value)));
    }
    hive
}

/// Loads the parameter data from upstream's files.
///
/// # Errors
///
/// Returns a message when one of them is not in the supported subset.
pub fn load_data(
    parameters: &str,
    weights: &str,
    widths: &str,
    slopes: &str,
) -> Result<Data, String> {
    let mut data = Data::new();
    for text in [parameters, weights, widths, slopes] {
        let Value::Table(table) = toml::parse(text)? else {
            return Err("a parameter file is not a table".to_string());
        };
        merge(&mut data, table)?;
    }
    Ok(data)
}

fn merge(into: &mut Data, from: Data) -> Result<(), String> {
    for (key, value) in from {
        match into.get_mut(&key) {
            None => {
                into.insert(key, value);
            }
            Some(Value::Table(existing)) => {
                let Value::Table(incoming) = value else {
                    return Err(format!("{key} is a value and a table"));
                };
                merge(existing, incoming)?;
            }
            Some(_) => return Err(format!("{key} is defined twice")),
        }
    }
    Ok(())
}
