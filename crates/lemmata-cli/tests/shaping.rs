//! The shaping suite: what a shaper makes of the font.
//!
//! A structural check can prove a table says the right thing. It cannot prove a
//! shaper acts on it, and for a contextual feature that is the whole question,
//! so this runs the font through `HarfBuzz` and looks at the glyphs that come
//! out.

use std::path::{Path, PathBuf};

use lemmata_build::font::{GlyphData, Metrics, Names};
use lemmata_build::texture::Side;
use read_fonts::TableProvider;
use read_fonts::types::GlyphId;

fn capture_path() -> PathBuf {
    std::env::var_os("LEMMATA_GLYPH_DUMP").map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("web/public/glyphs.bin")
        },
        PathBuf::from,
    )
}

/// Rounds a captured advance to the units the font table stores.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn advance_to_u16(advance: f64) -> u16 {
    advance.round().clamp(0.0, f64::from(u16::MAX)) as u16
}

/// Builds a font from the coverage capture, with the glyphs it was built from.
fn build_font() -> Option<(Vec<u8>, Vec<GlyphData>)> {
    let data = std::fs::read(capture_path()).ok()?;
    let captured = lemmata_capture::load(&data).expect("decode the capture");
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

    Some((bytes, glyphs))
}

/// The glyph id a derived form carries. Glyph zero is `.notdef`, the captured
/// glyphs follow in their order, then the derived ones.
fn form_id(glyphs: &[GlyphData], name: &str) -> GlyphId {
    let derived = lemmata_build::font::derived(glyphs, &Metrics::default());
    let index = derived
        .iter()
        .position(|glyph| glyph.name == name)
        .unwrap_or_else(|| panic!("{name} is not derived"));

    GlyphId::new(u32::try_from(glyphs.len() + 1 + index).expect("glyph id"))
}

/// `r[build.texture]`: the substitution fires where the neighbours call for it,
/// and nowhere else.
#[test]
fn texture_healing_fires_in_context() {
    let Some((bytes, glyphs)) = build_font() else {
        eprintln!("skipping: no capture");
        return;
    };
    let blob: harfrust::font::Blob = bytes.into();
    let font = harfrust::Font::new(blob, 0).expect("the shaper reads the font");
    let shaper = harfrust::ShaperFont::new(&font);
    let run = |text: &str| -> Vec<GlyphId> {
        let mut buffer = harfrust::Buffer::new();
        buffer.push_str(text);
        buffer.guess_segment_properties();
        // `calt` is on by default, which is the case that matters: an editor
        // does not ask for it.
        harfrust::shape(&shaper, &mut buffer, harfrust::ShapeOptions::default())
            .expect("shape the run");

        buffer
            .glyph_infos()
            .iter()
            .map(|info| GlyphId::new(info.glyph_id))
            .collect()
    };

    // The capture names its glyphs the way Iosevka does, so the letters are
    // found by codepoint and the forms by the name the derivation gives them.
    let base = |letter: char| -> GlyphId {
        let glyph = glyphs
            .iter()
            .find(|glyph| glyph.unicodes.contains(&u32::from(letter)))
            .unwrap_or_else(|| panic!("{letter} is not captured"));
        let index = glyphs
            .iter()
            .position(|other| other.name == glyph.name)
            .expect("index");

        GlyphId::new(u32::try_from(index + 1).expect("glyph id"))
    };
    let form = |letter: char, side: Side| -> GlyphId {
        let glyph = glyphs
            .iter()
            .find(|glyph| glyph.unicodes.contains(&u32::from(letter)))
            .unwrap_or_else(|| panic!("{letter} is not captured"));

        form_id(
            &glyphs,
            &lemmata_build::texture::form_name(&glyph.name, side),
        )
    };

    // A giver before a taker cedes toward it, and a taker between two givers
    // reaches both ways, which is the pass that only fires once the others have
    // run.
    assert_eq!(
        run("imi"),
        vec![
            form('i', Side::Left),
            form('m', Side::Both),
            form('i', Side::Right)
        ],
        "imi should heal"
    );
    // The giver moves away, which opens a gap on that side, and the taker
    // reaches into it. Both ends of the pair end up moved.
    assert_eq!(
        run("im"),
        vec![form('i', Side::Left), form('m', Side::Left)],
        "a pair heals both ways"
    );
    assert_eq!(
        run("mi"),
        vec![form('m', Side::Right), form('i', Side::Right)],
        "and heals it the other way round"
    );

    // Two givers have nothing to trade, so nothing should move. This is the
    // half a table read-back cannot check: a lookup with the wrong coverage
    // fires here too.
    assert_eq!(
        run("ii"),
        vec![base('i'), base('i')],
        "ii has nothing to heal"
    );

    // A mark beside a taker does not move, because a period that moved would
    // stop reading as a period. The taker moves toward it instead, which is the
    // pass that needs a third role.
    let mark = |letter: char| -> GlyphId {
        let glyph = glyphs
            .iter()
            .find(|glyph| glyph.unicodes.contains(&u32::from(letter)))
            .unwrap_or_else(|| panic!("{letter} is not captured"));
        let index = glyphs
            .iter()
            .position(|other| other.name == glyph.name)
            .expect("index");

        GlyphId::new(u32::try_from(index + 1).expect("glyph id"))
    };
    assert_eq!(
        run(".m"),
        vec![mark('.'), form('m', Side::Left)],
        "a taker reaches toward the mark before it"
    );
    assert_eq!(
        run("m."),
        vec![form('m', Side::Right), mark('.')],
        "and toward the one after it"
    );
}

