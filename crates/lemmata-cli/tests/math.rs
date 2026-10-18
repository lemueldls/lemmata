//! The MATH table over the math glyphs.
//!
//! The capture this reads is separate from the coverage one, because it holds a
//! different set: the math alphanumerics and the operators, rather than the
//! Latin letters, digits and punctuation the outline tests are calibrated on.
//! Skips when it is absent.

use std::path::{Path, PathBuf};

use lemmata_build::font::{GlyphData, Metrics, Names};
use lemmata_capture::Glyph;
use read_fonts::tables::glyf::Glyph as ReadGlyph;
use read_fonts::tables::math::{PartFlags, StretchAxis};
use read_fonts::types::GlyphId;
use read_fonts::{FontRef, TableProvider};

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
            .any(|code| char::from_u32(*code).is_some_and(char::is_alphabetic))
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

    // Every letter the font carries, not the math alphanumerics alone: a
    // renderer sets a plain Latin or Greek letter in math too.
    let letters = captured
        .iter()
        .filter(|glyph| {
            glyph
                .unicodes
                .iter()
                .any(|code| char::from_u32(*code).is_some_and(char::is_alphabetic))
        })
        .count();
    assert_eq!(covered, letters, "every letter is covered");
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

/// `r[math.stretch]`: the stretchy glyphs are named in the table, and the
/// overlap an assembly joins along is the stroke width.
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

    // Each construction names its derived sizes, and a derived size is taller
    // than the glyph it came from. That last check is the one a rendering
    // harness cannot make: a renderer with no variant draws the base glyph at
    // the wrong scale rather than falling back, so the font names stay clean.
    let loca = font.loca(None).expect("loca");
    let glyf = font.glyf().expect("glyf");
    let height = |id: GlyphId| -> i32 {
        let glyph = loca
            .get_glyf(id, &glyf)
            .expect("read a glyph")
            .unwrap_or_else(|| panic!("glyph {id} has no outline"));
        let ReadGlyph::Simple(simple) = glyph else {
            panic!("a stretchy glyph is a simple glyph");
        };
        let mut low = i32::MAX;
        let mut high = i32::MIN;
        for point in simple.points() {
            low = low.min(i32::from(point.y));
            high = high.max(i32::from(point.y));
        }
        high - low
    };

    let mut derived = 0usize;
    for (index, glyph) in captured.iter().enumerate() {
        if !glyph.unicodes.iter().any(|code| STRETCHY.contains(code)) {
            continue;
        }
        let id = GlyphId::new(u32::try_from(index + 1).expect("glyph id"));
        let construction = variants
            .glyph_construction(id, StretchAxis::Vertical)
            .unwrap_or_else(|| panic!("{} has no construction", glyph.name));
        let records = construction.math_glyph_variant_records();
        assert_eq!(
            records.len(),
            lemmata_build::variants::SCALES.len(),
            "{} should name one derived size per scale",
            glyph.name
        );
        let base = height(id);
        for record in records {
            let variant = record.variant_glyph();
            assert_ne!(variant, id, "a variant is not the base glyph");
            let grown = height(GlyphId::new(u32::from(variant)));
            assert!(
                grown > base,
                "{}'s derived size is {grown} tall against a base of {base}",
                glyph.name
            );
            derived += 1;
        }
    }
    assert_eq!(
        derived,
        STRETCHY.len() * lemmata_build::variants::SCALES.len()
    );
    eprintln!("stretchy: {covered} glyphs covered, {derived} derived sizes");
}

