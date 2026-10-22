//! `metric-override`, from `@iosevka/param`.
//!
//! A build plan can override a metric by name, either with a number or with an
//! expression over the defaults, the shape and a `blend` function:
//!
//! ```toml
//! [buildPlans.RefA.metricOverride]
//! cap = "700 + weight / 4"
//! xHeight = "blend(weight, [400, 520], [700, 550])"
//! ```
//!
//! Only the fields in [`FIELDS`] can be overridden. A field that cannot is
//! skipped, which is what upstream does, and it prints a message rather than
//! failing the build; the port skips it and says so in [`apply`]'s return.
//!
//! The arithmetic matches upstream's expression order and stays clear of
//! fused multiply-add, which rounds differently, and an `i64` literal becomes an
//! `f64` the way JavaScript would.
#![allow(clippy::suboptimal_flops, clippy::cast_precision_loss)]

//! The number literal accumulates its value in `f64`, digit by digit, rather
//! than parsing an integer and converting. The order changes the last bit for
//! some literals, and the point of this port is to agree with upstream.

use std::collections::BTreeMap;

use super::{Hive, Shape, interpolate::monotonic_interpolate, toml::Value};

/// The fields a plan may override.
pub const FIELDS: &[&str] = &[
    "cap",
    "ascender",
    "xHeight",
    "sb",
    "accentWidth",
    "accentClearance",
    "accentHeight",
    "accentStackOffset",
    "dotSize",
    "periodSize",
    "leading",
    "winMetricAscenderPad",
    "winMetricDescenderPad",
    "symbolMid",
    "parenSize",
    "powerlineScaleY",
    "powerlineScaleX",
    "powerlineShiftY",
    "powerlineShiftX",
    "onumZeroHeightRatio",
    "essRatioUpper",
    "essRatioLower",
    "essRatioQuestion",
    "essRatio",
    "archDepth",
    "smallArchDepth",
    "advanceScaleSp",
];

/// Applies a plan's metric overrides to a parameter set.
///
/// Returns the fields that were skipped, which upstream reports to the console.
///
/// # Errors
///
/// Returns a message when an expression does not parse, does not evaluate to a
/// number, or names an unknown identifier or function.
pub fn apply(sink: &mut Hive, overrides: &Hive, shape: &Shape) -> Result<Vec<String>, String> {
    let mut bindings = BTreeMap::new();
    for field in FIELDS {
        let value = sink.get(*field).and_then(Value::as_f64).unwrap_or(0.0);
        bindings.insert(format!("default_{field}"), value);
    }
    bindings.insert("weight".to_string(), shape.weight);
    bindings.insert("width".to_string(), shape.width);
    bindings.insert("slopeAngle".to_string(), shape.slope_angle);

    let mut skipped = Vec::new();
    for (field, expression) in overrides {
        if !FIELDS.contains(&field.as_str()) {
            skipped.push(field.clone());
            continue;
        }
        let value = match expression {
            Value::Float(number) => *number,
            Value::Int(number) => *number as f64,
            Value::Str(text) => {
                let mut state = State::new(text);
                let value = expression_of(&mut state, &bindings)?;
                state.expect_end()?;
                value
            }
            other => return Err(format!("invalid expression for {field}: {other:?}")),
        };
        sink.insert(field.clone(), Value::Float(value));
    }
    Ok(skipped)
}

/// `RootExpression`.
fn expression_of(state: &mut State, bindings: &BTreeMap<String, f64>) -> Result<f64, String> {
    skip_spaces(state);
    let value = sum(state, bindings)?;
    skip_spaces(state);
    Ok(value)
}

/// `Sum`.
fn sum(state: &mut State, bindings: &BTreeMap<String, f64>) -> Result<f64, String> {
    let mut value = term(state, bindings)?;
    skip_spaces(state);
    while state.test(b'+') || state.test(b'-') {
        let operator = state.advance().expect("an operator");
        skip_spaces(state);
        let right = term(state, bindings)?;
        skip_spaces(state);
        value = match operator {
            b'+' => value + right,
            _ => value - right,
        };
    }
    Ok(value)
}

/// `Term`.
fn term(state: &mut State, bindings: &BTreeMap<String, f64>) -> Result<f64, String> {
    let mut value = factor(state, bindings)?;
    skip_spaces(state);
    while state.test(b'*') || state.test(b'/') {
        let operator = state.advance().expect("an operator");
        skip_spaces(state);
        let right = factor(state, bindings)?;
        skip_spaces(state);
        value = match operator {
            b'*' => value * right,
            _ => value / right,
        };
    }
    Ok(value)
}

/// `Factor`, which carries the unary signs.
fn factor(state: &mut State, bindings: &BTreeMap<String, f64>) -> Result<f64, String> {
    if state.test(b'+') {
        state.advance();
        skip_spaces(state);
        return factor(state, bindings);
    }
    if state.test(b'-') {
        state.advance();
        skip_spaces(state);
        return Ok(-factor(state, bindings)?);
    }
    primitive(state, bindings)
}

/// `Primitive`.
fn primitive(state: &mut State, bindings: &BTreeMap<String, f64>) -> Result<f64, String> {
    match state.peek() {
        Some(byte) if byte.is_ascii_digit() => Ok(literal(state)),
        Some(byte) if byte.is_ascii_alphabetic() || byte == b'_' => {
            binding_or_call(state, bindings)
        }
        Some(b'(') => group(state, bindings),
        Some(b'[') => {
            // A bare list only appears as an argument, where it is read directly.
            let items = list(state, bindings)?;
            Err(format!("a list has {items:?} where a number was expected"))
        }
        _ => state.fail("a primitive"),
    }
}

