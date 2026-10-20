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

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

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

/// The contours of one ported glyph.
pub type PortedContours = Vec<Vec<QuadifyPoint>>;

// The registry and its contours, keyed by the shape the plan asks for. A
// variant choice does not reach the glyph source, so it reuses both. A weight,
// width, slope or spacing change derives different metrics and clears them.
//
// Contours are cached per glyph, so a preview fills only the ones it draws and a
// later full build fills the rest.
thread_local! {
    static SHAPE: RefCell<Option<u64>> = const { RefCell::new(None) };
    static REGISTRY: RefCell<Option<Rc<Vec<(String, Geometry)>>>> = const { RefCell::new(None) };
    static CONTOURS: RefCell<BTreeMap<String, Rc<PortedContours>>> = const { RefCell::new(BTreeMap::new()) };
}

/// The key the ported glyphs depend on: the shape, not the whole plan.
fn shape_key(plan: &lemmata_plan::Plan) -> u64 {
    let shape = shape_of(plan);
    let mut hasher = DefaultHasher::new();
    shape.serifs.hash(&mut hasher);
    shape.spacing.hash(&mut hasher);
    shape.weight.to_bits().hash(&mut hasher);
    shape.width.to_bits().hash(&mut hasher);
    shape.slope_angle.to_bits().hash(&mut hasher);
    shape.slope.hash(&mut hasher);

    hasher.finish()
}

/// The registry for this shape, rebuilt when the shape changed.
fn registry_for(plan: &lemmata_plan::Plan) -> Option<Rc<Vec<(String, Geometry)>>> {
    let key = shape_key(plan);
    if SHAPE.with(|cached| *cached.borrow() != Some(key)) {
        SHAPE.with(|cached| *cached.borrow_mut() = Some(key));
        CONTOURS.with(|contours| contours.borrow_mut().clear());
        REGISTRY.with(|registry| *registry.borrow_mut() = None);
    }
    if let Some(registry) = REGISTRY.with(|registry| registry.borrow().clone()) {
        return Some(registry);
    }
    let metrics = port_metrics(plan)?;
    let built = Rc::new(lemmata_glyphs::registry::build(
        &metrics,
        &Design::default(),
    ));
    REGISTRY.with(|registry| *registry.borrow_mut() = Some(Rc::clone(&built)));

    Some(built)
}

/// Fills the contour cache. `only` limits it to the names a preview draws.
fn fill(plan: &lemmata_plan::Plan, only: Option<&BTreeSet<String>>) {
    let Some(registry) = registry_for(plan) else {
        return;
    };
    let mut todo = Vec::new();
    for (name, geometry) in registry.iter() {
        if only.is_some_and(|only| !only.contains(name)) {
            continue;
        }
        if CONTOURS.with(|contours| contours.borrow().contains_key(name)) {
            continue;
        }
        todo.push((name.clone(), geometry.clone()));
    }
    for (name, geometry) in todo {
        let contours = Geometry::Simplify {
            gizmo: kurbo::Affine::IDENTITY,
            geom: Box::new(geometry),
        }
        .to_contours();
        CONTOURS.with(|cache| {
            cache.borrow_mut().insert(name, Rc::new(contours));
        });
    }
}

fn contours_of(name: &str) -> Option<Rc<PortedContours>> {
    CONTOURS.with(|cache| cache.borrow().get(name).cloned())
}

/// The glyphs the port draws, as contours, by name.
///
/// Each is put through the same simplify and quadratic conversion the capture
/// went through, so a caller can swap one set for the other without knowing
/// where a glyph came from.
#[must_use]
pub fn ported(plan: &lemmata_plan::Plan) -> BTreeMap<String, Rc<PortedContours>> {
    fill(plan, None);

    CONTOURS.with(|cache| cache.borrow().clone())
}

// The capture's own contours, by name. A preview is called once per plan change
// but the capture changes only when the spacing does, and turning its trees into
// contours costs about as much as a fifth of the ported set.
thread_local! {
    static CAPTURE_KEY: RefCell<Option<u64>> = const { RefCell::new(None) };
    static CAPTURE_CONTOURS: RefCell<BTreeMap<String, Rc<PortedContours>>> =
        const { RefCell::new(BTreeMap::new()) };
}

