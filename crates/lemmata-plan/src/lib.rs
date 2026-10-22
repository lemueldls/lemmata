//! The plan: what a caller chooses about the font.
//!
//! A first slice, and a narrow one on purpose. The plan carries the values the
//! writer puts in `head`, `hhea` and `OS/2`, which is the seam
//! [`lemmata_build::font::build`] takes. It also reaches the outlines the port
//! draws: the weight, width and slope derive the parameter set the definitions
//! read. A glyph still in the capture keeps its recorded shape.
//!
//! The default is Iosevka's vertical metrics at 1000 units per em and its 600
//! width grade, so an empty plan is the normal face the project ships. That is
//! the invariant `tests/default.rs` pins.
//!
//! The boundary format is JSON, because the page sends an object and
//! `serde_json` is already in the tree. The struct is the part a file format
//! would be an adapter over, and the keys are Iosevka's, so a plan written for
//! one tool reads in the other for the fields both have.

use std::{collections::BTreeMap, path::Path};

use lemmata_build::font::Metrics;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The spacing a font is drawn at.
///
/// It selects which capture the caller supplies, because the spacing is a
/// property of the drawings rather than of the advances: Iosevka draws each
/// glyph into a frame whose width comes from its letter class, so a
/// quasi-proportional face is a second build and not a second set of advances
/// over the same outlines. `docs/plan.md` records the measurement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spacing {
    /// The monospace face, one advance for every glyph.
    Normal,
    /// Each glyph drawn into a frame the width of its letter class, so `i` is
    /// narrow and `m` is wide.
    QuasiProportional,
}

impl Spacing {
    /// The name a plan writes.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::QuasiProportional => "quasi-proportional",
        }
    }

    /// Reads the name a plan writes.
    ///
    /// # Errors
    ///
    /// Returns a message for a name that is not one of the two spacings.
    pub fn from_name(name: &str) -> Result<Self, String> {
        match name {
            "normal" => Ok(Self::Normal),
            "quasi-proportional" => Ok(Self::QuasiProportional),
            _ => Err("spacing must be \"normal\" or \"quasi-proportional\"".to_string()),
        }
    }
}

/// The OS/2 width class Iosevka gives each width grade.
const WIDTH_GRADES: [(f64, u16); 7] = [
    (416.0, 3),
    (456.0, 4),
    (500.0, 5),
    (547.0, 6),
    (600.0, 7),
    (657.0, 8),
    (720.0, 9),
];

/// The class nearest a width.
///
/// The grades are not evenly spaced, so dividing a width by a hundred puts 600
/// in class 6 where Iosevka has class 7. A width outside the grades takes the
/// nearest end. Classes 1 and 2 are narrower than any Iosevka grade and are not
/// reachable from a width.
#[must_use]
pub fn width_class(width: f64) -> u16 {
    WIDTH_GRADES
        .iter()
        .min_by(|(left, _), (right, _)| (left - width).abs().total_cmp(&(right - width).abs()))
        .map_or(5, |(_, class)| *class)
}

/// What a caller chooses about the font.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub units_per_em: u16,
    pub ascender: i16,
    pub descender: i16,
    pub line_gap: i16,
    pub typo_ascender: i16,
    pub typo_descender: i16,
    pub typo_line_gap: i16,
    pub win_ascent: u16,
    pub win_descent: u16,
    pub x_height: i16,
    pub cap_height: i16,
    pub underline_position: i16,
    pub underline_thickness: i16,
    pub strikeout_position: i16,
    pub strikeout_thickness: i16,
    pub weight_class: u16,
    /// The width the outlines are drawn at, in font units, which is also the
    /// normal spacing's advance.
    ///
    /// Iosevka grades the width at 416, 456, 500, 547, 600, 657 and 720 and
    /// interpolates between them, so a value between two grades is a coherent
    /// frame rather than a rounded one. 500 is grade 5 and 600 grade 7; the
    /// default plan draws at 600.
    pub width: f64,
    /// The OS/2 width class, which follows from the grade unless a plan sets it
    /// on its own.
    pub width_class: u16,
    pub italic_angle: f64,
    /// The spacing the outlines were drawn at.
    pub spacing: Spacing,
    /// The serif style the outlines were drawn with. The ported source reads
    /// this, so a slab family cannot be drawn as sans.
    pub serifs: String,
    /// The character variants the plan selects, by group and variant name.
    ///
    /// Iosevka names these in kebab case under `variants.design`, and the glyph
    /// source spells the same choice in camel case: `double-storey-serifed` is
    /// the glyph `a.doubleStoreySerifed`. The names are kept as written and
    /// translated where a glyph is looked up, so a plan and an Iosevka plan read
    /// the same.
    pub variants: Vec<(String, String)>,
}

