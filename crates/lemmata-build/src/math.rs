//! The MATH table.
//!
//! r[impl math.table]
//!
//! The table has three parts: the constants a renderer reads to lay out an
//! equation, per-glyph information such as italic corrections, and the size
//! variants and assembly recipes a stretchy delimiter needs. Only the constants
//! are written so far. The other two are separate slices: the per-glyph
//! information needs coverage tables over math glyphs, and the variants need
//! glyphs that do not exist yet, since Iosevka ships no MATH table and so has
//! no size variants to inherit.
//!
//! The specification treats a constant of zero as "not specified, use the
//! renderer's default", so a profile only names the constants it has an opinion
//! about. Everything else stays zero and the renderer decides.
//!
//! The values here are plumbing, not typography. They follow from the font's own
//! measurements, which is enough to make the table exist and parse, and a design
//! pass over a reference math font is what would replace them.

#![allow(clippy::cast_possible_truncation)]

use write_fonts::tables::layout::CoverageTable;
use write_fonts::tables::math::{
    Math, MathConstants, MathGlyphConstruction, MathGlyphInfo, MathItalicsCorrectionInfo,
    MathTopAccentAttachment, MathValueRecord, MathVariants,
};
use write_fonts::types::{FWord, GlyphId16, UfWord};

use crate::font::{GlyphData, Metrics};

/// The constants the profile has an opinion about.
#[derive(Clone, Copy, Debug)]
pub struct MathProfile {
    /// How much smaller script-size text is drawn, as a percentage.
    pub script_percent_scale_down: i16,
    /// The same for script-script size.
    pub script_script_percent_scale_down: i16,
}

impl Default for MathProfile {
    fn default() -> Self {
        // The specification's suggested values. Both must be non-zero: a
        // renderer that scales by zero draws nothing.

        Self {
            script_percent_scale_down: 80,
            script_script_percent_scale_down: 60,
        }
    }
}

impl MathProfile {
    /// The constants that must not be zero for the table to be usable.
    #[must_use]
    pub const fn mandatory_nonzero() -> &'static [&'static str] {
        &[
            "script_percent_scale_down",
            "script_script_percent_scale_down",
            "axis_height",
        ]
    }
}

/// A constant in design units, with no device table.
fn value(units: i16) -> MathValueRecord {
    MathValueRecord {
        value: FWord::new(units),
        ..MathValueRecord::default()
    }
}

/// The constants, from the font's measurements and the profile.
#[must_use]
pub fn constants(metrics: &Metrics, profile: &MathProfile) -> MathConstants {
    let stroke = metrics.underline_thickness;
    MathConstants {
        script_percent_scale_down: profile.script_percent_scale_down,
        script_script_percent_scale_down: profile.script_script_percent_scale_down,
        // The math axis sits halfway up the x-height in every text face.
        axis_height: value(metrics.x_height / 2),
        // Accents stack from the cap height.
        accent_base_height: value(metrics.cap_height),
        flattened_accent_base_height: value(metrics.cap_height),
        // A rule is drawn at the stroke width, which the underline stands in
        // for until the plan carries one.
        fraction_rule_thickness: value(stroke),
        overbar_rule_thickness: value(stroke),
        underbar_rule_thickness: value(stroke),
        radical_rule_thickness: value(stroke),
        ..MathConstants::default()
    }
}

/// The glyphs a renderer stretches vertically: delimiters, the radical, the
/// large operators and the corners that bracket a matrix.
const STRETCHY: &[u32] = &[
    0x0028, 0x0029, // parentheses
    0x005B, 0x005D, // brackets
    0x007B, 0x007D, // braces
    0x221A, // radical
    0x222B, // integral
    0x2211, // summation
    0x220F, // product
    0x2308, 0x2309, 0x230A, 0x230B, // ceiling and floor
    0x27E8, 0x27E9, // angle brackets
];

/// The stretchy constructions, one per glyph.
///
/// Every one is empty for now: no size variants, and no assembly. The coverage
/// is what makes the table meaningful before the parts exist, since it names
/// the glyphs a renderer may ask to stretch.
#[must_use]
pub fn variants(metrics: &Metrics, glyphs: &[GlyphData]) -> MathVariants {
    let mut ids = Vec::new();
    for (index, glyph) in glyphs.iter().enumerate() {
        if !glyph.unicodes.iter().any(|code| STRETCHY.contains(code)) {
            continue;
        }
        let Ok(id) = u16::try_from(index + 1) else {
            continue;
        };
        ids.push(GlyphId16::new(id));
    }
    if ids.is_empty() {
        return MathVariants::default();
    }

    let constructions = ids
        .iter()
        .map(|_| {
            MathGlyphConstruction {
                glyph_assembly: None.into(),
                math_glyph_variant_records: Vec::new(),
            }
            .into()
        })
        .collect();

    MathVariants {
        // The pieces of an assembly join along this overlap, so it is the width
        // of a stroke: anything narrower leaves a seam and anything wider
        // doubles the ink where they meet.
        min_connector_overlap: UfWord::new(metrics.underline_thickness.unsigned_abs()),
        vert_glyph_coverage: Some(ids.iter().copied().collect::<CoverageTable>()).into(),
        vert_glyph_constructions: constructions,
        ..MathVariants::default()
    }
}