/// The capture's contours, rebuilt only when a different capture arrives.
fn capture_map(captured: &[lemmata_capture::Glyph]) -> BTreeMap<String, Rc<PortedContours>> {
    let mut hasher = DefaultHasher::new();
    for glyph in captured {
        glyph.name.hash(&mut hasher);
        glyph.unicodes.hash(&mut hasher);
    }
    let key = hasher.finish();
    if CAPTURE_KEY.with(|cached| *cached.borrow() != Some(key)) {
        let map = captured
            .iter()
            .map(|glyph| {
                (
                    glyph.name.clone(),
                    Rc::new(lemmata_capture::contours(glyph)),
                )
            })
            .collect();
        CAPTURE_CONTOURS.with(|cache| *cache.borrow_mut() = map);
        CAPTURE_KEY.with(|cached| *cached.borrow_mut() = Some(key));
    }

    CAPTURE_CONTOURS.with(|cache| cache.borrow().clone())
}

/// The bounding box of a set of contours.
fn box_of(contours: &[Vec<QuadifyPoint>]) -> Option<(f64, f64, f64, f64)> {
    contours.iter().flatten().fold(None, |acc, point| {
        Some(
            acc.map_or((point.x, point.y, point.x, point.y), |(l, b, r, t)| {
                (
                    l.min(point.x),
                    b.min(point.y),
                    r.max(point.x),
                    t.max(point.y),
                )
            }),
        )
    })
}

/// The registry's glyphs at the reference plan, which is what a capture's base
/// letters were drawn from.
fn reference_registry() -> Option<Rc<Vec<(String, Geometry)>>> {
    thread_local! {
        static REFERENCE: RefCell<Option<Rc<Vec<(String, Geometry)>>>> =
            const { RefCell::new(None) };
    }
    if let Some(registry) = REFERENCE.with(|cache| cache.borrow().clone()) {
        return Some(registry);
    }
    let metrics = port_metrics(&lemmata_plan::Plan::default())?;
    let registry = Rc::new(lemmata_glyphs::registry::build(
        &metrics,
        &Design::default(),
    ));
    REFERENCE.with(|cache| *cache.borrow_mut() = Some(Rc::clone(&registry)));

    Some(registry)
}

/// Which variant a capture's base letter was drawn from.
///
/// The registry names variants (`a.doubleStoreySerifless`), not the base letter
/// the font maps a codepoint to, so a plan cannot reach the letter until the two
/// are tied together. The tie is the drawing: at the reference plan the base
/// letter's contours are one of its variants', and the first one that matches is
/// the one the table selected by default.
fn drawn_from(base: &str, contours: &[Vec<QuadifyPoint>]) -> Option<String> {
    thread_local! {
        static DEFAULTS: RefCell<BTreeMap<String, String>> = const { RefCell::new(BTreeMap::new()) };
    }
    if let Some(name) = DEFAULTS.with(|cache| cache.borrow().get(base).cloned()) {
        return Some(name);
    }
    let wanted = box_of(contours)?;
    let registry = reference_registry()?;
    for (name, geometry) in registry.iter() {
        let Some(rest) = name.strip_prefix(base) else {
            continue;
        };
        if !rest.starts_with('.') {
            continue;
        }
        let candidate = Geometry::Simplify {
            gizmo: kurbo::Affine::IDENTITY,
            geom: Box::new(geometry.clone()),
        }
        .to_contours();
        let Some(found) = box_of(&candidate) else {
            continue;
        };
        let close = (found.0 - wanted.0).abs() <= 0.5
            && (found.1 - wanted.1).abs() <= 0.5
            && (found.2 - wanted.2).abs() <= 0.5
            && (found.3 - wanted.3).abs() <= 0.5;
        if close {
            DEFAULTS.with(|cache| cache.borrow_mut().insert(base.to_string(), name.clone()));

            return Some(name.clone());
        }
    }

    None
}

/// The variant each of the capture's base letters is drawn from.
fn base_variants(
    captured: &[lemmata_capture::Glyph],
    contours: &BTreeMap<String, Rc<PortedContours>>,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for glyph in captured {
        if glyph.name.contains('.') || glyph.name.contains('/') {
            continue;
        }
        let Some(drawn) = contours.get(&glyph.name) else {
            continue;
        };
        if let Some(name) = drawn_from(&glyph.name, drawn) {
            out.insert(glyph.name.clone(), name);
        }
    }

    out
}