/// Where a corpus lives.
fn corpus_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("corpus")
        .join(name)
}

/// `r[test.corpus]`: the corpora, and the coverage each one needs.
///
/// A corpus is what a suite runs, so a character the font does not carry makes it
/// a corpus for a different font. The prose and the ligature corpora are written
/// in what the coverage capture holds. The code one is not, and the characters it
/// needs are named rather than left to shape `.notdef` quietly: that set is the
/// coverage gap, and the day it closes the assertion flips.
#[test]
fn the_corpora_are_within_the_coverage() {
    let Some((bytes, _)) = build_font() else {
        eprintln!("skipping: no capture");
        return;
    };
    let font = read_fonts::FontRef::new(&bytes).expect("reopen the font");
    let cmap = font.cmap().expect("cmap");

    let gaps: [(&str, &[char]); 3] = [
        ("prose.txt", &[]),
        ("ligatures.txt", &['|']),
        ("code.txt", &['V', '[', ']', '{', '|', '}']),
    ];
    for (name, expected) in gaps {
        let text = std::fs::read_to_string(corpus_path(name))
            .unwrap_or_else(|error| panic!("reading {name}: {error}"));
        let mut unmapped: Vec<char> = text
            .chars()
            .filter(|character| {
                !character.is_whitespace() && cmap.map_codepoint(u32::from(*character)).is_none()
            })
            .collect();
        unmapped.sort_unstable();
        unmapped.dedup();
        assert_eq!(
            unmapped.as_slice(),
            expected,
            "{name}: the characters it needs that the font does not carry"
        );
        eprintln!(
            "{name}: {} characters, {} the font does not carry",
            text.chars().count(),
            expected.len()
        );
    }
}

/// Where the reference font lives.
fn reference_path() -> PathBuf {
    std::env::var_os("LEMMATA_REFERENCE_DIR")
        .map_or_else(
            || {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .join("target/reference")
            },
            PathBuf::from,
        )
        .join("RefA-Regular.ttf")
}

