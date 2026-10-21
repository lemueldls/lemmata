//! The MATH table.
//!
//! r[impl math.table]
//!
//! The table has three parts: the constants a renderer reads to lay out an
//! equation, per-glyph information such as italic corrections, and the size
//! variants and assembly recipes a stretchy delimiter needs. The glyphs the
//! last two index into do not come from upstream, which ships no MATH table and
//! so has no variants or assembly pieces to inherit: `crate::variants` and
//! `crate::assembly` derive them from the glyphs that do exist.
//!
//! The specification treats a constant of zero as "not specified, use the
//! renderer's default", so a profile only names the constants it has an opinion
//! about. Everything else stays zero and the renderer decides.
//!
//! The constants are derived rather than designed: the specification gives a
//! formula for most of them, and the rest are ratios of the x-height taken from
//! a reference math font. Leaving one at zero is not neutral, because a renderer
//! that reads it literally places scripts on the baseline and never grows a
//! display operator.

#![allow(clippy::cast_possible_truncation)]

use write_fonts::tables::layout::CoverageTable;
use write_fonts::tables::math::{
    GlyphAssembly, GlyphPartRecord, Math, MathConstants, MathGlyphConstruction, MathGlyphInfo,
    MathGlyphVariantRecord, MathItalicsCorrectionInfo, MathKern, MathKernInfo, MathKernInfoRecord,
    MathTopAccentAttachment, MathValueRecord, MathVariants, PartFlags,
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

/// Rounds a computed value into the 16-bit field the table stores.
fn units(value: f64) -> i16 {
    value
        .round()
        .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
}

/// The same, for the fields that are unsigned.
#[allow(clippy::cast_sign_loss)]
fn positive(value: f64) -> UfWord {
    UfWord::new(value.round().clamp(0.0, f64::from(u16::MAX)) as u16)
}

/// A constant in design units, with no device table.
fn value(units: i16) -> MathValueRecord {
    MathValueRecord {
        value: FWord::new(units),
        ..MathValueRecord::default()
    }
}

/// The constants, from the font's measurements and the profile.
///
/// A constant of zero means "not specified, use your own", and a renderer that
/// reads it literally then places scripts on the baseline and never grows a
/// display operator. So every constant the specification describes is written,
/// from two sources.
///
/// Most are multiples of the rule thickness, because the specification gives a
/// formula for them in those terms: a fraction's numerator sits one rule above
/// its bar and three in display style, a stack's gap is three rules and seven, a
/// radical's gap is one and a quarter. The rest are design values taken from New
/// Computer Modern Math, which is set at the same 1000 units per em, so its
/// numbers are ours as they stand. They are floors, not the last word: a
/// renderer also clears the base it is placing a script against, and that rule
/// reads this font's own x-height.
#[must_use]
pub fn constants(metrics: &Metrics, profile: &MathProfile) -> MathConstants {
    let em = f64::from(metrics.units_per_em);
    // The rule the specification measures its gaps against. A minus sign would
    // do as well; the underline is the stroke width the plan already carries.
    let rule = f64::from(metrics.underline_thickness.unsigned_abs());

    // A multiple of the rule thickness.
    let of_rule = |multiple: f64| value(units(rule * multiple));
    // A whole number of design units, which the reference font set at the same
    // em already carries.
    let at = |design: f64| value(units(design));

    MathConstants {
        script_percent_scale_down: profile.script_percent_scale_down,
        script_script_percent_scale_down: profile.script_script_percent_scale_down,
        // A delimited expression shorter than this is set as a sub-formula.
        delimited_sub_formula_min_height: positive(em * 1.3),
        // What a large operator grows to in display style. Reaching it is what
        // finds the derived size variants, and then the assembly. It sits above
        // the second scale so that a display operator picks the third: at two
        // times an operator is only as tall as the delimiters beside it, which
        // is what left the integral looking no taller than its own text size.
        display_operator_min_height: positive(em * 1.7),
        math_leading: at(154.0),
        axis_height: at(250.0),
        // The one pair the specification defines against this font rather than
        // against the rule: an accent base is the x-height plus overshoot, and
        // flattening starts at the cap height.
        accent_base_height: at(f64::from(metrics.x_height) * 1.044),
        flattened_accent_base_height: at(f64::from(metrics.cap_height)),
        subscript_shift_down: at(247.0),
        subscript_top_max: at(344.0),
        subscript_baseline_drop_min: at(200.0),
        superscript_shift_up: at(363.0),
        superscript_shift_up_cramped: at(289.0),
        superscript_bottom_min: at(108.0),
        superscript_baseline_drop_max: at(250.0),
        superscript_bottom_max_with_subscript: at(344.0),
        sub_superscript_gap_min: of_rule(4.0),
        space_after_script: at(56.0),
        upper_limit_gap_min: at(200.0),
        upper_limit_baseline_rise_min: at(111.0),
        lower_limit_gap_min: at(167.0),
        lower_limit_baseline_drop_min: at(600.0),
        stack_top_shift_up: at(444.0),
        stack_top_display_style_shift_up: at(677.0),
        stack_bottom_shift_down: at(345.0),
        stack_bottom_display_style_shift_down: at(686.0),
        stack_gap_min: of_rule(3.0),
        stack_display_style_gap_min: of_rule(7.0),
        stretch_stack_top_shift_up: at(111.0),
        stretch_stack_bottom_shift_down: at(600.0),
        stretch_stack_gap_above_min: at(200.0),
        stretch_stack_gap_below_min: at(167.0),
        fraction_numerator_shift_up: at(394.0),
        fraction_numerator_display_style_shift_up: at(677.0),
        fraction_denominator_shift_down: at(345.0),
        fraction_denominator_display_style_shift_down: at(686.0),
        fraction_numerator_gap_min: of_rule(1.0),
        fraction_num_display_style_gap_min: of_rule(3.0),
        fraction_rule_thickness: of_rule(1.0),
        fraction_denominator_gap_min: of_rule(1.0),
        fraction_denom_display_style_gap_min: of_rule(3.0),
        skewed_fraction_horizontal_gap: at(350.0),
        skewed_fraction_vertical_gap: at(96.0),
        overbar_vertical_gap: of_rule(3.0),
        overbar_rule_thickness: of_rule(1.0),
        overbar_extra_ascender: of_rule(1.0),
        underbar_vertical_gap: of_rule(3.0),
        underbar_rule_thickness: of_rule(1.0),
        underbar_extra_descender: of_rule(1.0),
        radical_vertical_gap: of_rule(1.25),
        radical_display_style_vertical_gap: at(f64::from(metrics.x_height).mul_add(0.25, rule)),
        radical_rule_thickness: of_rule(1.0),
        radical_extra_ascender: of_rule(1.0),
        radical_kern_before_degree: at(em * 5.0 / 18.0),
        radical_kern_after_degree: at(-em * 10.0 / 18.0),
        radical_degree_bottom_raise_percent: 60,
    }
}

/// The stretchy constructions, one per glyph: the derived sizes a renderer
/// picks from, and the assembly it builds when none is tall enough.
#[must_use]
pub fn variants(metrics: &Metrics, glyphs: &[GlyphData]) -> MathVariants {
    let stroke = metrics.underline_thickness.unsigned_abs();
    let (vert_ids, mut vert) = constructions(
        glyphs,
        crate::variants::STRETCHY,
        Axis::Vertical,
        |glyph| {
            // A large operator carries one more size than a delimiter, so the
            // list is the glyph's own rather than the delimiter's.
            let large = glyph
                .unicodes
                .iter()
                .any(|code| crate::variants::OPERATORS.contains(code));
            crate::variants::scales(large)
        },
        crate::variants::variant_name,
        stroke,
    );
    // A brace has a point in the middle, so no band of it can repeat. It gets
    // the five-part construction instead: two bars either side of the point.
    for (id, construction) in vert_ids.iter().zip(vert.iter_mut()) {
        let base = usize::from(id.to_u16()) - 1;
        if is_brace(&glyphs[base]) {
            construction.glyph_assembly = brace_assembly(&glyphs[base], glyphs, stroke).into();
        }
    }
    let (horiz_ids, horiz) = constructions(
        glyphs,
        crate::variants::HORIZONTAL,
        Axis::Horizontal,
        |_| crate::variants::HORIZONTAL_SCALES,
        crate::variants::horizontal_variant_name,
        stroke,
    );

    MathVariants {
        // The pieces of an assembly join along this overlap, so it is the width
        // of a stroke: anything narrower leaves a seam and anything wider
        // doubles the ink where they meet.
        min_connector_overlap: UfWord::new(stroke),
        vert_glyph_coverage: (!vert_ids.is_empty())
            .then(|| vert_ids.iter().copied().collect::<CoverageTable>())
            .into(),
        horiz_glyph_coverage: (!horiz_ids.is_empty())
            .then(|| horiz_ids.iter().copied().collect::<CoverageTable>())
            .into(),
        vert_glyph_constructions: vert.into_iter().map(Into::into).collect(),
        horiz_glyph_constructions: horiz.into_iter().map(Into::into).collect(),
    }
}

/// The direction a construction grows in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    Vertical,
    Horizontal,
}

