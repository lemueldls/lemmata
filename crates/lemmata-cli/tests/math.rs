//! The MATH table over the math glyphs.
//!
//! The capture this reads is separate from the coverage one, because it holds a
//! different set: the math alphanumerics and the operators, rather than the
//! Latin letters, digits and punctuation the outline tests are calibrated on.
//! Skips when it is absent.

use std::path::{Path, PathBuf};

use lemmata_build::font::{GlyphData, Metrics, Names};
use lemmata_capture::Glyph;
use read_fonts::tables::math::StretchAxis;
use read_fonts::types::GlyphId;
use read_fonts::{FontRef, TableProvider};

/// The math alphanumerics block, which is what `r[math.corrections]` covers.
const MATH_ALPHANUMERICS: std::ops::RangeInclusive<u32> = 0x1D400..=0x1D7FF;

fn math_dump_path() -> PathBuf {
    std::env::var_os("LEMMATA_MATH_DUMP").map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("target/math.bin")
        },
        PathBuf::from,
    )
}

/// Rounds a captured advance to the units the font table stores.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn advance_to_u16(advance: f64) -> u16 {
    advance.round().clamp(0.0, f64::from(u16::MAX)) as u16
}

/// Builds a font from the math capture.
fn build_math_font() -> Option<(Vec<u8>, Vec<Glyph>)> {
    let data = std::fs::read(math_dump_path()).ok()?;
    let captured = lemmata_capture::load(&data).expect("decode the math capture");
    let glyphs: Vec<GlyphData> = captured
        .iter()
        .map(|glyph| GlyphData {
            name: glyph.name.clone(),
            unicodes: glyph.unicodes.clone(),
            advance: advance_to_u16(glyph.advance),
            contours: lemmata_capture::contours(glyph),
        })
        .collect();
    let bytes = lemmata_build::font::build(&glyphs, &Metrics::default(), &Names::default())
        .expect("build the font");

    Some((bytes, captured))
}

/// The styles `r[math.alphabets]` names, as the ranges they occupy in the math
/// alphanumerics block.
///
/// Iosevka implements all of them except the script pair. That gap is in the
/// glyph source rather than in the table, so the test pins it instead of hiding
/// it: the day the source grows script letters, the assertion flips and the
/// requirement can be closed.
const STYLES: &[(&str, u32, u32)] = &[
    ("bold", 0x1D400, 0x1D433),
    ("italic", 0x1D434, 0x1D467),
    ("bold italic", 0x1D468, 0x1D49B),
    ("script", 0x1D49C, 0x1D4CF),
    ("fraktur", 0x1D504, 0x1D537),
    ("double-struck", 0x1D538, 0x1D56B),
    ("sans-serif", 0x1D5A0, 0x1D5D3),
    ("monospace", 0x1D670, 0x1D6A3),
];

/// `r[math.alphabets]`: every named style is in the font and reachable, apart
/// from script, which the glyph source does not have.
#[test]
fn math_alphabets_are_present() {
    let Some((bytes, _)) = build_math_font() else {
        eprintln!("skipping: no math capture");
        return;
    };
    let font = FontRef::new(&bytes).expect("reopen the font");
    let cmap = font.cmap().expect("the font has a cmap");

    let mut present = Vec::new();
    let mut missing = Vec::new();
    for (style, low, high) in STYLES {
        let mapped = (*low..=*high)
            .filter(|code| cmap.map_codepoint(*code).is_some())
            .count();
        if mapped == 0 {
            missing.push(*style);
        } else {
            present.push(*style);
        }
    }

    assert_eq!(
        missing,
        ["script"],
        "the only style the glyph source lacks should be script"
    );
    assert_eq!(present.len(), STYLES.len() - 1);
    eprintln!(
        "math alphabets: {} of {} styles present, missing {missing:?}",
        present.len(),
        STYLES.len()
    );
}

