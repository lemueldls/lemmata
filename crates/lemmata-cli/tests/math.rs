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
use read_fonts::tables::gsub::SubstitutionSubtables;
use read_fonts::tables::math::{MathKernCorner, PartFlags, StretchAxis};
use read_fonts::types::{GlyphId, Tag};
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
    let bytes = lemmata_build::font::build(&glyphs, &Metrics::default(), &Names::default(), &[])
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
    // A letter a renderer sets on its own. A combining mark is alphabetic and is
    // not one: it attaches to a base and carries no advance, so there is no
    // advance for either measurement to be inside, and both are read for the
    // bases.
    let is_letter = |glyph: &lemmata_capture::Glyph| {
        glyph.advance != 0.0
            && glyph
                .unicodes
                .iter()
                .any(|code| char::from_u32(*code).is_some_and(char::is_alphabetic))
    };
    for (index, glyph) in captured.iter().enumerate() {
        if !is_letter(glyph) {
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
    let letters = captured.iter().filter(|glyph| is_letter(glyph)).count();
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

/// The top-corner kerns: an italic letter's overhang is recorded at the heights
/// where it changes, so a renderer can place a script against the ink instead
/// of the advance.
#[test]
fn math_letters_carry_top_corner_kerning() {
    let Some((bytes, captured)) = build_math_font() else {
        eprintln!("skipping: no math capture");
        return;
    };
    let font = FontRef::new(&bytes).expect("reopen the font");
    let info = font
        .math()
        .expect("the font has a MATH table")
        .math_glyph_info()
        .expect("MathGlyphInfo");
    let kern_info = info
        .math_kern_info()
        .expect("MathKernInfo")
        .expect("the table resolves");

    let mut italic = 0usize;
    let mut with_kern = 0usize;
    for (index, glyph) in captured.iter().enumerate() {
        if !glyph
            .unicodes
            .iter()
            .any(|code| (0x1D434..=0x1D467).contains(code))
        {
            continue;
        }
        italic += 1;
        let id = GlyphId::new(u32::try_from(index + 1).expect("glyph id"));
        let Some(kern) = kern_info.kern(id, MathKernCorner::TopRight) else {
            continue;
        };
        with_kern += 1;
        let heights: Vec<i32> = kern
            .correction_height()
            .iter()
            .map(|height| i32::from(height.value().to_i16()))
            .collect();
        assert!(
            heights.windows(2).all(|pair| pair[0] < pair[1]),
            "{} records heights out of order: {heights:?}",
            glyph.name
        );
        let values: Vec<i32> = kern
            .kern_values()
            .iter()
            .map(|value| i32::from(value.value().to_i16()))
            .collect();
        assert_eq!(
            values.len(),
            heights.len() + 1,
            "{} has one more kern than the heights it bands",
            glyph.name
        );
        assert!(
            values.last().copied().unwrap_or(0) > 0,
            "{}'s top-right overhang is not positive: {values:?}",
            glyph.name
        );
    }

    // An italic x-height letter can lean and still stay inside its cell. What
    // the table has to catch is the tall letters whose hook or ascender leaves
    // it, and there are enough of those to prove the table is filled.
    assert!(
        with_kern >= 10,
        "only {with_kern} of {italic} italic letters carry a top-right kern"
    );

    // The stored value is the outline's overhang, not a guess. The italic f's
    // rightmost point is the top of its hook, so the last band is its widest.
    let (index, glyph) = captured
        .iter()
        .enumerate()
        .find(|(_, glyph)| glyph.unicodes.contains(&0x1D453))
        .expect("the capture holds the italic f");
    let id = GlyphId::new(u32::try_from(index + 1).expect("glyph id"));
    let kern = kern_info
        .kern(id, MathKernCorner::TopRight)
        .expect("the italic f has a top-right kern");
    let contours = lemmata_capture::contours(glyph);
    let right = contours
        .iter()
        .flatten()
        .fold(f64::NEG_INFINITY, |right, point| right.max(point.x));
    let overhang = (right - glyph.advance).round().max(0.0);
    assert_eq!(
        f64::from(
            kern.kern_values()
                .last()
                .expect("a final value")
                .value()
                .to_i16()
        ),
        overhang,
        "the italic f stores a peak kern against an overhang of {overhang}"
    );

    // The upright block stays inside its advance, so it has no overhang to
    // record. Covering it with a zero table would only cost bytes.
    let upright = captured
        .iter()
        .enumerate()
        .find(|(_, glyph)| glyph.unicodes.contains(&0x1D400))
        .map(|(index, _)| GlyphId::new(u32::try_from(index + 1).expect("glyph id")))
        .expect("the capture holds bold A");
    assert!(
        kern_info.kern(upright, MathKernCorner::TopRight).is_none(),
        "an upright letter inside its box should have no top-right kern"
    );

    eprintln!("math kerning: {with_kern} of {italic} italic letters carry a top-right kern");
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

    let width = |id: GlyphId| -> i32 {
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
            low = low.min(i32::from(point.x));
            high = high.max(i32::from(point.x));
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
        // A large operator is set bigger rather than stretched, so it carries
        // one more size than a delimiter does.
        let large = glyph
            .unicodes
            .iter()
            .any(|code| lemmata_build::variants::OPERATORS.contains(code));
        let scales = lemmata_build::variants::scales(large);
        let records = construction.math_glyph_variant_records();
        assert_eq!(
            records.len(),
            scales.len(),
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
            // The summation and the product are set bigger, so their display
            // forms are wider as well as taller. A delimiter stretches, and so
            // does the integral, whose limits sit in the gaps a wider drawing
            // would push them out of.
            let uniform = glyph
                .unicodes
                .iter()
                .any(|code| lemmata_build::variants::SET_BIGGER.contains(code));
            let narrow = width(id);
            let wide = width(GlyphId::new(u32::from(variant)));
            if uniform {
                assert!(
                    wide > narrow,
                    "{}'s derived size is {wide} wide against a base of {narrow}, and a \
                     summation should grow in both axes",
                    glyph.name
                );
            } else {
                // A composed size draws its own hook cap, so its ink can differ
                // from the text glyph by a rounding step. A set-bigger operator
                // is the only one that may change width by more.
                assert!(
                    (wide - narrow).abs() <= 4,
                    "{}'s derived size changed width from {narrow} to {wide}, and only a set-bigger operator should",
                    glyph.name
                );
            }
            derived += 1;
        }
    }
    let expected: usize = STRETCHY
        .iter()
        .map(|code| {
            lemmata_build::variants::scales(lemmata_build::variants::OPERATORS.contains(code)).len()
        })
        .sum();
    assert_eq!(derived, expected);
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
        let brace = glyph
            .unicodes
            .iter()
            .any(|code| *code == 0x007B || *code == 0x007D);
        let Some(resolved) = construction.glyph_assembly() else {
            without.push(glyph.unicodes[0]);
            continue;
        };
        let assembly = resolved.unwrap_or_else(|error| panic!("{}: {error:?}", glyph.name));
        let parts = assembly.part_records();
        let expected = if brace {
            lemmata_build::assembly::BRACE_PIECES.len()
        } else {
            lemmata_build::assembly::PIECES.len()
        };
        assert_eq!(
            parts.len(),
            expected,
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
                let expected = if brace {
                    lemmata_build::assembly::BRACE_EXTENDERS.contains(&position)
                } else {
                    position == lemmata_build::assembly::EXTENDER
                };
                assert!(
                    expected,
                    "{} repeats a piece that is not an extender",
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
            extenders,
            if brace { 2 } else { 1 },
            "{} should repeat the expected number of pieces",
            glyph.name
        );

        // The repeat seam, which is what a derived extender can get wrong. The
        // extender is a bar, so its two cut edges have to agree; a bar that
        // still curves would show every repeat as a step.
        for repeat in parts
            .iter()
            .filter(|part| part.part_flags().contains(PartFlags::EXTENDER_FLAG))
        {
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
            assert_eq!(
                seam, 0,
                "{}'s extender steps {seam} where it repeats",
                glyph.name
            );
            worst = worst.max(seam);
        }

        let (_, base_low, _, base_high) = span(id);
        let height = base_high - base_low;
        assert!(
            height - bottom <= joins,
            "{} assembles to {bottom} against a glyph {height} tall",
            glyph.name
        );
        assembled += 1;
    }

    // A glyph whose middle is not a bar has no band that repeats. A summation's
    // middle holds its crossbar, an angle bracket's holds its vertex and a
    // radical's is a diagonal, so the piece would stack the feature rather than
    // a stroke. They need a drawn extender, and the test names them rather than
    // accepting one that scallops. The braces used to be here; their five-part
    // construction puts the point between two bars instead.
    let pinned = [0x2211_u32, 0x221A, 0x27E8, 0x27E9];
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

/// The radical is a hook and a vertical stem, not a scaled check mark.
///
/// A renderer draws the vinculum itself, starting at the radical's advance. The
/// stem's right edge is that advance and its top is flat, so the rule meets a
/// corner rather than the point of a diagonal.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[test]
fn the_radical_has_a_vertical_stem() {
    let Some((bytes, captured)) = build_math_font() else {
        eprintln!("skipping: no math capture");
        return;
    };
    let Some(glyph) = captured
        .iter()
        .find(|glyph| glyph.unicodes.contains(&0x221A))
    else {
        eprintln!("skipping: the capture holds no radical");
        return;
    };
    let font = FontRef::new(&bytes).expect("reopen the font");
    let loca = font.loca(None).expect("loca");
    let glyf = font.glyf().expect("glyf");
    let id = GlyphId::new(
        u32::try_from(
            captured
                .iter()
                .position(|other| other.name == glyph.name)
                .expect("the radical is in the capture"),
        )
        .expect("glyph id")
            + 1,
    );
    let read = loca
        .get_glyf(id, &glyf)
        .expect("read a glyph")
        .expect("the radical has an outline");
    let ReadGlyph::Simple(simple) = read else {
        panic!("the radical is a simple glyph");
    };
    let points: Vec<(i32, i32)> = simple
        .points()
        .map(|point| (i32::from(point.x), i32::from(point.y)))
        .collect();
    let advance = i32::try_from(glyph.advance.round() as u32).expect("advance");
    let top = points.iter().map(|(_, y)| *y).max().expect("points");
    let right = points.iter().map(|(x, _)| *x).max().expect("points");
    let left = points.iter().map(|(x, _)| *x).min().expect("points");
    let at_top: Vec<i32> = points
        .iter()
        .filter(|(_, y)| *y == top)
        .map(|(x, _)| *x)
        .collect();

    assert_eq!(right, advance, "the stem's right edge is the advance");
    assert!(
        left < advance - 40,
        "the hook reaches left of the stem: {left}"
    );
    assert!(
        at_top.iter().any(|x| *x == advance),
        "the stem does not reach the top: {at_top:?}"
    );
    assert!(
        at_top.iter().any(|x| *x < advance),
        "the top is not flat across the stem: {at_top:?}"
    );
    eprintln!("radical: hook from {left}, stem to {right}, top {top}, advance {advance}");
}

/// `r[math.stretch]`: a horizontal arrow grows along x/// `r[math.stretch]`: a horizontal arrow grows along x, by lengthening its
/// shaft rather than by scaling the head with the cell.
#[test]
fn horizontal_arrows_have_assemblies() {
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

    let span = |id: GlyphId| -> (i32, i32, i32, i32) {
        let glyph = loca
            .get_glyf(id, &glyf)
            .expect("read a glyph")
            .unwrap_or_else(|| panic!("glyph {id} has no outline"));
        let ReadGlyph::Simple(simple) = glyph else {
            panic!("a stretchable arrow is a simple glyph");
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

    let horizontal = lemmata_build::variants::HORIZONTAL;
    let mut assembled = 0usize;
    let mut worst = 0i32;
    let mut without: Vec<&str> = Vec::new();
    for (index, glyph) in captured.iter().enumerate() {
        if !glyph.unicodes.iter().any(|code| horizontal.contains(code)) {
            continue;
        }
        let id = GlyphId::new(u32::try_from(index + 1).expect("glyph id"));
        let construction = variants
            .glyph_construction(id, StretchAxis::Horizontal)
            .unwrap_or_else(|| panic!("{} has no horizontal construction", glyph.name));
        // The sizes are the first thing a renderer reaches for, and each one
        // measures the width it grew to rather than its height.
        let records = construction.math_glyph_variant_records();
        assert_eq!(
            records.len(),
            lemmata_build::variants::HORIZONTAL_SCALES.len(),
            "{} should carry one size per scale",
            glyph.name
        );
        let base_width = span(id).2 - span(id).0;
        let mut previous = base_width;
        for record in records {
            let width = i32::from(u16::from(record.advance_measurement()));
            assert!(
                width > previous,
                "{}'s sizes do not grow: {width} after {previous}",
                glyph.name
            );
            previous = width;
        }

        let Some(resolved) = construction.glyph_assembly() else {
            without.push(glyph.name.as_str());
            continue;
        };
        let assembly = resolved.unwrap_or_else(|error| panic!("{}: {error:?}", glyph.name));
        let parts = assembly.part_records();
        assert_eq!(
            parts.len(),
            lemmata_build::assembly::HORIZONTAL_PIECES.len(),
            "{} should have one piece per column",
            glyph.name
        );

        // Lay the parts left to right the way a renderer does: the first starts
        // at the origin, and each later one overlaps the one before it.
        let mut left = 0i32;
        let mut before: Option<(i32, i32, i32)> = None;
        let mut extenders = 0usize;
        let mut joins = 0i32;
        for (position, part) in parts.iter().enumerate() {
            let (x_min, _, x_max, _) = span(GlyphId::new(u32::from(part.glyph_id().to_u16())));
            let full = i32::from(u16::from(part.full_advance()));
            let start = i32::from(u16::from(part.start_connector_length()));
            let end = i32::from(u16::from(part.end_connector_length()));
            assert_eq!(
                x_min, 0,
                "{} piece {position} does not start on the origin",
                glyph.name
            );
            assert_eq!(
                x_max, full,
                "{} piece {position} spans {x_max} against an advance of {full}",
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
            if let Some((below_x_min, below_x_max, below_end)) = before {
                assert!(
                    x_min < below_x_max && below_x_min < x_max,
                    "{} piece {position} does not cross the one before it",
                    glyph.name
                );
                let overlap = below_end.min(start);
                assert!(
                    overlap > 0,
                    "{} piece {position} does not overlap the one before it",
                    glyph.name
                );
                assert!(
                    overlap <= tolerance,
                    "{} piece {position} overlaps by {overlap}, past the {tolerance} the table allows",
                    glyph.name
                );
                joins += overlap;
                left += full - overlap;
            } else {
                left = full;
            }
            before = Some((x_min, x_max, end));
        }
        assert_eq!(
            extenders, 1,
            "{} should repeat exactly one piece",
            glyph.name
        );

        // The repeat seam, on the vertical axis this time: the middle column is
        // a band of a shaft, so its top and bottom inks have to agree at the
        // left and right edges where the piece repeats.
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
            if x <= 0 {
                low_min = low_min.min(y);
                low_max = low_max.max(y);
            }
            if x >= full {
                high_min = high_min.min(y);
                high_max = high_max.max(y);
            }
        }
        let seam = (high_min - low_min).abs().max((high_max - low_max).abs());
        assert!(
            seam <= tolerance,
            "{}'s extender steps {seam} where it repeats, past the {tolerance} the table allows",
            glyph.name
        );
        worst = worst.max(seam);

        assert!(
            base_width - left <= joins,
            "{} assembles to {left} against a glyph {base_width} wide",
            glyph.name
        );
        assembled += 1;
    }

    assert!(
        assembled + without.len()
            == captured
                .iter()
                .filter(|glyph| glyph.unicodes.iter().any(|code| horizontal.contains(code)))
                .count(),
        "every horizontal arrow is either assembled or named"
    );
    // A glyph whose middle is not a bar has no column that repeats. The
    // double-headed right arrow's heads meet in the middle, and the negated
    // arrow's stroke crosses the shaft, so both would stack the feature rather
    // than a stroke. They need a drawn extender.
    without.sort_unstable();
    assert_eq!(
        without,
        ["dhArrowRight.WWID", "uni21AE.WWID"],
        "the arrows without an assembly should be the ones whose middle is not a bar"
    );
    eprintln!(
        "horizontal assemblies: {assembled} glyphs, {} pinned, {tolerance} unit joins, worst repeat seam {worst}",
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
        let vertical = glyph.unicodes.iter().any(|code| STRETCHY.contains(code));
        let horizontal = glyph
            .unicodes
            .iter()
            .any(|code| lemmata_build::variants::HORIZONTAL.contains(code));
        if !vertical && !horizontal {
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
        let axis = if vertical {
            StretchAxis::Vertical
        } else {
            StretchAxis::Horizontal
        };
        let construction = variants
            .glyph_construction(id, axis)
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

    let vertical_sizes: usize = STRETCHY
        .iter()
        .map(|code| {
            lemmata_build::variants::scales(lemmata_build::variants::OPERATORS.contains(code)).len()
        })
        .sum();
    let horizontal_sizes = lemmata_build::variants::HORIZONTAL.len()
        * lemmata_build::variants::HORIZONTAL_SCALES.len();
    assert_eq!(
        covered,
        STRETCHY.len()
            + vertical_sizes
            + lemmata_build::variants::HORIZONTAL.len()
            + horizontal_sizes,
        "every stretchy glyph and every size derived from it is covered"
    );
    // Nothing else is covered: an extended shape changes how a renderer places
    // what is beside it, so covering an ordinary glyph would move things that
    // have no reason to move.
    assert_eq!(coverage.iter().count(), covered);
    eprintln!("extended shapes: {covered} glyphs");
}

/// `r[math.script-style]`: the script-size alternates, and the `ssty` feature
/// that offers them to a renderer.
#[test]
fn script_style_alternates_are_carried_by_ssty() {
    let Some((bytes, captured)) = build_math_font() else {
        eprintln!("skipping: no math capture");
        return;
    };
    let font = FontRef::new(&bytes).expect("reopen the font");
    let gsub = font.gsub().expect("the font carries a GSUB");

    let feature_list = gsub.feature_list().expect("a feature list");
    let record = feature_list
        .feature_records()
        .iter()
        .find(|record| record.feature_tag() == Tag::new(b"ssty"))
        .expect("the font carries ssty");
    let index = usize::from(
        record
            .feature(feature_list.offset_data())
            .expect("the feature reads")
            .lookup_list_indices()
            .first()
            .expect("the feature names a lookup")
            .get(),
    );
    let lookup = gsub
        .lookup_list()
        .expect("a lookup list")
        .lookups()
        .get(index)
        .expect("the lookup is in the list");

    // An alternate substitution, not a single one. The difference is the whole
    // point: the feature hands the renderer a list and the renderer picks a form
    // by index, which is how two optical sizes are offered without the feature
    // choosing between them. A single substitution here would give a renderer
    // one form and no way to ask for the other.
    let SubstitutionSubtables::Alternate(subtables) = lookup.subtables().expect("subtables") else {
        panic!("ssty is not an alternate substitution");
    };
    let table = subtables
        .iter()
        .next()
        .expect("a subtable")
        .expect("the subtable reads");
    let sets = table.alternate_sets();
    assert!(!sets.is_empty(), "ssty names no glyphs");

    let scales = lemmata_build::ssty::SCALES.len();
    for set in sets.iter() {
        let forms = set.expect("the alternate set reads").alternate_glyph_ids();
        assert_eq!(
            forms.len(),
            scales,
            "an alternate set should offer one form per scale"
        );
    }

    // The derivation holds the height and widens. Both reference fonts widen,
    // and they disagree about the height, so the height is the one thing a
    // transform should not move: a script-size glyph that grows taller collides
    // with the line above it.
    let glyphs: Vec<GlyphData> = captured
        .iter()
        .map(|glyph| GlyphData {
            name: glyph.name.clone(),
            unicodes: glyph.unicodes.clone(),
            advance: advance_to_u16(glyph.advance),
            contours: lemmata_capture::contours(glyph),
        })
        .collect();
    let alternates = lemmata_build::ssty::derive(&glyphs, 0);
    assert!(
        !alternates.is_empty(),
        "the capture has glyphs a renderer sets at script size"
    );
    assert_eq!(
        alternates.len(),
        sets.len() * scales,
        "the derivation and the feature should cover the same glyphs"
    );
    for (index, alternate) in alternates.iter().enumerate() {
        let base_name = alternate
            .name
            .strip_suffix(".sts")
            .or_else(|| alternate.name.strip_suffix(".st"))
            .expect("an alternate is named for its base");
        let base = glyphs
            .iter()
            .find(|glyph| glyph.name == base_name)
            .unwrap_or_else(|| panic!("{} has no base", alternate.name));
        let (_, low, _, high) = alternate.bounding_box().expect("the alternate has ink");
        let (_, base_low, _, base_high) = base.bounding_box().expect("the base has ink");
        assert_eq!(
            (high - low, low),
            (base_high - base_low, base_low),
            "{} should keep its base's height and baseline",
            alternate.name
        );
        assert!(
            alternate.advance > base.advance,
            "{} should take more room than its base",
            alternate.name
        );
        let (x0, _, x1, _) = alternate.bounding_box().expect("the alternate has ink");
        let (base_x0, _, base_x1, _) = base.bounding_box().expect("the base has ink");
        assert!(
            x1 - x0 > base_x1 - base_x0,
            "{} should be wider than its base",
            alternate.name
        );
        // The second scale is the wider of the two, which is what makes the
        // list an order rather than a set.
        if index % scales == 1 {
            let narrower = &alternates[index - 1];
            assert!(
                alternate.advance > narrower.advance,
                "{} should be the wider of the pair",
                alternate.name
            );
        }
    }
    eprintln!(
        "ssty: {} glyphs, {} alternates",
        sets.len(),
        alternates.len()
    );
}

/// `r[math.script-style]`: the alternate reaches a shaper, and the form it picks
/// is the one the value asks for.
#[test]
fn ssty_reaches_a_shaper() {
    let Some((bytes, captured)) = build_math_font() else {
        eprintln!("skipping: no math capture");
        return;
    };
    let glyphs: Vec<GlyphData> = captured
        .iter()
        .map(|glyph| GlyphData {
            name: glyph.name.clone(),
            unicodes: glyph.unicodes.clone(),
            advance: advance_to_u16(glyph.advance),
            contours: lemmata_capture::contours(glyph),
        })
        .collect();

    // The math italic `x`, which is the glyph an equation sets a variable in.
    let code = 0x1D465;
    let base = captured
        .iter()
        .find(|glyph| glyph.unicodes.contains(&code))
        .unwrap_or_else(|| panic!("the capture has no U+{code:04X}"));
    let text = char::from_u32(code).expect("a scalar value").to_string();

    let blob: harfrust::font::Blob = bytes.into();
    let font = harfrust::Font::new(blob, 0).expect("the shaper reads the font");
    let shaper = harfrust::ShaperFont::new(&font);
    // A renderer sets the run's script to math when it sets an equation, and
    // that is what selects the feature: it is registered under `math`, so a text
    // run never reaches it. `guess_segment_properties` reads the script off the
    // text and the math alphanumerics are Common, so the script is set after it.
    let run = |value: u32| -> Vec<GlyphId> {
        let mut buffer = harfrust::Buffer::new();
        buffer.push_str(&text);
        buffer.guess_segment_properties();
        buffer.set_script(Some(harfrust::Script::MATH));
        let features = [harfrust::Feature::new(Tag::new(b"ssty"), value, ..)];
        harfrust::shape(
            &shaper,
            &mut buffer,
            harfrust::ShapeOptions::default().features(&features),
        )
        .expect("shape the run");

        buffer
            .glyph_infos()
            .iter()
            .map(|info| GlyphId::new(info.glyph_id))
            .collect()
    };
    // The glyph ids are the input's, then the derived ones in order, so a shaped
    // id names a glyph the way the derivation does.
    let mut all = glyphs.clone();
    all.extend(lemmata_build::font::derived(&glyphs, &Metrics::default()));
    let name_of = |id: GlyphId| -> String {
        let index = usize::try_from(id.to_u32()).expect("an id");
        if index == 0 {
            return ".notdef".to_string();
        }

        all.get(index - 1)
            .unwrap_or_else(|| panic!("glyph {index} is not in the font"))
            .name
            .clone()
    };

    // Zero disables the feature, which is the base glyph. One and two select the
    // two forms, in the order the table lists them.
    let plain = run(0);
    assert_eq!(
        name_of(plain[0]),
        base.name,
        "a disabled ssty changes nothing"
    );
    assert_eq!(
        name_of(run(1)[0]),
        lemmata_build::ssty::alternate_name(&base.name, 0),
        "the first alternate should be the script size"
    );
    assert_eq!(
        name_of(run(2)[0]),
        lemmata_build::ssty::alternate_name(&base.name, 1),
        "the second alternate should be the scriptscript size"
    );
    eprintln!("ssty: a shaper picks {} for {}", base.name, text);
}

/// `r[math.script-style]`'s inventory: the dotless forms the specification asks
/// for, and the `dtls` feature that names them.
#[test]
fn dotless_forms_are_carried_by_dtls() {
    let Some((bytes, captured)) = build_math_font() else {
        eprintln!("skipping: no math capture");
        return;
    };
    let font = FontRef::new(&bytes).expect("reopen the font");
    let gsub = font.gsub().expect("the font carries a GSUB");
    let tags: Vec<Tag> = gsub
        .feature_list()
        .expect("a feature list")
        .feature_records()
        .iter()
        .map(|record| record.feature_tag())
        .collect();
    assert!(
        tags.contains(&Tag::new(b"dtls")),
        "the features are {tags:?}"
    );
    assert!(
        !gsub
            .lookup_list()
            .expect("a lookup list")
            .lookups()
            .is_empty(),
        "the feature has nothing to point at"
    );

    // The forms are dotless: the derivation drops the dot contour and nothing
    // else, so each one is exactly one contour shorter than its base.
    let glyphs: Vec<GlyphData> = captured
        .iter()
        .map(|glyph| GlyphData {
            name: glyph.name.clone(),
            unicodes: glyph.unicodes.clone(),
            advance: advance_to_u16(glyph.advance),
            contours: lemmata_capture::contours(glyph),
        })
        .collect();
    let forms = lemmata_build::dotless::derive(&glyphs, f64::from(Metrics::default().x_height), 0);
    assert!(!forms.is_empty(), "the corpus has letters carrying a dot");
    for form in &forms {
        let base = glyphs
            .iter()
            .find(|glyph| form.name == lemmata_build::dotless::dotless_name(&glyph.name))
            .unwrap_or_else(|| panic!("{} has no base", form.name));
        assert_eq!(
            form.contours.len() + 1,
            base.contours.len(),
            "{} should drop one contour",
            form.name
        );
        let (_, _, _, top) = form.bounding_box().expect("the form has ink");
        let (_, _, _, base_top) = base.bounding_box().expect("the base has ink");
        assert!(top < base_top, "{} should be shorter", form.name);
    }
    eprintln!("dtls: {} dotless forms", forms.len());
}

/// `r[math.script-style]`: the flattened accents, and the `flac` feature that
/// names them.
#[test]
fn flattened_accents_are_carried_by_flac() {
    let Some((bytes, captured)) = build_math_font() else {
        eprintln!("skipping: no math capture");
        return;
    };
    let font = FontRef::new(&bytes).expect("reopen the font");
    let gsub = font.gsub().expect("the font carries a GSUB");
    let feature_list = gsub.feature_list().expect("a feature list");
    let record = feature_list
        .feature_records()
        .iter()
        .find(|record| record.feature_tag() == Tag::new(b"flac"))
        .expect("the font carries flac");
    let index = usize::from(
        record
            .feature(feature_list.offset_data())
            .expect("the feature reads")
            .lookup_list_indices()
            .first()
            .expect("the feature names a lookup")
            .get(),
    );
    let lookup = gsub
        .lookup_list()
        .expect("a lookup list")
        .lookups()
        .get(index)
        .expect("the lookup is in the list");

    // A single substitution, unlike `ssty`: the renderer asks for `flac` and
    // gets the flat form, with no choice to make.
    assert_eq!(
        lookup.lookup_type(),
        1,
        "flac should be a single substitution"
    );

    let glyphs: Vec<GlyphData> = captured
        .iter()
        .map(|glyph| GlyphData {
            name: glyph.name.clone(),
            unicodes: glyph.unicodes.clone(),
            advance: advance_to_u16(glyph.advance),
            contours: lemmata_capture::contours(glyph),
        })
        .collect();
    let flattened = lemmata_build::flac::derive(&glyphs, 0);
    assert!(
        !flattened.is_empty(),
        "the capture has accents above the baseline"
    );
    for form in &flattened {
        let base_name = form
            .name
            .strip_suffix(".mathcap")
            .expect("an alternate is named for its base");
        let base = glyphs
            .iter()
            .find(|glyph| glyph.name == base_name)
            .unwrap_or_else(|| panic!("{} has no base", form.name));
        assert!(
            base.name.contains("Above"),
            "{} flattens an accent that does not sit above the base",
            form.name
        );
        assert_eq!(
            form.advance, base.advance,
            "{} should keep its base's advance",
            form.name
        );

        // Flattened: the same bottom edge, a shorter height and a wider drawing.
        // The bottom is the edge the accent attaches by, so keeping it is what
        // lets the flattened form stand in for the ordinary one.
        let (x0, low, x1, high) = form.bounding_box().expect("the form has ink");
        let (base_x0, base_low, base_x1, base_high) =
            base.bounding_box().expect("the base has ink");
        assert!(
            (low - base_low).abs() <= 1,
            "{} moves its bottom edge from {base_low} to {low}",
            form.name
        );
        assert!(
            high - low < base_high - base_low,
            "{} is {} tall against its base's {}",
            form.name,
            high - low,
            base_high - base_low
        );
        assert!(
            x1 - x0 > base_x1 - base_x0,
            "{} is {} wide against its base's {}",
            form.name,
            x1 - x0,
            base_x1 - base_x0
        );
    }
    eprintln!("flac: {} flattened accents", flattened.len());
}

/// `r[math.script-style]`: the two single substitutions reach a shaper, not just
/// the table.
///
/// A read-back proves the table says the right thing and cannot prove a shaper
/// acts on it, and for a feature registered under a script that is the whole
/// question: `ssty` was written and inert until the run's script was set to
/// math, and this is the same check for the other two.
#[test]
fn the_math_substitutions_reach_a_shaper() {
    let Some((bytes, captured)) = build_math_font() else {
        eprintln!("skipping: no math capture");
        return;
    };
    let glyphs: Vec<GlyphData> = captured
        .iter()
        .map(|glyph| GlyphData {
            name: glyph.name.clone(),
            unicodes: glyph.unicodes.clone(),
            advance: advance_to_u16(glyph.advance),
            contours: lemmata_capture::contours(glyph),
        })
        .collect();
    let mut all = glyphs.clone();
    all.extend(lemmata_build::font::derived(&glyphs, &Metrics::default()));

    let blob: harfrust::font::Blob = bytes.into();
    let font = harfrust::Font::new(blob, 0).expect("the shaper reads the font");
    let shaper = harfrust::ShaperFont::new(&font);
    let shape = |text: &str, tag: &[u8; 4]| -> String {
        let mut buffer = harfrust::Buffer::new();
        buffer.push_str(text);
        buffer.guess_segment_properties();
        // `dtls` and `flac` are registered under `math`, like `ssty`, and a text
        // run never selects it.
        buffer.set_script(Some(harfrust::Script::MATH));
        let features = [harfrust::Feature::new(Tag::new(tag), 1, ..)];
        harfrust::shape(
            &shaper,
            &mut buffer,
            harfrust::ShapeOptions::default().features(&features),
        )
        .expect("shape the run");

        let id = GlyphId::new(buffer.glyph_infos()[0].glyph_id);
        let index = usize::try_from(id.to_u32()).expect("an id");
        all.get(index - 1)
            .unwrap_or_else(|| panic!("glyph {index} is not in the font"))
            .name
            .clone()
    };

    // `dtls` is the dotless form, which is what an accent sits on.
    assert_eq!(
        shape("i", b"dtls"),
        lemmata_build::dotless::dotless_name("i"),
        "dtls should draw the dotless `i`"
    );
    // `flac` is the flattened accent, which is what a tall base takes.
    assert_eq!(
        shape("\u{300}", b"flac"),
        lemmata_build::flac::flattened_name("graveAbove"),
        "flac should draw the flattened grave"
    );
    eprintln!("shaper: dtls and flac both fire");
}
