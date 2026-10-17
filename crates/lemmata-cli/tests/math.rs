//! The MATH table over the math glyphs.
//!
//! The capture this reads is separate from the coverage one, because it holds a
//! different set: the math alphanumerics and the operators, rather than the
//! Latin letters, digits and punctuation the outline tests are calibrated on.
//! Skips when it is absent.

use std::path::{Path, PathBuf};

use lemmata_build::font::{GlyphData, Metrics, Names};
use lemmata_capture::Glyph;
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