/// `r[math.stretch]`: every stretchy glyph has an assembly whose pieces tile the
/// glyph's height, so a renderer stacking them covers it without a gap.
#[test]
fn stretchy_glyphs_have_assemblies() {
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
    let tolerance = i32::from(u16::from(variants.min_connector_overlap()));
    let loca = font.loca(None).expect("loca");
    let glyf = font.glyf().expect("glyf");

    // The ink's bounding box, read back from the font rather than from the
    // capture, so the check covers what the writer actually stored.
    let span = |id: GlyphId| -> (i32, i32, i32, i32) {
        let glyph = loca
            .get_glyf(id, &glyf)
            .expect("read a glyph")
            .unwrap_or_else(|| panic!("glyph {id} has no outline"));
        let ReadGlyph::Simple(simple) = glyph else {
            panic!("a stretchy piece is a simple glyph");
        };
        let (mut x_min, mut y_min) = (i32::MAX, i32::MAX);
        let (mut x_max, mut y_max) = (i32::MIN, i32::MIN);
        for point in simple.points() {
            x_min = x_min.min(i32::from(point.x));
            y_min = y_min.min(i32::from(point.y));
            x_max = x_max.max(i32::from(point.x));
            y_max = y_max.max(i32::from(point.y));
        }

        (x_min, y_min, x_max, y_max)
    };

    let mut assembled = 0usize;
    let mut worst = 0i32;
    let mut without: Vec<u32> = Vec::new();
    for (index, glyph) in captured.iter().enumerate() {
        if !glyph.unicodes.iter().any(|code| STRETCHY.contains(code)) {
            continue;
        }
        let id = GlyphId::new(u32::try_from(index + 1).expect("glyph id"));
        let construction = variants
            .glyph_construction(id, StretchAxis::Vertical)
            .unwrap_or_else(|| panic!("{} has no construction", glyph.name));
        let Some(resolved) = construction.glyph_assembly() else {
            without.push(glyph.unicodes[0]);
            continue;
        };
        let assembly = resolved.unwrap_or_else(|error| panic!("{}: {error:?}", glyph.name));
        let parts = assembly.part_records();
        assert_eq!(
            parts.len(),
            lemmata_build::assembly::PIECES.len(),
            "{} should have one piece per band",
            glyph.name
        );

        // Stack the parts the way a renderer does: the bottom one first, each
        // later one overlapping the one below it by the smaller of the two
        // connectors.
        let mut bottom = 0i32;
        let mut below: Option<(i32, i32, i32)> = None;
        let mut extenders = 0usize;
        let mut joins = 0i32;
        for (position, part) in parts.iter().enumerate() {
            let (x_min, y_min, x_max, y_max) =
                span(GlyphId::new(u32::from(part.glyph_id().to_u16())));
            let full = i32::from(u16::from(part.full_advance()));
            let start = i32::from(u16::from(part.start_connector_length()));
            let end = i32::from(u16::from(part.end_connector_length()));
            // A part's ink spans its advance, which is what lets the advances
            // tiling stand in for the pieces joining.
            assert_eq!(
                y_min, 0,
                "{} piece {position} does not start on the baseline",
                glyph.name
            );
            assert_eq!(
                y_max, full,
                "{} piece {position} spans {y_max} against an advance of {full}",
                glyph.name
            );
            assert!(
                start + end <= full,
                "{} piece {position} has connectors wider than the part",
                glyph.name
            );
            if part.part_flags().contains(PartFlags::EXTENDER_FLAG) {
                extenders += 1;
                assert_eq!(
                    position,
                    lemmata_build::assembly::EXTENDER,
                    "{} repeats a piece that is not the middle one",
                    glyph.name
                );
            }

            if let Some((below_x_min, below_x_max, below_end)) = below {
                // The join is only a join if the two parts' ink crosses.
                assert!(
                    x_min < below_x_max && below_x_min < x_max,
                    "{} piece {position} does not cross the one below it",
                    glyph.name
                );
                let overlap = below_end.min(start);
                assert!(
                    overlap > 0,
                    "{} piece {position} does not overlap the one below it",
                    glyph.name
                );
                assert!(
                    overlap <= tolerance,
                    "{} piece {position} overlaps by {overlap}, past the {tolerance} the table allows",
                    glyph.name
                );
                joins += overlap;
                bottom += full - overlap;
            } else {
                bottom = full;
            }
            below = Some((x_min, x_max, end));
        }

        assert_eq!(
            extenders, 1,
            "{} should repeat exactly one piece",
            glyph.name
        );

        // The repeat seam, which is what a derived extender can get wrong. The
        // middle piece is a band of a curved glyph, so the ink along its bottom
        // edge need not match the ink along its top edge, and every repeat
        // steps by that much. The band is narrow for this reason, and this is
        // the check that it stayed narrow enough.
        let repeat = &parts[lemmata_build::assembly::EXTENDER];
        let full = i32::from(u16::from(repeat.full_advance()));
        let piece = loca
            .get_glyf(GlyphId::new(u32::from(repeat.glyph_id().to_u16())), &glyf)
            .expect("read a glyph")
            .expect("the extender has an outline");
        let ReadGlyph::Simple(simple) = piece else {
            panic!("a stretchy piece is a simple glyph");
        };
        let (mut low_min, mut low_max) = (i32::MAX, i32::MIN);
        let (mut high_min, mut high_max) = (i32::MAX, i32::MIN);
        for point in simple.points() {
            let (x, y) = (i32::from(point.x), i32::from(point.y));
            if y <= 0 {
                low_min = low_min.min(x);
                low_max = low_max.max(x);
            }
            if y >= full {
                high_min = high_min.min(x);
                high_max = high_max.max(x);
            }
        }
        let seam = (high_min - low_min).abs().max((high_max - low_max).abs());
        assert!(
            seam <= tolerance,
            "{}'s extender steps {seam} where it repeats, past the {tolerance} the table allows",
            glyph.name
        );
        worst = worst.max(seam);

        let (_, base_low, _, base_high) = span(id);
        let height = base_high - base_low;
        assert!(
            height - bottom <= joins,
            "{} assembles to {bottom} against a glyph {height} tall",
            glyph.name
        );
        assembled += 1;
    }

    // A glyph whose middle is not a bar has no band that repeats: a summation's
    // middle holds its crossbar, and an angle bracket's holds its vertex, so the
    // piece would stack the feature rather than a stroke. They need a drawn
    // extender, and the test names them rather than accepting one that scallops.
    let pinned = [0x2211_u32, 0x27E8, 0x27E9];
    without.sort_unstable();
    assert_eq!(
        without, pinned,
        "the glyphs without an assembly should be the ones whose middle is not a bar"
    );
    assert_eq!(
        assembled + without.len(),
        STRETCHY.len(),
        "every stretchy glyph is either assembled or pinned"
    );
    eprintln!(
        "assemblies: {assembled} glyphs, {} pinned, {tolerance} unit joins, worst repeat seam {worst}",
        without.len()
    );
}