impl Default for Plan {
    fn default() -> Self {
        Self {
            units_per_em: 1000,
            ascender: 965,
            descender: -215,
            line_gap: 70,
            typo_ascender: 965,
            typo_descender: -285,
            typo_line_gap: 0,
            win_ascent: 965,
            win_descent: 285,
            x_height: 520,
            cap_height: 735,
            underline_position: -100,
            underline_thickness: 50,
            strikeout_position: 250,
            strikeout_thickness: 50,
            weight_class: 400,
            width: 600.0,
            width_class: 7,
            italic_angle: 0.0,
            spacing: Spacing::Normal,
            serifs: "sans".to_string(),
            variants: Vec::new(),
        }
    }
}

/// Reads a plan from an object, taking the default for every key it omits, so a
/// page can send the one value a control changed.
///
/// # Errors
///
/// Returns a message when the text is not a JSON object or a value has the
/// wrong shape.
pub fn from_json(text: &str) -> Result<Plan, String> {
    let mut plan = Plan::default();
    if text.trim().is_empty() {
        return Ok(plan);
    }
    let value: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let object = value
        .as_object()
        .ok_or_else(|| "the plan must be an object".to_string())?;

    let integer = |key: &str| -> Result<Option<i64>, String> {
        match object.get(key) {
            None => Ok(None),
            Some(value) => {
                value
                    .as_i64()
                    .map(Some)
                    .ok_or_else(|| format!("{key} must be a whole number"))
            }
        }
    };
    let set_i16 = |key: &str, target: &mut i16| -> Result<(), String> {
        if let Some(value) = integer(key)? {
            *target = i16::try_from(value).map_err(|_| format!("{key} is out of range"))?;
        }
        Ok(())
    };
    let set_u16 = |key: &str, target: &mut u16| -> Result<(), String> {
        if let Some(value) = integer(key)? {
            *target = u16::try_from(value).map_err(|_| format!("{key} is out of range"))?;
        }
        Ok(())
    };

    set_u16("unitsPerEm", &mut plan.units_per_em)?;
    set_i16("ascender", &mut plan.ascender)?;
    set_i16("descender", &mut plan.descender)?;
    set_i16("lineGap", &mut plan.line_gap)?;
    set_i16("typoAscender", &mut plan.typo_ascender)?;
    set_i16("typoDescender", &mut plan.typo_descender)?;
    set_i16("typoLineGap", &mut plan.typo_line_gap)?;
    set_u16("winAscent", &mut plan.win_ascent)?;
    set_u16("winDescent", &mut plan.win_descent)?;
    set_i16("xHeight", &mut plan.x_height)?;
    set_i16("capHeight", &mut plan.cap_height)?;
    set_i16("underlinePosition", &mut plan.underline_position)?;
    set_i16("underlineThickness", &mut plan.underline_thickness)?;
    set_i16("strikeoutPosition", &mut plan.strikeout_position)?;
    set_i16("strikeoutThickness", &mut plan.strikeout_thickness)?;
    set_u16("weight", &mut plan.weight_class)?;
    if let Some(value) = object.get("width") {
        let width = value
            .as_f64()
            .ok_or_else(|| "width must be a number in font units".to_string())?;
        if !(100.0..=2000.0).contains(&width) {
            return Err("width is in font units, so 500 is normal, not 5".to_string());
        }
        plan.width = width;
        plan.width_class = width_class(width);
    }
    set_u16("widthClass", &mut plan.width_class)?;
    if !(1..=9).contains(&plan.width_class) {
        return Err("widthClass is an OS/2 class from 1 to 9".to_string());
    }
    if let Some(angle) = object.get("italicAngle") {
        plan.italic_angle = angle
            .as_f64()
            .ok_or_else(|| "italicAngle must be a number".to_string())?;
    }
    if let Some(spacing) = object.get("spacing") {
        plan.spacing = Spacing::from_name(
            spacing
                .as_str()
                .ok_or_else(|| "spacing must be a name".to_string())?,
        )?;
    }
    if let Some(serifs) = object.get("serifs") {
        plan.serifs = serifs
            .as_str()
            .ok_or_else(|| "serifs must be a name".to_string())?
            .to_string();
    }
    if let Some(variants) = object.get("variants") {
        let variants = variants
            .as_object()
            .ok_or_else(|| "variants must be an object".to_string())?;
        let name = |value: &Value| -> Result<String, String> {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "a variant must be a name".to_string())
        };
        let mut chosen = Vec::new();
        for (key, value) in variants {
            match value.as_object() {
                // Iosevka splits the choices across `design`, `upright`, `italic`
                // and `oblique`. One build is one slope, so the four are read as
                // one set of choices.
                Some(table) => {
                    for (group, variant) in table {
                        chosen.push((group.clone(), name(variant)?));
                    }
                }
                None => chosen.push((key.clone(), name(value)?)),
            }
        }
        // Sorted and deduplicated, so a plan that names a group twice cannot
        // depend on the order the keys arrived in.
        chosen.sort();
        chosen.dedup();
        plan.variants = chosen;
    }
    Ok(plan)
}

