//! The plan: what a caller chooses about the font.
//!
//! A first slice, and a narrow one on purpose. The plan carries the values the
//! writer puts in `head`, `hhea` and `OS/2`, which is the seam
//! [`lemmata_build::font::build`] takes. It does not reach the outlines: those
//! come from the glyph source, and until that responds to a parameter the
//! shapes are the ones recorded in the capture.
//!
//! The default is Iosevka's vertical metrics at 1000 units per em, so an empty
//! plan reproduces the font this project built before the plan existed. That is
//! the invariant `tests/default.rs` pins.
//!
//! The boundary format is JSON, because the page sends an object and
//! `serde_json` is already in the tree. The struct is the part a file format
//! would be an adapter over, and the keys are Iosevka's, so a plan written for
//! one tool reads in the other for the fields both have.

use lemmata_build::font::Metrics;
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
    /// frame rather than a rounded one. 500 is the normal face.
    pub width: f64,
    /// The OS/2 width class, which follows from the grade unless a plan sets it
    /// on its own.
    pub width_class: u16,
    pub italic_angle: f64,
    /// The spacing the outlines were drawn at.
    pub spacing: Spacing,
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
            width: 500.0,
            width_class: 5,
            italic_angle: 0.0,
            spacing: Spacing::Normal,
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
            Some(value) => value
                .as_i64()
                .map(Some)
                .ok_or_else(|| format!("{key} must be a whole number")),
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
        plan.spacing = match spacing.as_str() {
            Some("normal") => Spacing::Normal,
            Some("quasi-proportional") => Spacing::QuasiProportional,
            _ => return Err("spacing must be \"normal\" or \"quasi-proportional\"".to_string()),
        };
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
    /// The values the writer puts in the font tables.
    #[must_use]
    pub const fn to_font_metrics(&self) -> Metrics {
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
        }
    }
}
