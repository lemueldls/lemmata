//! The browser build.
//!
//! The generator is one Rust core with two front ends, so this crate is the
//! browser one: the same build the CLI runs, behind a `wasm-bindgen` surface.
//! [`build`] is the whole of it and stays free of the wasm types, so it runs
//! under `cargo test` on the host as well as in the browser.
//!
//! The glyph source is [`lemmata_source`], shared with the CLI. That is what
//! keeps the two builds byte-identical: a page that drew a texture form and a
//! CLI that did not would be two different fonts.

use lemmata_build::font::Names;
use wasm_bindgen::prelude::*;

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
    let mut glyphs = lemmata_source::glyphs(&plan, &captured);
    glyphs.extend(lemmata_source::ligature_glyphs(&plan));
    let rules = lemmata_build::ligatures::add(&mut glyphs, plan.width);
    let drawn = lemmata_source::texture_forms(&plan, &captured);
    let names = names_for(&plan);

    lemmata_build::font::build_all(&glyphs, &metrics, &names, &plan.variants, rules, &drawn)
}

/// The names the page's font carries. The page has one family; the style comes
/// from the plan's weight and slope, so a bold or slanted build is not filed as
/// a Regular.
fn names_for(plan: &lemmata_plan::Plan) -> Names {
    let subfamily = plan.style_subfamily();

    Names {
        family: "Lemmata".to_string(),
        subfamily,
        typographic_family: None,
        typographic_subfamily: None,
        postscript_name: None,
        ..Names::default()
    }
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

/// A short preview of the plan, recomputing only the glyphs the text draws.
///
/// The font is whole, so the page can swap it under the specimen without
/// changing anything else. The ported glyphs the text does not use keep the
/// capture's outline, which is the reference plan's drawing. The ligature rules
/// the text can trigger are included, so the preview joins like the built file.
///
/// # Errors
///
/// Returns a message when the capture or the plan is bad, or the font does not
/// assemble.
pub fn build_preview(capture: &[u8], plan: &str, text: &str) -> Result<Vec<u8>, String> {
    let plan = lemmata_plan::from_json(plan)?;
    let metrics = plan.to_font_metrics();
    let captured = lemmata_capture::load(capture)?;
    let mut glyphs = lemmata_source::preview_glyphs(&plan, &captured, text);
    glyphs.extend(lemmata_source::ligature_glyphs(&plan));
    let wanted: std::collections::BTreeSet<u32> = text.chars().map(u32::from).collect();
    let text_names: std::collections::BTreeSet<String> = captured
        .iter()
        .filter(|glyph| glyph.unicodes.iter().any(|code| wanted.contains(code)))
        .map(|glyph| glyph.name.clone())
        .collect();
    let rules = lemmata_build::ligatures::add_for(&mut glyphs, &text_names, plan.width);
    let names = names_for(&plan);

    lemmata_build::font::build_with(&glyphs, &metrics, &names, &plan.variants, &rules)
}

/// The preview, for the page.
///
/// # Errors
///
/// Returns the message from [`build_preview`] as a JavaScript string.
#[wasm_bindgen]
pub fn preview_font(capture: &[u8], plan: &str, text: &str) -> Result<Vec<u8>, JsValue> {
    build_preview(capture, plan, text).map_err(|message| JsValue::from_str(&message))
}

/// The plan every key defaults to, so the page can show one.
#[must_use]
#[wasm_bindgen]
pub fn default_plan() -> String {
    let plan = lemmata_plan::Plan::default();
    format!(
        "{{\n  \"capHeight\": {},\n  \"xHeight\": {},\n  \"ascender\": {},\n  \"descender\": {},\n  \"italicAngle\": {},\n  \"weight\": {},\n  \"width\": {},\n  \"spacing\": \"{}\",\n  \"serifs\": \"{}\"\n}}",
        plan.cap_height,
        plan.x_height,
        plan.ascender,
        plan.descender,
        plan.italic_angle,
        plan.weight_class,
        plan.width,
        plan.spacing.name(),
        plan.serifs
    )
}

/// Every name the registry builds, sorted, one per line.
///
/// The page groups these by letter. The list carries no geometry, so it is
/// cheap to read on first paint.
#[must_use]
#[wasm_bindgen]
pub fn registry_names() -> String {
    lemmata_source::registry_names().join("\n")
}

/// A font with one private-use codepoint for every primary registry variant.
///
/// The codepoints start at `U+E000` and follow the sorted names
/// [`registry_names`] returns, filtered to the single-character groups. The page
/// uses the same order, so it can show the drawings without building a font per
/// click.
///
/// # Errors
///
/// Returns a message when the plan or the parameter data cannot be read, or when
/// the font does not assemble.
pub fn registry_font(plan: &str) -> Result<Vec<u8>, String> {
    let plan = lemmata_plan::from_json(plan)?;

    lemmata_source::registry_font(&plan)
}

/// The registry font, for the page.
///
/// # Errors
///
/// Returns the message from [`registry_font`] as a JavaScript string.
#[wasm_bindgen]
pub fn registry_specimen(plan: &str) -> Result<Vec<u8>, JsValue> {
    registry_font(plan).map_err(|message| JsValue::from_str(&message))
}

/// A font with the variants of one group, for the page.
///
/// # Errors
///
/// Returns a message when the group is absent, the plan or the parameter data
/// cannot be read, or the font does not assemble.
pub fn registry_group(plan: &str, group: &str) -> Result<Vec<u8>, String> {
    let plan = lemmata_plan::from_json(plan)?;

    lemmata_source::registry_group_font(&plan, group)
}

/// The group font, for the page.
///
/// # Errors
///
/// Returns the message from [`registry_group`] as a JavaScript string.
#[wasm_bindgen]
pub fn registry_group_specimen(plan: &str, group: &str) -> Result<Vec<u8>, JsValue> {
    registry_group(plan, group).map_err(|message| JsValue::from_str(&message))
}

/// A font with one representative drawing per letter, for the page.
///
/// # Errors
///
/// Returns a message when the plan or the parameter data cannot be read, or the
/// font does not assemble.
pub fn registry_index(plan: &str) -> Result<Vec<u8>, String> {
    let plan = lemmata_plan::from_json(plan)?;

    lemmata_source::registry_index_font(&plan)
}

/// The letter-index font, for the page.
///
/// # Errors
///
/// Returns the message from [`registry_index`] as a JavaScript string.
#[wasm_bindgen]
pub fn registry_index_specimen(plan: &str) -> Result<Vec<u8>, JsValue> {
    registry_index(plan).map_err(|message| JsValue::from_str(&message))
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
pub fn output_name() -> String {
    "Lemmata-Regular.ttf".to_string()
}