impl Plan {
    /// The RIBBI style name the face's weight and slope imply.
    #[must_use]
    pub fn style_subfamily(&self) -> String {
        let bold = self.weight_class >= 700;
        let italic = self.italic_angle != 0.0;
        match (bold, italic) {
            (true, true) => "Bold Italic".to_string(),
            (true, false) => "Bold".to_string(),
            (false, true) => "Italic".to_string(),
            (false, false) => "Regular".to_string(),
        }
    }

    /// The values the writer puts in the font tables.
    #[must_use]
    pub fn to_font_metrics(&self) -> Metrics {
        Metrics {
            units_per_em: self.units_per_em,
            ascender: self.ascender,
            descender: self.descender,
            line_gap: self.line_gap,
            typo_ascender: self.typo_ascender,
            typo_descender: self.typo_descender,
            typo_line_gap: self.typo_line_gap,
            win_ascent: self.win_ascent,
            win_descent: self.win_descent,
            x_height: self.x_height,
            cap_height: self.cap_height,
            underline_position: self.underline_position,
            underline_thickness: self.underline_thickness,
            strikeout_position: self.strikeout_position,
            strikeout_thickness: self.strikeout_thickness,
            weight_class: self.weight_class,
            width_class: self.width_class,
            italic_angle: self.italic_angle,
            is_fixed_pitch: self.spacing == Spacing::Normal,
        }
    }
}

/// A family file: one base plan and the faces it expands to.
///
/// r[impl family.schema]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FamilyDocument {
    pub family: Family,
}

/// The family header and its instances.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Family {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default = "default_vendor")]
    pub vendor: String,
    #[serde(default = "default_out")]
    pub out: String,
    #[serde(default = "default_formats")]
    pub formats: Vec<OutputFormat>,
    #[serde(default)]
    pub policy: CoveragePolicy,
    /// The Unicode ranges the host should capture. `None` means read them from
    /// the base capture.
    #[serde(default)]
    pub codepoints: Option<String>,
    /// A base capture, used for its codepoints. An instance that does not name
    /// its own capture is drawn by the host.
    #[serde(default)]
    pub capture: Option<String>,
    /// The plan keys every instance starts from.
    #[serde(default)]
    pub base: Value,
    /// A CJK source to bundle into every face.
    #[serde(default)]
    pub cjk: Option<FamilyCjk>,
    pub instances: BTreeMap<String, FamilyInstance>,
}

