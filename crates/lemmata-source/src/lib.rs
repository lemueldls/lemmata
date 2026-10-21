//! The glyph source both front ends build from.
//!
//! The CLI and the browser differ in their surface, not in their glyphs. A plan
//! derives the metrics the ported definitions read, the registry draws every
//! glyph it covers, and the capture supplies the rest. Doing that in one place
//! is what keeps the native and wasm builds byte-identical: a page that draws a
//! texture form and a CLI that does not would be two different fonts.
//!
//! The registry and its contours are cached on the shape, not on the whole plan.
//! A variant choice does not reach the glyph source, so it reuses both. A
//! weight, width, slope or spacing change derives different metrics and clears
//! them. Contours are cached per glyph, so a preview fills only what it draws
//! and a later full build fills the rest.

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use lemmata_build::font::GlyphData;
use lemmata_geom::{Geometry, QuadifyPoint};
use lemmata_plan::{Plan, Spacing};
use lemmata_ptl::metrics::{Metrics, Para};
use lemmata_ptl::param::{self, Shape};
use lemmata_ptl::shapes::Design;

/// The parameter files the port's metrics are derived from. They are upstream's
/// own, read verbatim, and together they are about twenty kilobytes.
const PARAMETERS: &str = include_str!("../../../vendor/Iosevka/params/parameters.toml");
const WEIGHTS: &str = include_str!("../../../vendor/Iosevka/params/shape-weight.toml");
const WIDTHS: &str = include_str!("../../../vendor/Iosevka/params/shape-width.toml");
const SLOPES: &str = include_str!("../../../vendor/Iosevka/params/shape-slope.toml");

/// The contours of one glyph, in the form the font build takes.
pub type Contours = Vec<Vec<QuadifyPoint>>;

/// The registry: every glyph the port draws, by name.
type Registry = Vec<(String, Geometry)>;

/// The contours of every glyph in a set, by name.
type ContoursByGlyph = BTreeMap<String, Rc<Contours>>;