/// `r[math.corrections]`: italic corrections and top accent positions exist for
/// every math letter, and are inside the glyph.
#[test]
fn math_letters_have_corrections_and_accents() {
    let Some((bytes, captured)) = build_math_font() else {
        eprintln!("skipping: no math capture");
        return;
    };
    let font = FontRef::new(&bytes).expect("reopen the font");
    let math = font.math().expect("the font has a MATH table");
    let info = math.math_glyph_info().expect("MathGlyphInfo");
    let corrections = info
        .math_italics_correction_info()
        .expect("italics corrections")
        .expect("the table resolves");
    let attachments = info
        .math_top_accent_attachment()
        .expect("top accent attachments")
        .expect("the table resolves");

    let mut covered = 0usize;
    let mut italic_total = 0i64;
    let mut italic_count = 0i64;
    let mut upright_total = 0i64;
    let mut upright_count = 0i64;
    for (index, glyph) in captured.iter().enumerate() {
        if !glyph
            .unicodes
            .iter()
            .any(|code| MATH_ALPHANUMERICS.contains(code))
        {
            continue;
        }
        // The font's glyph order is the capture's, with `.notdef` first.
        let id = GlyphId::new(u32::try_from(index + 1).expect("glyph id"));
        let correction = corrections
            .correction(id)
            .unwrap_or_else(|| panic!("{} has no italic correction", glyph.name));
        let attachment = attachments
            .attachment(id)
            .unwrap_or_else(|| panic!("{} has no top accent attachment", glyph.name));

        covered += 1;
        let advance = i32::from(advance_to_u16(glyph.advance));
        assert!(
            (0..=advance / 4).contains(&correction),
            "{}: correction {correction} is outside the quarter advance a renderer can use",
            glyph.name
        );
        assert!(
            (0..advance).contains(&attachment),
            "{}: attachment {attachment} is outside the advance",
            glyph.name
        );

        // The italic styles are the second block of the Latin letters, the
        // bold ones the first. A correction that cannot tell them apart is not
        // measuring a slant.

        match glyph.unicodes[0] {
            0x1D434..=0x1D467 => {
                italic_total += i64::from(correction);
                italic_count += 1;
            }
            0x1D400..=0x1D433 => {
                upright_total += i64::from(correction);
                upright_count += 1;
            }
            _ => {}
        }
    }

    assert_eq!(covered, 904, "every math alphanumeric is covered");
    let italic_mean = italic_total / italic_count.max(1);
    let upright_mean = upright_total / upright_count.max(1);
    assert!(
        italic_mean > upright_mean,
        "the italic letters average {italic_mean} and the upright ones {upright_mean}, \
         so the correction is not measuring a slant"
    );
    // The upright block is not perfectly symmetric, so its mean is small rather
    // than zero. What matters is that the slant dominates it.
    assert!(
        upright_mean * 3 < italic_mean,
        "the upright block averages {upright_mean}, too close to the italic {italic_mean}"
    );
    eprintln!(
        "math: {covered} letters covered, italic mean {italic_mean}, upright mean {upright_mean}"
    );
}

/// The glyphs a renderer stretches vertically, by codepoint.
const STRETCHY: &[u32] = &[
    0x0028, 0x0029, 0x005B, 0x005D, 0x007B, 0x007D, 0x221A, 0x222B, 0x2211, 0x220F, 0x2308, 0x2309,
    0x230A, 0x230B, 0x27E8, 0x27E9,
];

/// `r[math.stretch]` as far as it goes: the stretchy glyphs are named in the
/// table, and the overlap its assemblies will join along is the stroke width.
///
/// The size variants and the assemblies themselves are not written yet. Iosevka
/// ships no MATH table, so it has no size variants to inherit, and a font with
/// no MATH table has no reason to draw three widths of a parenthesis.
#[test]
fn stretchy_glyphs_are_covered() {
    let Some((bytes, captured)) = build_math_font() else {
        eprintln!("skipping: no math capture");
        return;
    };
    let font = FontRef::new(&bytes).expect("reopen the font");
    let variants = font
        .math()
        .expect("MATH")
        .math_variants()
        .expect("MathVariants");

    assert_eq!(
        variants.min_connector_overlap(),
        read_fonts::types::UfWord::new(Metrics::default().underline_thickness.unsigned_abs()),
        "the overlap an assembly joins along is the stroke width"
    );

    let mut covered = 0usize;
    let mut missing = Vec::new();
    for (index, glyph) in captured.iter().enumerate() {
        if !glyph.unicodes.iter().any(|code| STRETCHY.contains(code)) {
            continue;
        }
        let id = GlyphId::new(u32::try_from(index + 1).expect("glyph id"));
        if variants
            .glyph_construction(id, StretchAxis::Vertical)
            .is_some()
        {
            covered += 1;
        } else {
            missing.push(glyph.name.clone());
        }
    }

    assert!(
        missing.is_empty(),
        "the font has these but the table does not name them: {missing:?}"
    );
    // Every stretchy codepoint is in the font. This counted twelve until the
    // capture set grew to carry the brackets and the braces: the assertion
    // named the gap, so the day it closed was visible.
    let absent: Vec<u32> = STRETCHY
        .iter()
        .copied()
        .filter(|code| !captured.iter().any(|glyph| glyph.unicodes.contains(code)))
        .collect();
    assert!(
        absent.is_empty(),
        "the font lacks these stretchy glyphs: {absent:?}"
    );
    assert_eq!(covered, STRETCHY.len(), "every stretchy glyph is covered");
    eprintln!("stretchy: {covered} of {} covered", STRETCHY.len());
}