/// A CJK source shared by a family's faces.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FamilyCjk {
    /// A TrueType font to take CJK glyphs from.
    pub source: String,
    /// How many Latin cells a full-width CJK glyph occupies.
    #[serde(default = "default_cjk_scale")]
    pub scale: f64,
    /// Unicode ranges to take. Defaults to the CN set.
    #[serde(default)]
    pub ranges: Option<String>,
    /// A label appended to the family name when the faces are split by charset.
    #[serde(default)]
    pub label: Option<String>,
}

const fn default_cjk_scale() -> f64 {
    2.0
}

/// A CJK source with its path resolved against the family document.
#[derive(Clone, Debug)]
pub struct ResolvedCjk {
    pub source: std::path::PathBuf,
    pub scale: f64,
    pub ranges: Option<String>,
}

/// One face in a family.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FamilyInstance {
    #[serde(default)]
    pub weight: Option<f64>,
    #[serde(default)]
    pub slope: Option<f64>,
    #[serde(default)]
    pub width: Option<f64>,
    #[serde(default)]
    pub serifs: Option<String>,
    #[serde(default)]
    pub spacing: Option<String>,
    #[serde(default)]
    pub legacy_family: Option<String>,
    #[serde(default)]
    pub legacy: Option<String>,
    #[serde(default)]
    pub typographic_family: Option<String>,
    #[serde(default)]
    pub typographic: Option<String>,
    #[serde(default)]
    pub postscript: Option<String>,
    /// A capture at this instance's shape. Omit it to have the host build one.
    #[serde(default)]
    pub capture: Option<String>,
    /// Plan keys that apply only to this face.
    #[serde(default)]
    pub plan: Value,
}

/// A family instance with its plan and release names resolved.
#[derive(Clone, Debug)]
pub struct ResolvedInstance {
    pub id: String,
    pub plan: Plan,
    pub capture: Option<std::path::PathBuf>,
    pub legacy_family: String,
    pub legacy_subfamily: String,
    pub typographic_family: String,
    pub typographic_subfamily: String,
    pub postscript_name: String,
    pub cjk: Option<ResolvedCjk>,
}

/// What a family build does when a glyph is not ported.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CoveragePolicy {
    /// Fail when the port does not cover a glyph.
    Strict,
    /// Build, and report what came from the capture.
    #[default]
    Mixed,
    /// Fail when a capture is needed at all.
    PortOnly,
}

/// One file a family build can write.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Ttf,
    Woff2,
    Css,
}

fn default_version() -> String {
    "Version 1.000".to_string()
}

fn default_vendor() -> String {
    "LEMM".to_string()
}

fn default_out() -> String {
    "dist".to_string()
}

fn default_formats() -> Vec<OutputFormat> {
    vec![OutputFormat::Ttf, OutputFormat::Woff2, OutputFormat::Css]
}

/// The family file as JSON. The CLI reads TOML through the same serde types.
///
/// # Errors
///
/// Returns a message when the document does not match the family schema.
pub fn family_from_json(text: &str) -> Result<FamilyDocument, String> {
    serde_json::from_str(text).map_err(|error| error.to_string())
}

