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

use write_fonts::tables::math::{
    Math, MathConstants, MathGlyphInfo, MathValueRecord, MathVariants,
};
use write_fonts::types::FWord;

use crate::font::Metrics;

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

/// The MATH table.
#[must_use]
pub fn table(metrics: &Metrics, profile: &MathProfile) -> Math {
    Math::new(
        constants(metrics, profile),
        MathGlyphInfo::default(),
        MathVariants::default(),
    )
}