/// One construction per glyph in `codes`: the derived sizes a renderer picks
/// from, and the assembly it builds when none is large enough.
fn constructions(
    glyphs: &[GlyphData],
    codes: &[u32],
    axis: Axis,
    scales: impl Fn(&GlyphData) -> &'static [f64],
    name: impl Fn(&str, usize) -> String,
    stroke: u16,
) -> (Vec<GlyphId16>, Vec<MathGlyphConstruction>) {
    let mut ids = Vec::new();
    let mut constructions = Vec::new();
    for (index, glyph) in glyphs.iter().enumerate() {
        if !glyph.unicodes.iter().any(|code| codes.contains(code)) {
            continue;
        }
        let Ok(id) = u16::try_from(index + 1) else {
            continue;
        };
        ids.push(GlyphId16::new(id));
        let records = scales(glyph)
            .iter()
            .enumerate()
            .filter_map(|(step, _)| {
                let variant = glyphs
                    .iter()
                    .position(|other| other.name == name(&glyph.name, step))?;
                Some(MathGlyphVariantRecord {
                    variant_glyph: GlyphId16::new(u16::try_from(variant + 1).ok()?),
                    advance_measurement: UfWord::new(extent(&glyphs[variant], axis)?),
                })
            })
            .collect();
        constructions.push(MathGlyphConstruction {
            glyph_assembly: assembly(glyph, glyphs, axis, stroke).into(),
            math_glyph_variant_records: records,
        });
    }

    (ids, constructions)
}