impl Family {
    /// Expands the base plan and the instances into plans and names.
    ///
    /// # Errors
    ///
    /// Returns a message for a bad plan value, a duplicate style name, or an
    /// output name that Iosevka reserves.
    // r[impl cjk.family]
    pub fn resolve(&self, config_dir: &Path) -> Result<Vec<ResolvedInstance>, String> {
        if self.name.trim().is_empty() {
            return Err("the family needs a name".to_string());
        }
        if self.instances.is_empty() {
            return Err(format!("family {} has no instances", self.name));
        }
        let base = if self.base.is_null() {
            Value::Object(serde_json::Map::new())
        } else {
            self.base.clone()
        };
        let display_name = self
            .cjk
            .as_ref()
            .and_then(|cjk| cjk.label.as_deref())
            .map_or_else(
                || self.name.clone(),
                |label| format!("{} {label}", self.name),
            );
        let cjk = self
            .cjk
            .as_ref()
            .map(|cjk| {
                if !cjk.scale.is_finite() || cjk.scale <= 0.0 {
                    return Err("family cjk scale must be positive".to_string());
                }
                let path = Path::new(&cjk.source);
                Ok(ResolvedCjk {
                    source: if path.is_absolute() {
                        path.to_path_buf()
                    } else {
                        config_dir.join(path)
                    },
                    scale: cjk.scale,
                    ranges: cjk.ranges.clone(),
                })
            })
            .transpose()?;
        let mut resolved = Vec::with_capacity(self.instances.len());
        let mut styles: BTreeMap<(String, String), String> = BTreeMap::new();
        let mut typographic_styles: BTreeMap<(String, String), String> = BTreeMap::new();
        let mut postscript: BTreeMap<String, String> = BTreeMap::new();

        for (id, instance) in &self.instances {
            if id.trim().is_empty() {
                return Err("an instance needs an id".to_string());
            }
            let mut value = base.clone();
            merge_json(&mut value, &instance.plan);
            let mut plan = plan_from_value(&value)?;
            if let Some(weight) = instance.weight {
                if !(1.0..=1000.0).contains(&weight) {
                    return Err(format!("{id}: weight must be between 1 and 1000"));
                }
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let weight = weight.round().clamp(1.0, 1000.0) as u16;
                plan.weight_class = weight;
            }
            if let Some(slope) = instance.slope {
                if !(-90.0..=90.0).contains(&slope) {
                    return Err(format!("{id}: slope must be between -90 and 90 degrees"));
                }
                plan.italic_angle = slope;
            }
            if let Some(width) = instance.width {
                if !(100.0..=2000.0).contains(&width) {
                    return Err(format!("{id}: width is in font units, so 500 is normal"));
                }
                plan.width = width;
                plan.width_class = width_class(width);
            }
            if let Some(serifs) = &instance.serifs {
                if serifs.trim().is_empty() {
                    return Err(format!("{id}: serifs cannot be empty"));
                }
                plan.serifs.clone_from(serifs);
            }
            if let Some(spacing) = &instance.spacing {
                plan.spacing = Spacing::from_name(spacing)?;
            }

            let italic = plan.italic_angle != 0.0;
            let bold = plan.weight_class >= 700;
            let width = width_label(plan.width_class);
            let weight = weight_label(plan.weight_class);
            let ribbi_weight = matches!(plan.weight_class, 400 | 700);

            let default_legacy_family = if ribbi_weight {
                width.map_or_else(
                    || display_name.clone(),
                    |width| format!("{display_name} {width}"),
                )
            } else {
                let mut parts = vec![display_name.clone()];
                if let Some(width) = width {
                    parts.push(width.to_string());
                }
                parts.push(weight.to_string());
                parts.join(" ")
            };
            let default_legacy_subfamily = if italic {
                if bold { "Bold Italic" } else { "Italic" }
            } else if bold {
                "Bold"
            } else {
                "Regular"
            };
            let default_typographic_subfamily = {
                let mut parts = Vec::new();
                if let Some(width) = width {
                    parts.push(width.to_string());
                }
                if plan.weight_class != 400 {
                    parts.push(weight.to_string());
                }
                if italic {
                    parts.push("Italic".to_string());
                }
                if parts.is_empty() {
                    "Regular".to_string()
                } else {
                    parts.join(" ")
                }
            };

            let legacy_family = instance
                .legacy_family
                .clone()
                .unwrap_or(default_legacy_family);
            let legacy_subfamily = instance
                .legacy
                .clone()
                .unwrap_or_else(|| default_legacy_subfamily.to_string());
            let typographic_family = instance
                .typographic_family
                .clone()
                .unwrap_or_else(|| display_name.clone());
            let typographic_subfamily = instance
                .typographic
                .clone()
                .unwrap_or(default_typographic_subfamily);
            let postscript_name = instance.postscript.clone().unwrap_or_else(|| {
                format!(
                    "{}-{}",
                    typographic_family.replace(' ', ""),
                    typographic_subfamily.replace(' ', "")
                )
            });

            if legacy_family.trim().is_empty()
                || legacy_subfamily.trim().is_empty()
                || typographic_family.trim().is_empty()
                || typographic_subfamily.trim().is_empty()
                || postscript_name.trim().is_empty()
            {
                return Err(format!("{id}: a family name cannot be empty"));
            }
            if is_reserved_family(&legacy_family) || is_reserved_family(&typographic_family) {
                return Err(format!(
                    "{id}: the family name {legacy_family} is reserved by Iosevka"
                ));
            }
            if let Some(other) = styles.insert(
                (legacy_family.clone(), legacy_subfamily.clone()),
                id.clone(),
            ) {
                return Err(format!(
                    "{id}: legacy style {legacy_family} {legacy_subfamily} is already used by {other}"
                ));
            }
            if let Some(other) = typographic_styles.insert(
                (typographic_family.clone(), typographic_subfamily.clone()),
                id.clone(),
            ) {
                return Err(format!(
                    "{id}: typographic style {typographic_family} {typographic_subfamily} is already used by {other}"
                ));
            }
            if let Some(other) = postscript.insert(postscript_name.clone(), id.clone()) {
                return Err(format!(
                    "{id}: PostScript name {postscript_name} is already used by {other}"
                ));
            }
            if postscript_name.contains(' ') {
                return Err(format!("{id}: a PostScript name cannot contain a space"));
            }

            let capture = instance.capture.as_ref().map(|path| {
                let path = Path::new(path);
                if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    config_dir.join(path)
                }
            });
            resolved.push(ResolvedInstance {
                id: id.clone(),
                plan,
                capture,
                legacy_family,
                legacy_subfamily,
                typographic_family,
                typographic_subfamily,
                postscript_name,
                cjk: cjk.clone(),
            });
        }

