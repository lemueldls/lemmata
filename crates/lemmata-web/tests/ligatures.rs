//! Checks that the font the site builds joins the sequences the ligature rows
//! name.
//!
//! The sequence is read back through the shaper rather than the tables: a
//! lookup a shaper ignores is a lookup that does nothing, and the whole point is
//! what a renderer draws.

use std::fs;
use std::path::{Path, PathBuf};

fn capture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("web/public/glyphs.bin")
}

/// Shapes `text` and returns the glyph ids and the total advance.
fn shape(font: &harfrust::Font, text: &str) -> (Vec<u32>, i32) {
    let shaper = harfrust::ShaperFont::new(font);
    let mut buffer = harfrust::Buffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    harfrust::shape(&shaper, &mut buffer, harfrust::ShapeOptions::default())
        .expect("shape the run");

    (
        buffer
            .glyph_infos()
            .iter()
            .map(|info| info.glyph_id)
            .collect(),
        buffer
            .glyph_positions()
            .iter()
            .map(|position| position.x_advance)
            .sum(),
    )
}

#[test]
fn the_sequence_joins_into_the_ligature_forms() {
    let Ok(capture) = fs::read(capture_path()) else {
        eprintln!("skipping: no capture");
        return;
    };
    let bytes = lemmata_web::build(&capture, "").expect("build");
    let blob: harfrust::font::Blob = bytes.into();
    let font = harfrust::Font::new(blob, 0).expect("the shaper reads the font");

    let (plain_hyphen, _) = shape(&font, "-");
    let (plain_greater, _) = shape(&font, ">");
    let (joined, advance) = shape(&font, "->");

    // A joined run is not just the two characters: the shaper replaced at least
    // one of them with a form that meets its neighbour.
    assert_ne!(
        joined,
        vec![plain_hyphen[0], plain_greater[0]],
        "-> did not join"
    );
    // The joined form still occupies the two cells the characters did.
    assert_eq!(advance, 1000, "-> does not keep its two advances");

    for text in ["->", "<-", "<->", "=>", "==", "!=", "<=", ">=", "/*", "*/"] {
        let (plain_a, _) = shape(&font, &text[..1]);
        let (plain_b, _) = shape(&font, &text[1..]);
        let (joined, _) = shape(&font, text);
        assert_ne!(joined, vec![plain_a[0], plain_b[0]], "{text} did not join");
    }

    eprintln!("ligatures: -> shapes to {} glyphs", joined.len());
}