/// `Lit`. The accumulation is in `f64` on purpose.
fn literal(state: &mut State) -> f64 {
    let mut integer_part = 0.0_f64;
    let mut fraction_part = 0.0_f64;
    let mut fraction_scale = 1.0_f64;
    while state.peek().is_some_and(|byte| byte.is_ascii_digit()) {
        let digit = f64::from(state.advance().expect("a digit") - b'0');
        integer_part = integer_part * 10.0 + digit;
    }
    if state.test(b'.') {
        state.advance();
        while state.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            fraction_scale /= 10.0;
            let digit = f64::from(state.advance().expect("a digit") - b'0');
            fraction_part += digit * fraction_scale;
        }
    }
    integer_part + fraction_part
}

/// `BindingOrCall`.
fn binding_or_call(state: &mut State, bindings: &BTreeMap<String, f64>) -> Result<f64, String> {
    let mut name = String::new();
    while state
        .peek()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
    {
        name.push(char::from(state.advance().expect("a letter")));
    }
    if state.test(b'(') {
        let arguments = list(state, bindings)?;
        if name == "blend" {
            Ok(blend(&arguments))
        } else {
            Err(format!("unknown function {name}"))
        }
    } else {
        bindings
            .get(&name)
            .copied()
            .ok_or_else(|| format!("unknown identifier {name}"))
    }
}

/// `Group`.
fn group(state: &mut State, bindings: &BTreeMap<String, f64>) -> Result<f64, String> {
    state.expect(b'(')?;
    skip_spaces(state);
    let value = expression_of(state, bindings)?;
    skip_spaces(state);
    state.expect(b')')?;
    Ok(value)
}

/// `List`: comma-separated expressions between brackets. `blend` is the only
/// function, and it wants numbers for the grade it is against and pairs after
/// it, so the flattened list is what its two callers read.
fn list(state: &mut State, bindings: &BTreeMap<String, f64>) -> Result<Vec<f64>, String> {
    let opener = state.peek().expect("an opener");
    let closer = match opener {
        b'(' => b')',
        _ => b']',
    };
    state.advance();
    skip_spaces(state);
    let mut items = vec![expression_of(state, bindings)?];
    skip_spaces(state);
    while state.test(b',') {
        state.advance();
        skip_spaces(state);
        if state.test(b'[') {
            // A pair: flatten it, since `blend` reads xs and ys straight off.
            let pair = nested_list(state, bindings)?;
            items.extend(pair);
        } else {
            items.push(expression_of(state, bindings)?);
        }
        skip_spaces(state);
    }
    state.expect(closer)?;
    Ok(items)
}

fn nested_list(state: &mut State, bindings: &BTreeMap<String, f64>) -> Result<Vec<f64>, String> {
    state.expect(b'[')?;
    skip_spaces(state);
    let mut items = vec![expression_of(state, bindings)?];
    skip_spaces(state);
    while state.test(b',') {
        state.advance();
        skip_spaces(state);
        items.push(expression_of(state, bindings)?);
        skip_spaces(state);
    }
    state.expect(b']')?;
    Ok(items)
}

/// `blend`: the first argument is the grade, the rest are `[grade, value]`.
///
/// It cannot fail, because `list` flattens each pair as it parses, so the
/// argument list always has a grade and a whole number of pairs.
fn blend(arguments: &[f64]) -> f64 {
    let (against, pairs) = arguments
        .split_first()
        .expect("the parser supplies a grade");
    let xs: Vec<f64> = pairs.iter().step_by(2).copied().collect();
    let ys: Vec<f64> = pairs.iter().skip(1).step_by(2).copied().collect();
    monotonic_interpolate(&xs, &ys)(*against)
}

struct State<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> State<'a> {
    const fn new(input: &'a str) -> Self {
        Self {
            bytes: input.as_bytes(),
            position: 0,
        }
    }

    // `slice::get` is not const yet, so this cannot be one.
    #[allow(clippy::missing_const_for_fn)]
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn test(&self, byte: u8) -> bool {
        self.peek() == Some(byte)
    }

    fn advance(&mut self) -> Option<u8> {
        let byte = self.peek();
        if byte.is_some() {
            self.position += 1;
        }
        byte
    }

    fn expect(&mut self, byte: u8) -> Result<(), String> {
        if self.test(byte) {
            self.advance();
            Ok(())
        } else {
            self.fail(&format!("{:?}", char::from(byte)))
        }
    }

    fn expect_end(&self) -> Result<(), String> {
        if self.position < self.bytes.len() {
            return Err(format!(
                "trailing input at {} in {:?}",
                self.position,
                String::from_utf8_lossy(self.bytes)
            ));
        }
        Ok(())
    }

    fn fail<T>(&self, wanted: &str) -> Result<T, String> {
        Err(format!(
            "expected {wanted} at {} in {:?}",
            self.position,
            String::from_utf8_lossy(self.bytes)
        ))
    }
}

fn skip_spaces(state: &mut State) {
    while matches!(state.peek(), Some(b' ' | b'\t')) {
        state.advance();
    }
}