/// Whether a glyph is a brace, which needs the five-part assembly.
fn is_brace(glyph: &GlyphData) -> bool {
    glyph
        .unicodes
        .iter()
        .any(|code| *code == 0x007B || *code == 0x007D)
}

/// The assembly recipe for a brace: lower hook, lower bar, point, upper bar,
/// upper hook.
///
/// Both bars carry the extender flag, which is how a renderer grows a brace
/// around its point instead of on one side of it. The reference math fonts
/// write the same five parts.
fn brace_assembly(base: &GlyphData, glyphs: &[GlyphData], stroke: u16) -> Option<GlyphAssembly> {
    let last = crate::assembly::BRACE_PIECES.len() - 1;
    let mut part_records = Vec::new();
    for index in 0..crate::assembly::BRACE_PIECES.len() {
        let name = crate::assembly::brace_piece_name(&base.name, index);
        let position = glyphs.iter().position(|glyph| glyph.name == name)?;
        let full = extent(&glyphs[position], Axis::Vertical)?;
        let overlap = stroke.min(full / 2);
        let part_flags = if crate::assembly::BRACE_EXTENDERS.contains(&index) {
            PartFlags::EXTENDER_FLAG
        } else {
            PartFlags::empty()
        };
        part_records.push(GlyphPartRecord {
            glyph_id: GlyphId16::new(u16::try_from(position + 1).ok()?),
            start_connector_length: UfWord::new(if index == 0 { 0 } else { overlap }),
            end_connector_length: UfWord::new(if index == last { 0 } else { overlap }),
            full_advance: UfWord::new(full),
            part_flags,
        });
    }

    Some(GlyphAssembly {
        italics_correction: value(0),
        part_records,
    })
}

/// The size a variant grew to: its height for a vertical construction, its
/// width for a horizontal one.
fn extent(glyph: &GlyphData, axis: Axis) -> Option<u16> {
    let (x_min, y_min, x_max, y_max) = glyph.bounding_box()?;
    let size = match axis {
        Axis::Vertical => y_max - y_min,
        Axis::Horizontal => x_max - x_min,
    };
    u16::try_from(size).ok()
}