/// `r[drift.shaping]`: the prose corpus advances the way the oracle font does.
///
/// The oracle is the pinned Iosevka build, shaped by the same shaper. Glyph names
/// are not compared: our font exports none, because `post` is written without a
/// name table, so the comparison is what a shaper can be held to here, the
/// advances and the glyph count. Texture healing does not move an advance, which
/// is what makes the two comparable at all.
#[test]
fn the_prose_corpus_advances_like_the_reference() {
    let Some((bytes, _)) = build_font() else {
        eprintln!("skipping: no capture");
        return;
    };
    let Ok(reference) = std::fs::read(reference_path()) else {
        eprintln!("skipping: no reference font");
        return;
    };
    let text = std::fs::read_to_string(corpus_path("prose.txt")).expect("read the prose corpus");

    // The advance total and the glyph count, which is what a shaper decides.
    let shape = |font: &[u8]| -> (i64, usize) {
        let blob: harfrust::font::Blob = font.to_vec().into();
        let font = harfrust::Font::new(blob, 0).expect("the shaper reads the font");
        let shaper = harfrust::ShaperFont::new(&font);
        let mut buffer = harfrust::Buffer::new();
        buffer.push_str(&text);
        buffer.guess_segment_properties();
        harfrust::shape(&shaper, &mut buffer, harfrust::ShapeOptions::default())
            .expect("shape the run");

        let advance: i64 = buffer
            .glyph_positions()
            .iter()
            .map(|position| i64::from(position.x_advance))
            .sum();

        (advance, buffer.glyph_infos().len())
    };

    let (ours, our_glyphs) = shape(&bytes);
    let (theirs, their_glyphs) = shape(&reference);
    assert_eq!(
        our_glyphs, their_glyphs,
        "the same text should shape to the same number of glyphs"
    );
    assert_eq!(
        ours, theirs,
        "the prose corpus advances {ours} against the reference's {theirs}"
    );
    eprintln!("shaping: {our_glyphs} glyphs, {ours} units, matching the reference");
}

/// `r[test.corpus]`, `r[drift.shaping]`: the ligature corpus, and why neither
/// font ligates it.
///
/// Ours carries `calt` for texture healing and no ligature. The reference does
/// not either, and that is worth recording rather than working around: the
/// pinned plan selects no ligation set, so the build ships every trigger as its
/// own glyphs and the reference font has six ligature lookups in total, none of
/// them for an arrow. Comparing ligatures needs an oracle that has them, which
/// means a reference plan with a ligation set and a new fingerprint, and the
/// parity pair records the current one.
///
/// So the test pins both halves. The day a ligature lands here, or a reference
/// plan starts selecting a set, one of the two assertions moves.
#[test]
fn neither_font_ligates_the_triggers_yet() {
    let Some((bytes, _)) = build_font() else {
        eprintln!("skipping: no capture");
        return;
    };
    let Ok(reference) = std::fs::read(reference_path()) else {
        eprintln!("skipping: no reference font");
        return;
    };
    let text =
        std::fs::read_to_string(corpus_path("ligatures.txt")).expect("read the ligature corpus");

    let count = |font: &[u8], text: &str| -> usize {
        let blob: harfrust::font::Blob = font.to_vec().into();
        let font = harfrust::Font::new(blob, 0).expect("the shaper reads the font");
        let shaper = harfrust::ShaperFont::new(&font);
        let mut buffer = harfrust::Buffer::new();
        buffer.push_str(text);
        buffer.guess_segment_properties();
        // A trigger is all punctuation, so the guessed script is Common, and a
        // ligation set is registered under a script. An editor setting code is in
        // a Latin run, which is what this is.
        buffer.set_script(Some(harfrust::Script::LATIN));
        harfrust::shape(&shaper, &mut buffer, harfrust::ShapeOptions::default())
            .expect("shape the run");

        buffer.glyph_infos().len()
    };

    let mut triggers = 0usize;
    for trigger in text.lines().filter(|line| !line.is_empty()) {
        let characters = trigger.chars().count();
        assert_eq!(
            count(&bytes, trigger),
            characters,
            "{trigger} combined here, so a ligature has landed and this pin can go"
        );
        assert_eq!(
            count(&reference, trigger),
            characters,
            "{trigger} combined in the reference, so the oracle can measure ligatures now"
        );
        triggers += 1;
    }
    assert!(triggers > 0, "the corpus has no triggers");
    eprintln!("ligatures: {triggers} triggers, neither font combines one yet");
}