/// Builds a font for a short piece of text, recomputing only the glyphs it
/// draws.
///
/// The output is the whole font, not a subset: the capture's glyphs are cheap to
/// carry and the layout tables need them, while the ported contours are what
/// costs. A glyph the text does not use keeps the capture's outline, which is
/// the same drawing at the reference plan.
///
/// # Errors
///
/// Returns the message from [`build`] when the capture or the plan is bad.
pub fn build_preview(capture: &[u8], plan: &str, text: &str) -> Result<Vec<u8>, String> {
    let plan = lemmata_plan::from_json(plan)?;
    let metrics = plan.to_font_metrics();
    let captured = lemmata_capture::load(capture)?;
    let wanted: BTreeSet<u32> = text.chars().map(u32::from).collect();
    let capture = capture_map(&captured);
    let bases = base_variants(&captured, &capture);
    let needed: BTreeSet<String> = captured
        .iter()
        .filter(|glyph| glyph.unicodes.iter().any(|code| wanted.contains(code)))
        .filter_map(|glyph| bases.get(&glyph.name).cloned())
        .collect();
    fill(&plan, Some(&needed));

    let glyphs: Vec<GlyphData> = captured
        .iter()
        .map(|glyph| {
            let drawn = bases
                .get(&glyph.name)
                .filter(|_| glyph.unicodes.iter().any(|code| wanted.contains(code)))
                .and_then(|variant| contours_of(variant))
                .or_else(|| contours_of(&glyph.name));
            GlyphData {
                name: glyph.name.clone(),
                unicodes: glyph.unicodes.clone(),
                advance: advance_to_u16(glyph.advance),
                contours: drawn
                    .or_else(|| capture.get(&glyph.name).cloned())
                    .map(|contours| contours.as_ref().clone())
                    .unwrap_or_default(),
            }
        })
        .collect();

    lemmata_build::font::build(&glyphs, &metrics, &Names::default(), &plan.variants)
}

/// The group a glyph belongs to when the group is a single character.
///
/// The registry also builds derived glyphs (`aCap`, `bSlash`, `mathbb/b`) and
/// small capitals. A plan chooses a letter, so those are not groups the page
/// offers.
fn primary_group(name: &str) -> Option<&str> {
    let (group, suffix) = name.split_once('.')?;
    if group.chars().count() != 1 {
        return None;
    }
    if !group.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    if suffix.is_empty() || !suffix.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }

    Some(group)
}

/// Every name the registry builds, sorted, one per line.
///
/// The page groups these by letter. The list carries no geometry, so it is
/// cheap to read on first paint.
#[must_use]
#[wasm_bindgen]
pub fn registry_names() -> String {
    let Some(metrics) = port_metrics(&lemmata_plan::Plan::default()) else {
        return String::new();
    };
    let mut names: Vec<String> = lemmata_glyphs::registry::build(&metrics, &Design::default())
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    names.sort();
    names.dedup();

    names.join("\n")
}