/// The MATH table.
///
/// `glyphs` is the font's glyph order, which is what the per-glyph coverage
/// tables index into.
#[must_use]
pub fn table(metrics: &Metrics, profile: &MathProfile, glyphs: &[GlyphData]) -> Math {
    Math::new(
        constants(metrics, profile),
        glyph_info(glyphs),
        variants(metrics, glyphs),
    )
}

/// The math alphanumerics block: the letters that take an italic correction and
/// an accent attachment.
const MATH_ALPHANUMERICS: std::ops::RangeInclusive<u32> = 0x1D400..=0x1D7FF;

/// A glyph's ink split at its midline: the horizontal extent of the upper half
/// and of the lower half.
///
/// Returns `None` for a glyph with no ink on one side of the midline.
fn ink_halves(glyph: &GlyphData) -> Option<((f64, f64), (f64, f64))> {
    let (_, y_min, _, y_max) = glyph.bounding_box()?;
    let midline = f64::from(y_min) + f64::from(y_max - y_min) / 2.0;
    let mut upper: Option<(f64, f64)> = None;
    let mut lower: Option<(f64, f64)> = None;
    for point in glyph.contours.iter().flatten() {
        let side = if point.y < midline {
            &mut lower
        } else {
            &mut upper
        };
        let entry = side.get_or_insert((point.x, point.x));
        entry.0 = entry.0.min(point.x);
        entry.1 = entry.1.max(point.x);
    }

    Some((upper?, lower?))
}

/// How far a slanted glyph's ink leans: the upper half's centre against the
/// lower half's.
///
/// An upright glyph is symmetric and measures zero, which the bold styles do
/// exactly. An italic glyph leans and measures positive. It is a proxy for the
/// design value, which is what a designer measures against a reference face,
/// and it is clamped to a quarter of the advance so that a letter whose lower
/// half is a tail, an italic `l` for instance, cannot report more than a
/// renderer could use.
fn italics_correction(glyph: &GlyphData) -> i16 {
    let Some((upper, lower)) = ink_halves(glyph) else {
        return 0;
    };
    let lean = f64::midpoint(upper.0, upper.1) - f64::midpoint(lower.0, lower.1);
    let limit = f64::from(glyph.advance) / 4.0;

    lean.clamp(0.0, limit).round() as i16
}

/// The horizontal midpoint of a glyph's upper ink, which is where a top accent
/// is centred.
fn top_accent_attachment(glyph: &GlyphData) -> i16 {
    let Some((upper, _)) = ink_halves(glyph) else {
        return (glyph.advance / 2).cast_signed();
    };
    f64::midpoint(upper.0, upper.1).round() as i16
}

/// The per-glyph information: italic corrections and top accent attachments for
/// the math letters.
///
/// `glyphs` is the font's glyph order, so a glyph's index plus one is its glyph
/// id, `.notdef` being zero.
///
/// r[impl math.corrections]
#[must_use]
pub fn glyph_info(glyphs: &[GlyphData]) -> MathGlyphInfo {
    let mut ids = Vec::new();
    let mut corrections = Vec::new();
    let mut attachments = Vec::new();
    for (index, glyph) in glyphs.iter().enumerate() {
        if !glyph
            .unicodes
            .iter()
            .any(|code| MATH_ALPHANUMERICS.contains(code))
        {
            continue;
        }
        let Ok(id) = u16::try_from(index + 1) else {
            continue;
        };
        ids.push(GlyphId16::new(id));
        corrections.push(value(italics_correction(glyph)));
        attachments.push(value(top_accent_attachment(glyph)));
    }
    if ids.is_empty() {
        return MathGlyphInfo::default();
    }
    let coverage: CoverageTable = ids.iter().copied().collect();

    MathGlyphInfo {
        math_italics_correction_info: Some(MathItalicsCorrectionInfo {
            coverage: coverage.clone().into(),
            italics_correction: corrections,
        })
        .into(),
        math_top_accent_attachment: Some(MathTopAccentAttachment {
            top_accent_coverage: coverage.into(),
            top_accent_attachment: attachments,
        })
        .into(),
        ..MathGlyphInfo::default()
    }
}