/// The shape a plan asks the glyph source for.
///
/// The plan carries the output metrics and the variant choices, not Iosevka's
/// shape names, so the three that matter are read off the classes it does carry.
/// The serifs default to sans, which is the family the fixtures were built from.
#[must_use]
pub fn shape_of(plan: &Plan) -> Shape {
    Shape {
        serifs: Some("sans".to_string()),
        spacing: Some(plan.spacing.name().to_string()),
        weight: f64::from(plan.weight_class),
        width: plan.width,
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
#[must_use]
pub fn metrics(plan: &Plan) -> Option<Metrics> {
    let data = param::load_data(PARAMETERS, WEIGHTS, WIDTHS, SLOPES).ok()?;
    let hive = param::init(&data, &shape_of(plan));
    let para = Para::from_named(
        |key| hive.get(key).and_then(param::toml::Value::as_f64),
        |key| hive.get(key).and_then(param::toml::Value::as_bool),
    )
    .ok()?;

    Some(Metrics::calculate(&para))
}

thread_local! {
    static SHAPE: RefCell<Option<u64>> = const { RefCell::new(None) };
    static REGISTRY: RefCell<Option<Rc<Registry>>> = const { RefCell::new(None) };
    static CONTOURS: RefCell<ContoursByGlyph> = const { RefCell::new(BTreeMap::new()) };
}

/// The key the ported glyphs depend on: the shape, not the whole plan.
fn shape_key(plan: &Plan) -> u64 {
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
fn registry_for(plan: &Plan) -> Option<Rc<Registry>> {
    let key = shape_key(plan);
    if SHAPE.with(|cached| *cached.borrow() != Some(key)) {
        SHAPE.with(|cached| *cached.borrow_mut() = Some(key));
        CONTOURS.with(|contours| contours.borrow_mut().clear());
        REGISTRY.with(|registry| *registry.borrow_mut() = None);
    }
    if let Some(registry) = REGISTRY.with(|registry| registry.borrow().clone()) {
        return Some(registry);
    }
    let metrics = metrics(plan)?;
    let built = Rc::new(lemmata_glyphs::registry::build(
        &metrics,
        &Design::default(),
    ));
    REGISTRY.with(|registry| *registry.borrow_mut() = Some(Rc::clone(&built)));

    Some(built)
}

/// Fills the contour cache. `only` limits it to the names a preview draws.
pub fn fill(plan: &Plan, only: Option<&BTreeSet<String>>) {
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

/// One ported glyph's contours, when they are in the cache.
#[must_use]
pub fn contours_of(name: &str) -> Option<Rc<Contours>> {
    CONTOURS.with(|cache| cache.borrow().get(name).cloned())
}

/// The glyphs the port draws, as contours, by name.
///
/// Each is put through the same simplify and quadratic conversion the capture
/// went through, so a caller can swap one set for the other without knowing
/// where a glyph came from.
#[must_use]
pub fn ported(plan: &Plan) -> ContoursByGlyph {
    fill(plan, None);

    CONTOURS.with(|cache| cache.borrow().clone())
}

thread_local! {
    static CAPTURE_KEY: RefCell<Option<u64>> = const { RefCell::new(None) };
    static CAPTURE_CONTOURS: RefCell<ContoursByGlyph> = const { RefCell::new(BTreeMap::new()) };
}

/// The capture's own contours, by name.
///
/// A preview is called once per plan change but the capture changes only when
/// the spacing does, and turning its trees into contours costs about as much as
/// a fifth of the ported set.
#[must_use]
pub fn capture_map(captured: &[lemmata_capture::Glyph]) -> ContoursByGlyph {
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
#[must_use]
pub fn box_of(contours: &[Vec<QuadifyPoint>]) -> Option<(f64, f64, f64, f64)> {
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
fn reference_registry() -> Option<Rc<Registry>> {
    thread_local! {
        static REFERENCE: RefCell<Option<Rc<Registry>>> = const { RefCell::new(None) };
    }
    if let Some(registry) = REFERENCE.with(|cache| cache.borrow().clone()) {
        return Some(registry);
    }
    let metrics = metrics(&Plan::default())?;
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
#[must_use]
pub fn drawn_from(base: &str, contours: &[Vec<QuadifyPoint>]) -> Option<String> {
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
#[must_use]
pub fn base_variants(
    captured: &[lemmata_capture::Glyph],
    contours: &ContoursByGlyph,
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

/// Rounds a captured advance to the units the font table stores.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn advance_to_u16(advance: f64) -> u16 {
    advance.round().clamp(0.0, f64::from(u16::MAX)) as u16
}

/// The advance the plan gives a glyph.
///
/// A normal-spacing face has one advance for every glyph, and the plan's width
/// is it. The capture's advance is the width the capture was drawn at, which is
/// the same only when the plan asks for the default. A quasi-proportional
/// capture carries an advance per letter class, so the plan keeps the capture's
/// and the width reaches the ported outlines only.
fn advance_of(plan: &Plan, glyph: &lemmata_capture::Glyph) -> u16 {
    match plan.spacing {
        Spacing::Normal => advance_to_u16(plan.width),
        Spacing::QuasiProportional => advance_to_u16(glyph.advance),
    }
}

/// The contours of an unported glyph, centred in the plan's frame.
///
/// A wider plan adds the space to the sidebearings, so the drawing moves half
/// the difference. Left where the capture put it, the ink would sit against the
/// left edge of a wider advance.
fn centred(plan: &Plan, glyph: &lemmata_capture::Glyph, contours: &Contours) -> Contours {
    let dx = (plan.width - glyph.advance) / 2.0;
    if plan.spacing != Spacing::Normal || dx.abs() < 0.5 {
        return contours.clone();
    }

    contours
        .iter()
        .map(|contour| {
            contour
                .iter()
                .map(|point| QuadifyPoint {
                    x: point.x + dx,
                    ..*point
                })
                .collect()
        })
        .collect()
}

/// The glyphs a plan builds, from the ported definitions and the capture.
///
/// A base letter takes the contours of the variant it was drawn from, so a plan
/// reaches a codepoint's drawing. A variant the capture never held is added when
/// the plan names it, so the choice still reaches the font.
#[must_use]
pub fn glyphs(plan: &Plan, captured: &[lemmata_capture::Glyph]) -> Vec<GlyphData> {
    fill(plan, None);
    let capture = capture_map(captured);
    let bases = base_variants(captured, &capture);
    let mut glyphs: Vec<GlyphData> = captured
        .iter()
        .map(|glyph| {
            let drawn = bases
                .get(&glyph.name)
                .and_then(|variant| contours_of(variant))
                .or_else(|| contours_of(&glyph.name));
            let contours = drawn.map_or_else(
                || {
                    capture
                        .get(&glyph.name)
                        .map_or_else(Vec::new, |contours| centred(plan, glyph, contours))
                },
                |contours| contours.as_ref().clone(),
            );
            GlyphData {
                name: glyph.name.clone(),
                unicodes: glyph.unicodes.clone(),
                advance: advance_of(plan, glyph),
                contours,
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

    glyphs
}

/// The glyphs a short piece of text draws, with the rest left at the capture's
/// outline.
///
/// The output is a whole glyph set, not a subset: the capture's glyphs are cheap
/// to carry and the layout tables need them, while the ported contours are what
/// costs. A glyph the text does not use keeps the capture's outline, which is
/// the same drawing at the reference plan.
#[must_use]
pub fn preview_glyphs(
    plan: &Plan,
    captured: &[lemmata_capture::Glyph],
    text: &str,
) -> Vec<GlyphData> {
    let wanted: BTreeSet<u32> = text.chars().map(u32::from).collect();
    let capture = capture_map(captured);
    let bases = base_variants(captured, &capture);
    let mut needed: BTreeSet<String> = captured
        .iter()
        .filter(|glyph| glyph.unicodes.iter().any(|code| wanted.contains(code)))
        .filter_map(|glyph| bases.get(&glyph.name).cloned())
        .collect();
    // A chosen variant is part of what the text draws, and the capture may not
    // hold it. Its contours are ported, so fill them and let the code below add
    // the glyph the font applies.
    for (group, variant) in &plan.variants {
        let in_text = captured.iter().any(|glyph| {
            glyph.name == *group && glyph.unicodes.iter().any(|code| wanted.contains(code))
        });
        if in_text {
            needed.insert(lemmata_build::character::variant_name(group, variant));
        }
    }
    fill(plan, Some(&needed));

    let mut glyphs: Vec<GlyphData> = captured
        .iter()
        .map(|glyph| {
            let drawn = bases
                .get(&glyph.name)
                .filter(|_| glyph.unicodes.iter().any(|code| wanted.contains(code)))
                .and_then(|variant| contours_of(variant))
                .or_else(|| contours_of(&glyph.name));
            let contours = drawn.map_or_else(
                || {
                    capture
                        .get(&glyph.name)
                        .map_or_else(Vec::new, |contours| centred(plan, glyph, contours))
                },
                |contours| contours.as_ref().clone(),
            );
            GlyphData {
                name: glyph.name.clone(),
                unicodes: glyph.unicodes.clone(),
                advance: advance_of(plan, glyph),
                contours,
            }
        })
        .collect();

    // A plan choice can name a drawing the capture never held. The arm above
    // only maps letters the capture has, so add the chosen variant here.
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

    glyphs
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

/// Every name the registry builds, sorted.
///
/// The list carries no geometry, so it is cheap to read on first paint.
#[must_use]
pub fn registry_names() -> Vec<String> {
    let Some(metrics) = metrics(&Plan::default()) else {
        return Vec::new();
    };
    let mut names: Vec<String> = lemmata_glyphs::registry::build(&metrics, &Design::default())
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    names.sort();
    names.dedup();

    names
}

/// The variant names for one group, sorted.
fn group_names(registry: &Registry, group: &str) -> Vec<String> {
    let prefix = format!("{group}.");
    let mut names: Vec<String> = registry
        .iter()
        .filter_map(|(name, _)| {
            let suffix = name.strip_prefix(&prefix)?;
            if suffix.is_empty() || !suffix.chars().all(|c| c.is_ascii_alphanumeric()) {
                return None;
            }

            Some(name.clone())
        })
        .collect();
    names.sort();

    names
}

/// The advance the registry fonts use for every glyph.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn registry_advance(metrics: &Metrics) -> u16 {
    metrics.width.clamp(0.0, f64::from(u16::MAX)).round() as u16
}

/// Assembles a font from the registry glyphs the caller prepared.
fn registry_build(plan: &Plan, glyphs: &[GlyphData]) -> Result<Vec<u8>, String> {
    lemmata_build::font::build(
        glyphs,
        &plan.to_font_metrics(),
        &lemmata_build::font::Names::default(),
        &[],
    )
}

/// A font with one private-use codepoint for every primary registry variant.
///
/// The codepoints start at `U+E000` and follow the sorted names
/// [`registry_names`] returns, filtered to the single-character groups. The
/// registry and its contours are cached on the shape, so a second call after a
/// variant choice reuses everything but the font assembly.
///
/// # Errors
///
/// Returns a message when the parameter data cannot be read or the font does not
/// assemble.
pub fn registry_font(plan: &Plan) -> Result<Vec<u8>, String> {
    let Some(registry) = registry_for(plan) else {
        return Err("the parameter data did not load".to_string());
    };
    let metrics = metrics(plan).ok_or_else(|| "the parameter data did not load".to_string())?;
    let mut names: Vec<String> = registry
        .iter()
        .filter(|(name, _)| primary_group(name).is_some())
        .map(|(name, _)| name.clone())
        .collect();
    names.sort();

    let only: BTreeSet<String> = names.iter().cloned().collect();
    fill(plan, Some(&only));
    let advance = registry_advance(&metrics);
    let glyphs: Vec<GlyphData> = names
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            let codepoint = 0xE000 + u32::try_from(index).unwrap_or(0);
            let contours = contours_of(&name)
                .map(|contours| contours.as_ref().clone())
                .unwrap_or_default();

            GlyphData {
                name,
                unicodes: vec![codepoint],
                advance,
                contours,
            }
        })
        .collect();

    registry_build(plan, &glyphs)
}

/// A font with one private-use codepoint for every variant of one group.
///
/// The codepoints start at `U+E000` and follow the group's sorted names, which
/// is the same order the page lists them in. Only this group's contours are
/// filled, so changing the group does not cost the rest of the registry.
///
/// # Errors
///
/// Returns a message when the group is absent, the parameter data cannot be
/// read, or the font does not assemble.
pub fn registry_group_font(plan: &Plan, group: &str) -> Result<Vec<u8>, String> {
    let Some(registry) = registry_for(plan) else {
        return Err("the parameter data did not load".to_string());
    };
    let names = group_names(&registry, group);
    if names.is_empty() {
        return Err(format!("{group} is not in the registry"));
    }
    let metrics = metrics(plan).ok_or_else(|| "the parameter data did not load".to_string())?;

    let only: BTreeSet<String> = names.iter().cloned().collect();
    fill(plan, Some(&only));
    let advance = registry_advance(&metrics);
    let glyphs: Vec<GlyphData> = names
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            let codepoint = 0xE000 + u32::try_from(index).unwrap_or(0);
            let contours = contours_of(&name)
                .map(|contours| contours.as_ref().clone())
                .unwrap_or_default();

            GlyphData {
                name,
                unicodes: vec![codepoint],
                advance,
                contours,
            }
        })
        .collect();

    registry_build(plan, &glyphs)
}

/// A font with one representative drawing per letter, mapped to the letter.
///
/// The letter grid uses this font, so a group's tile shows the drawing the plan
/// currently chooses for it without carrying every variant. Only the
/// representative contours are filled.
///
/// # Errors
///
/// Returns a message when the parameter data cannot be read or the font does not
/// assemble.
pub fn registry_index_font(plan: &Plan) -> Result<Vec<u8>, String> {
    let Some(registry) = registry_for(plan) else {
        return Err("the parameter data did not load".to_string());
    };
    let metrics = metrics(plan).ok_or_else(|| "the parameter data did not load".to_string())?;

    let mut names: Vec<String> = registry
        .iter()
        .filter_map(|(name, _)| primary_group(name).map(|_| name.clone()))
        .collect();
    names.sort();

    let mut representatives: BTreeMap<String, String> = BTreeMap::new();
    for name in names {
        let Some(group) = primary_group(&name) else {
            continue;
        };
        representatives.entry(group.to_string()).or_insert(name);
    }
    for (group, variant) in &plan.variants {
        if !representatives.contains_key(group) {
            continue;
        }
        let chosen = format!("{group}.{}", lemmata_build::character::camel(variant));
        if registry.iter().any(|(name, _)| name == &chosen) {
            representatives.insert(group.clone(), chosen);
        }
    }

    let only: BTreeSet<String> = representatives.values().cloned().collect();
    fill(plan, Some(&only));
    let advance = registry_advance(&metrics);
    let glyphs: Vec<GlyphData> = representatives
        .into_iter()
        .filter_map(|(group, name)| {
            let codepoint = u32::from(group.chars().next()?);
            let contours = contours_of(&name)?;

            Some(GlyphData {
                name,
                unicodes: vec![codepoint],
                advance,
                contours: contours.as_ref().clone(),
            })
        })
        .collect();

    registry_build(plan, &glyphs)
}

/// The texture forms the port draws, by base glyph and form.
///
/// A base letter's name is not the name of the drawing the registry builds; the
/// capture's contours identify the variant, and the same variant is built at
/// `1 ± 1/12`. The side is applied by the build, which knows the advance.
#[must_use]
pub fn texture_forms(
    plan: &Plan,
    captured: &[lemmata_capture::Glyph],
) -> lemmata_build::texture::Drawn {
    use lemmata_build::texture::{Drawn, Form, SHIFT};

    let Some(metrics) = metrics(plan) else {
        return Drawn::new();
    };
    let design = Design::default();
    let wide = lemmata_glyphs::registry::build_at(&metrics, &design, 1.0 + SHIFT);
    let narrow = lemmata_glyphs::registry::build_at(&metrics, &design, 1.0 - SHIFT);
    let contours = |set: &[(String, Geometry)], name: &str| -> Option<Contours> {
        let (_, geometry) = set.iter().find(|(built, _)| built == name)?;

        Some(
            Geometry::Simplify {
                gizmo: kurbo::Affine::IDENTITY,
                geom: Box::new(geometry.clone()),
            }
            .to_contours(),
        )
    };

    let capture = capture_map(captured);
    let bases = base_variants(captured, &capture);
    let mut out = Drawn::new();
    for (base, variant) in &bases {
        let Some(wide) = contours(&wide, variant) else {
            continue;
        };
        let Some(narrow) = contours(&narrow, variant) else {
            continue;
        };
        for (form, source) in [
            (Form::ExtL, &wide),
            (Form::ExtR, &wide),
            (Form::ExtLR, &wide),
            (Form::ShrL, &narrow),
            (Form::ShrR, &narrow),
            (Form::ShrLR, &narrow),
        ] {
            out.insert((base.clone(), form), source.clone());
        }
    }

    out
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
        let mut variant = base;
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