/// The assembly recipe for one stretchy glyph: its three pieces along the axis
/// it grows in, so bottom to top or left to right.
///
/// The pieces are bands of the glyph, so each one's ink already spans its band
/// and the only thing left to say is how far they overlap. Every connector is
/// the stroke width, which is the value the table gives as the minimum overlap
/// and roughly the bar material a band has to spare at its edge.
fn assembly(
    base: &GlyphData,
    glyphs: &[GlyphData],
    axis: Axis,
    stroke: u16,
) -> Option<GlyphAssembly> {
    let last = crate::assembly::PIECES.len() - 1;
    let mut part_records = Vec::new();
    for index in 0..crate::assembly::PIECES.len() {
        let name = match axis {
            Axis::Vertical => crate::assembly::piece_name(&base.name, index),
            Axis::Horizontal => crate::assembly::horizontal_piece_name(&base.name, index),
        };
        let position = glyphs.iter().position(|glyph| glyph.name == name)?;
        let full = extent(&glyphs[position], axis)?;
        // The spec requires the two connectors to fit inside the part, which
        // a short band would otherwise break.
        let overlap = stroke.min(full / 2);
        let part_flags = if index == crate::assembly::EXTENDER {
            PartFlags::EXTENDER_FLAG
        } else {
            PartFlags::empty()
        };
        part_records.push(GlyphPartRecord {
            glyph_id: GlyphId16::new(u16::try_from(position + 1).ok()?),
            // The start of a part is the side the assembly begins at, so the
            // piece that lands first has nothing before it to join to.
            start_connector_length: UfWord::new(if index == 0 { 0 } else { overlap }),
            end_connector_length: UfWord::new(if index == last { 0 } else { overlap }),
            full_advance: UfWord::new(full),
            part_flags,
        });
    }

    Some(GlyphAssembly {
        // An assembly's correction does not depend on its size, and the pieces
        // lean with the glyph rather than past it.
        italics_correction: value(0),
        part_records,
    })
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

/// Whether a glyph is a letter, which is what takes an italic correction and a
/// top accent attachment.
///
/// This is every letter the font carries rather than the math alphanumerics
/// alone. A renderer sets a plain Latin or Greek letter in math as readily as
/// it reaches for U+1D465, so covering only that block leaves the letters it
/// actually uses without corrections.
fn is_letter(code: u32) -> bool {
    char::from_u32(code).is_some_and(char::is_alphabetic)
}

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

/// How many heights the top-corner ink is sampled at.
///
/// The table stores one band per change, so the samples only have to find the
/// heights where the outline's edge moves. Six cuts catch an italic letter's
/// hook without writing a table of noise.
const KERN_SAMPLES: usize = 6;

/// The right or left edge of a glyph's ink at a height, within a small band.
fn ink_edge_at(glyph: &GlyphData, y: f64, left: bool) -> Option<f64> {
    let mut edge = if left {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    };
    let mut found = false;
    for point in glyph.contours.iter().flatten() {
        if (point.y - y).abs() > 3.0 {
            continue;
        }
        found = true;
        if left {
            edge = edge.min(point.x);
        } else {
            edge = edge.max(point.x);
        }
    }

    found.then_some(edge)
}

/// The corner kern for a glyph's top right or top left, or `None` when the ink
/// never leaves the advance.
///
/// A renderer places a script at the top of a letter against the letter's
/// advance. The ink can leave that box: an italic stem leans past the right
/// edge, and a hook or a swash can reach left of the origin. The table records
/// that overhang at the heights where it changes, so a renderer can tuck a
/// script under the lean or push it clear of the hook. Both are zero for an
/// upright letter that stays inside its box, which is why most glyphs get no
/// table at all.
fn top_kern(glyph: &GlyphData, left: bool) -> Option<MathKern> {
    let (_, y_min, _, y_max) = glyph.bounding_box()?;
    let low = f64::from(y_min).max(0.0);
    let high = f64::from(y_max);
    if high <= low {
        return None;
    }
    let advance = f64::from(glyph.advance);
    let mut heights = Vec::new();
    let mut kerns: Vec<i16> = Vec::new();
    for step in 0..=KERN_SAMPLES {
        #[allow(clippy::cast_precision_loss)]
        let y = low + (high - low) * step as f64 / KERN_SAMPLES as f64;
        let Some(edge) = ink_edge_at(glyph, y, left) else {
            continue;
        };
        let overhang = if left {
            edge.min(0.0)
        } else {
            (edge - advance).max(0.0)
        };
        let kern = units(overhang);
        if kerns.last().copied() == Some(kern) {
            continue;
        }
        if !kerns.is_empty() {
            heights.push(units(y));
        }
        kerns.push(kern);
    }
    if kerns.iter().all(|kern| *kern == 0) {
        return None;
    }

    Some(MathKern {
        correction_height: heights.into_iter().map(value).collect(),
        kern_values: kerns.into_iter().map(value).collect(),
    })
}

/// The glyphs a renderer positions against their ink box rather than the
/// baseline: the stretchy glyphs and the sizes derived from them.
///
/// This matters as soon as a glyph can be taller than the line. Beside a
/// parenthesis grown to enclose a fraction, a superscript belongs on the line
/// the rest of the equation sits on, not on the baseline the parenthesis
/// happens to start at.
fn extended_shapes(glyphs: &[GlyphData]) -> Vec<GlyphId16> {
    // A large operator carries one more size than a delimiter, so the set the
    // coverage names is the union rather than one glyph's list. A horizontal
    // arrow has fewer sizes than either.
    let vertical_steps = crate::variants::OPERATOR_SCALES
        .len()
        .max(crate::variants::SCALES.len());
    let bases: Vec<&GlyphData> = glyphs
        .iter()
        .filter(|glyph| {
            glyph.unicodes.iter().any(|code| {
                crate::variants::STRETCHY.contains(code)
                    || crate::variants::HORIZONTAL.contains(code)
            })
        })
        .collect();
    let mut ids = Vec::new();
    for (index, glyph) in glyphs.iter().enumerate() {
        let base = glyph.unicodes.iter().any(|code| {
            crate::variants::STRETCHY.contains(code) || crate::variants::HORIZONTAL.contains(code)
        });
        let derived = bases.iter().any(|base| {
            (0..vertical_steps).any(|step| {
                crate::variants::variant_name(&base.name, step) == glyph.name
                    || crate::variants::horizontal_variant_name(&base.name, step) == glyph.name
            })
        });
        if !base && !derived {
            continue;
        }
        let Ok(id) = u16::try_from(index + 1) else {
            continue;
        };
        ids.push(GlyphId16::new(id));
    }

    ids
}

/// The per-glyph information: italic corrections and top accent attachments for
/// the math letters, and the coverage of the glyphs a renderer may stretch.
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
        if !glyph.unicodes.iter().any(|code| is_letter(*code)) {
            continue;
        }
        let Ok(id) = u16::try_from(index + 1) else {
            continue;
        };
        ids.push(GlyphId16::new(id));
        corrections.push(value(italics_correction(glyph)));
        attachments.push(value(top_accent_attachment(glyph)));
    }
    let coverage: CoverageTable = ids.iter().copied().collect();
    let extended = extended_shapes(glyphs);

    let mut kern_ids = Vec::new();
    let mut kern_records = Vec::new();
    for (index, glyph) in glyphs.iter().enumerate() {
        if !glyph.unicodes.iter().any(|code| is_letter(*code)) {
            continue;
        }
        let right = top_kern(glyph, false);
        let left = top_kern(glyph, true);
        if right.is_none() && left.is_none() {
            continue;
        }
        let Ok(id) = u16::try_from(index + 1) else {
            continue;
        };
        kern_ids.push(GlyphId16::new(id));
        kern_records.push(MathKernInfoRecord::new(right, left, None, None));
    }
    let kern_info = (!kern_ids.is_empty()).then(|| {
        MathKernInfo::new(
            kern_ids.iter().copied().collect::<CoverageTable>(),
            kern_records,
        )
    });

    MathGlyphInfo {
        math_italics_correction_info: (!ids.is_empty())
            .then(|| MathItalicsCorrectionInfo {
                coverage: coverage.clone().into(),
                italics_correction: corrections,
            })
            .into(),
        math_top_accent_attachment: (!ids.is_empty())
            .then(|| MathTopAccentAttachment {
                top_accent_coverage: coverage.into(),
                top_accent_attachment: attachments,
            })
            .into(),
        extended_shape_coverage: (!extended.is_empty())
            .then(|| extended.into_iter().collect::<CoverageTable>())
            .into(),
        math_kern_info: kern_info.into(),
    }
}