/// `r[math.stretch]`: a glyph a renderer can stretch is an extended shape, so
/// what sits beside it is positioned against its ink box rather than its
/// baseline.
#[test]
fn stretched_glyphs_are_extended_shapes() {
    let Some((bytes, captured)) = build_math_font() else {
        eprintln!("skipping: no math capture");
        return;
    };
    let font = FontRef::new(&bytes).expect("reopen the font");
    let math = font.math().expect("MATH");
    let variants = math.math_variants().expect("MathVariants");
    let coverage = math
        .math_glyph_info()
        .expect("MathGlyphInfo")
        .extended_shape_coverage()
        .expect("extended shapes")
        .expect("the table resolves");

    let mut covered = 0usize;
    for (index, glyph) in captured.iter().enumerate() {
        if !glyph.unicodes.iter().any(|code| STRETCHY.contains(code)) {
            continue;
        }
        let id = GlyphId::new(u32::try_from(index + 1).expect("glyph id"));
        assert!(
            coverage.get(id).is_some(),
            "{} is a stretchy glyph but not an extended shape",
            glyph.name
        );
        covered += 1;
        // The sizes derived from it are the ones a renderer reaches for when
        // the base is too short, so they are the ones that most need the ink
        // box.
        let construction = variants
            .glyph_construction(id, StretchAxis::Vertical)
            .unwrap_or_else(|| panic!("{} has no construction", glyph.name));
        for record in construction.math_glyph_variant_records() {
            assert!(
                coverage
                    .get(GlyphId::new(u32::from(record.variant_glyph())))
                    .is_some(),
                "{}'s derived size is not an extended shape",
                glyph.name
            );
            covered += 1;
        }
    }

    assert_eq!(
        covered,
        STRETCHY.len() * (1 + lemmata_build::variants::SCALES.len()),
        "every stretchy glyph and every size derived from it is covered"
    );
    // Nothing else is covered: an extended shape changes how a renderer places
    // what is beside it, so covering an ordinary glyph would move things that
    // have no reason to move.
    assert_eq!(coverage.iter().count(), covered);
    eprintln!("extended shapes: {covered} glyphs");
}
