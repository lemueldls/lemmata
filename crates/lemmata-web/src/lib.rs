//! The browser build.
//!
//! The generator is one Rust core with two front ends, so this crate is the
//! browser one: the same build the CLI runs, behind a `wasm-bindgen` surface.
//! [`build`] is the whole of it and stays free of the wasm types, so it runs
//! under `cargo test` on the host as well as in the browser.
//!
//! The input is a capture and a plan. The glyphs the port draws are taken from
//! the definitions, and everything else keeps the capture's outline, so a plan
//! reaches the drawing for the letters that are ported and the tables for the
//! rest. [`ported`] is the seam: one name to one set of contours.

use std::collections::BTreeMap;

use lemmata_build::font::{GlyphData, Names};
use lemmata_geom::{Geometry, QuadifyPoint};
use lemmata_ptl::metrics::{Metrics, Para};
use lemmata_ptl::param::{self, Shape};
use lemmata_ptl::shapes::Design;
use wasm_bindgen::prelude::*;

/// The parameter files the port's metrics are derived from. They are upstream's
/// own, read verbatim, and together they are about twenty kilobytes.
const PARAMETERS: &str = include_str!("../../../vendor/Iosevka/params/parameters.toml");
const WEIGHTS: &str = include_str!("../../../vendor/Iosevka/params/shape-weight.toml");
const WIDTHS: &str = include_str!("../../../vendor/Iosevka/params/shape-width.toml");
const SLOPES: &str = include_str!("../../../vendor/Iosevka/params/shape-slope.toml");

/// The shape a plan asks the glyph source for.
///
/// The plan carries the output metrics and the variant choices, not Iosevka's
/// shape names, so the three that matter are read off the classes it does carry.
/// The serifs default to sans, which is the family the fixtures were built from.
fn shape_of(plan: &lemmata_plan::Plan) -> Shape {
    Shape {
        serifs: Some("sans".to_string()),
        spacing: Some(plan.spacing.name().to_string()),
        weight: f64::from(plan.weight_class),
        width: f64::from(plan.width_class) * 100.0,
        slope_angle: plan.italic_angle,
        slope: if plan.italic_angle == 0.0 {
            "Upright".to_string()
        } else {
            "Italic".to_string()
        },
    }
}

/// The metrics the ported definitions read, derived from the plan.
///
/// Returns `None` when the parameter data cannot be read, which leaves the
/// capture's contours in place rather than failing the build.
fn port_metrics(plan: &lemmata_plan::Plan) -> Option<Metrics> {
    let data = param::load_data(PARAMETERS, WEIGHTS, WIDTHS, SLOPES).ok()?;
    let hive = param::init(&data, &shape_of(plan));
    let para = Para::from_named(
        |key| hive.get(key).and_then(param::toml::Value::as_f64),
        |key| hive.get(key).and_then(param::toml::Value::as_bool),
    )
    .ok()?;

    Some(Metrics::calculate(&para))
}

/// The glyphs the port draws, as contours, by name.
///
/// Each is put through the same simplify and quadratic conversion the capture
/// went through, so a caller can swap one set for the other without knowing
/// where a glyph came from.
#[must_use]
pub fn ported(plan: &lemmata_plan::Plan) -> BTreeMap<String, Vec<Vec<QuadifyPoint>>> {
    let Some(metrics) = port_metrics(plan) else {
        return BTreeMap::new();
    };

    lemmata_glyphs::registry::build(&metrics, &Design::default())
        .into_iter()
        .map(|(name, geometry)| {
            let contours = Geometry::Simplify {
                gizmo: kurbo::Affine::IDENTITY,
                geom: Box::new(geometry),
            }
            .to_contours();
            (name, contours)
        })
        .collect()
}

/// Rounds a captured advance to the units the font table stores.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn advance_to_u16(advance: f64) -> u16 {
    advance.round().clamp(0.0, f64::from(u16::MAX)) as u16
}

/// Builds a font from a capture and a plan.
///
/// The plan is a JSON object; any key it omits takes its default, so the page
/// can send only what a control changed. A glyph the registry draws takes its
/// contours from there, and a glyph it does not keeps the capture's.
///
/// # Errors
///
/// Returns a message when the capture does not decode, the plan does not parse,
/// or the font does not assemble.
pub fn build(capture: &[u8], plan: &str) -> Result<Vec<u8>, String> {
    let plan = lemmata_plan::from_json(plan)?;
    let metrics = plan.to_font_metrics();
    let captured = lemmata_capture::load(capture)?;
    let ported = ported(&plan);
    let glyphs: Vec<GlyphData> = captured
        .iter()
        .map(|glyph| GlyphData {
            name: glyph.name.clone(),
            unicodes: glyph.unicodes.clone(),
            advance: advance_to_u16(glyph.advance),
            contours: ported
                .get(&glyph.name)
                .cloned()
                .unwrap_or_else(|| lemmata_capture::contours(glyph)),
        })
        .collect();

    lemmata_build::font::build(&glyphs, &metrics, &Names::default(), &plan.variants)
}

/// The same build, for the page.
///
/// # Errors
///
/// Returns the message from [`build`] as a JavaScript string.
#[wasm_bindgen]
pub fn build_font(capture: &[u8], plan: &str) -> Result<Vec<u8>, JsValue> {
    build(capture, plan).map_err(|message| JsValue::from_str(&message))
}

/// The plan every key defaults to, so the page can show one.
#[must_use]
#[wasm_bindgen]
pub fn default_plan() -> String {
    let plan = lemmata_plan::Plan::default();
    format!(
        "{{\n  \"capHeight\": {},\n  \"xHeight\": {},\n  \"ascender\": {},\n  \"descender\": {},\n  \"italicAngle\": {},\n  \"weight\": {},\n  \"spacing\": \"{}\"\n}}",
        plan.cap_height,
        plan.x_height,
        plan.ascender,
        plan.descender,
        plan.italic_angle,
        plan.weight_class,
        plan.spacing.name()
    )
}

/// The variants a group has in a capture, as a JSON array of plan names.
///
/// The page's picker offers what the capture holds, so the list follows the
/// capture rather than a table written into the page. The names are the
/// kebab-case ones a plan takes, sorted, and the group's own drawing is not
/// among them: it is what a plan gets by saying nothing.
#[must_use]
#[wasm_bindgen]
pub fn variants_of(capture: &[u8], group: &str) -> String {
    let Ok(captured) = lemmata_capture::load(capture) else {
        return "[]".to_string();
    };
    let prefix = format!("{group}.");
    let mut variants: Vec<String> = captured
        .iter()
        .filter_map(|glyph| glyph.name.strip_prefix(&prefix))
        .filter(|suffix| !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_alphanumeric()))
        .map(lemmata_build::character::kebab)
        .collect();
    variants.sort();
    variants.dedup();

    format!(
        "[{}]",
        variants
            .iter()
            .map(|name| format!("\"{name}\""))
            .collect::<Vec<_>>()
            .join(",")
    )
}

/// Compresses a built font to WOFF2, which is what a page wants to serve.
///
/// # Errors
///
/// Returns the message when the container cannot be built.
#[wasm_bindgen]
pub fn to_woff2(font: &[u8]) -> Result<Vec<u8>, JsValue> {
    lemmata_build::font::woff2(font).map_err(|error| JsValue::from_str(&error))
}

/// The name the page offers the download under.
#[must_use]
#[wasm_bindgen]
pub fn output_name() -> String {
    "Lemmata-Regular.ttf".to_string()
}