/// A font with one private-use codepoint for every primary registry variant.
///
/// The codepoints start at `U+E000` and follow the sorted names `registry_names`
/// returns, filtered to the single-character groups. The page uses the same
/// order, so it can show the drawings without building a font per click.
///
/// # Errors
///
/// Returns a message when the plan or the parameter data cannot be read, or when
/// the font does not assemble.
pub fn registry_font(plan: &str) -> Result<Vec<u8>, String> {
    let plan = lemmata_plan::from_json(plan)?;
    let font_metrics = plan.to_font_metrics();
    let metrics =
        port_metrics(&plan).ok_or_else(|| "the parameter data did not load".to_string())?;
    let mut entries: Vec<(String, Geometry)> =
        lemmata_glyphs::registry::build(&metrics, &Design::default())
            .into_iter()
            .filter(|(name, _)| primary_group(name).is_some())
            .collect();
    entries.sort_by(|left, right| left.0.cmp(&right.0));

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let advance = metrics.width.clamp(0.0, f64::from(u16::MAX)).round() as u16;

    let glyphs: Vec<GlyphData> = entries
        .into_iter()
        .enumerate()
        .map(|(index, (name, geometry))| {
            let codepoint = 0xE000 + u32::try_from(index).unwrap_or(0);

            GlyphData {
                name,
                unicodes: vec![codepoint],
                advance,
                contours: Geometry::Simplify {
                    gizmo: kurbo::Affine::IDENTITY,
                    geom: Box::new(geometry),
                }
                .to_contours(),
            }
        })
        .collect();

    lemmata_build::font::build(&glyphs, &font_metrics, &Names::default(), &[])
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
    fill(&plan, None);
    let capture = capture_map(&captured);
    let bases = base_variants(&captured, &capture);
    let mut glyphs: Vec<GlyphData> = captured
        .iter()
        .map(|glyph| {
            let drawn = bases
                .get(&glyph.name)
                .and_then(|variant| contours_of(variant))
                .or_else(|| contours_of(&glyph.name));
            GlyphData {
                name: glyph.name.clone(),
                unicodes: glyph.unicodes.clone(),
                advance: advance_to_u16(glyph.advance),
                contours: drawn
                    .or_else(|| capture.get(&glyph.name).cloned())
                    .map(|contours| contours.as_ref().clone())
                    .unwrap_or_default(),
            }
        })
        .collect();

    // A plan choice can name a drawing the capture never held. Add that one
    // glyph before the choice is applied, so the default build stays the same
    // and a selected variant still reaches the font.
    let mut known: BTreeSet<String> = glyphs.iter().map(|glyph| glyph.name.clone()).collect();
    for (group, variant) in &plan.variants {
        let name = lemmata_build::character::variant_name(group, variant);
        if known.contains(&name) {
            continue;
        }
        let Some(contours) = contours_of(&name) else {
            continue;
        };
        let Some(advance) = glyphs
            .iter()
            .find(|glyph| glyph.name == *group)
            .map(|glyph| glyph.advance)
        else {
            continue;
        };
        glyphs.push(GlyphData {
            name: name.clone(),
            unicodes: Vec::new(),
            advance,
            contours: contours.as_ref().clone(),
        });
        known.insert(name);
    }

    // The ligature targets are not letters the capture holds, so they are
    // added from the fixture. A glyph the capture already has keeps its own
    // outline.
    // The ligature glyphs and rules are part of the font source, so the build
    // crate carries them for both front ends.
    let rules = lemmata_build::ligatures::add(&mut glyphs);

    lemmata_build::font::build_with(&glyphs, &metrics, &Names::default(), &plan.variants, rules)
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
/// capture's outline, which is the reference plan's drawing.
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

#[cfg(test)]
mod cache_tests {
    use std::collections::BTreeSet;
    use std::rc::Rc;

    use lemmata_plan::Plan;

    use super::{CONTOURS, fill, ported};

    const NAME: &str = "a.doubleStoreySerifed";

    #[test]
    fn a_variant_choice_reuses_the_ported_glyphs() {
        let base = Plan::default();
        let first = ported(&base);
        let mut variant = base.clone();
        variant
            .variants
            .push(("a".to_string(), "single-storey-serifless".to_string()));
        let second = ported(&variant);
        assert!(
            Rc::ptr_eq(&first[NAME], &second[NAME]),
            "a variant choice changed the derived metrics"
        );

        let heavier = Plan {
            weight_class: 700,
            ..Plan::default()
        };
        let third = ported(&heavier);
        assert!(
            !Rc::ptr_eq(&first[NAME], &third[NAME]),
            "a weight change reused the old contours"
        );
    }

    #[test]
    fn a_preview_fills_only_the_glyphs_it_draws() {
        let plan = Plan::default();
        let mut only = BTreeSet::new();
        only.insert("a.crossing".to_string());
        fill(&plan, Some(&only));
        let cached = CONTOURS.with(|cache| cache.borrow().len());
        assert_eq!(cached, 1, "the preview computed {cached} glyphs");

        fill(&plan, None);
        let all = CONTOURS.with(|cache| cache.borrow().len());
        assert!(all > 100, "the full fill covered {all} glyphs");
    }
}