        Ok(resolved)
    }
}

fn plan_from_value(value: &Value) -> Result<Plan, String> {
    let text = serde_json::to_string(value).map_err(|error| error.to_string())?;
    from_json(&text)
}

fn merge_json(target: &mut Value, source: &Value) {
    if source.is_null() {
        return;
    }
    match (target, source) {
        (Value::Object(target), Value::Object(source)) => {
            for (key, value) in source {
                match target.get_mut(key) {
                    Some(existing) => merge_json(existing, value),
                    None => {
                        target.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (target, source) => *target = source.clone(),
    }
}

const fn weight_label(weight: u16) -> &'static str {
    match weight {
        0..=150 => "Thin",
        151..=250 => "ExtraLight",
        251..=350 => "Light",
        351..=425 => "Regular",
        426..=550 => "Medium",
        551..=650 => "SemiBold",
        651..=750 => "Bold",
        751..=850 => "ExtraBold",
        _ => "Black",
    }
}

const fn width_label(class: u16) -> Option<&'static str> {
    match class {
        1 => Some("UltraCondensed"),
        2 => Some("ExtraCondensed"),
        3 => Some("Condensed"),
        4 => Some("SemiCondensed"),
        5 => None,
        6 => Some("SemiExpanded"),
        7 => Some("Expanded"),
        8 => Some("ExtraExpanded"),
        _ => Some("UltraExpanded"),
    }
}

fn is_reserved_family(name: &str) -> bool {
    matches!(
        name,
        "Iosevka"
            | "Iosevka Aile"
            | "Iosevka Comfy"
            | "Iosevka Curly"
            | "Iosevka Etoile"
            | "Iosevka Fixed"
            | "Iosevka Slab"
            | "Iosevka Term"
    )
}
