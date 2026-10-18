//! Builds the font the way the browser does and checks it came out whole.
//!
//! Skips without a capture. The same test body runs under `wasm-pack test`
//! when the capture is embedded, and the page's end-to-end run is the other
//! half: the browser produces the bytes and they are compared against the
//! host's.

#![allow(clippy::cast_precision_loss)]

use std::fs;
use std::path::{Path, PathBuf};

use read_fonts::TableProvider;

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

#[test]
fn builds_a_font_from_the_capture() {
    let Ok(capture) = fs::read(capture_path()) else {
        eprintln!("skipping: no capture");
        return;
    };

    let bytes = lemmata_web::build(&capture, "").expect("build");

    // The font has to be readable by something other than the writer, so it
    // is reopened the way a renderer would open it.
    let font = read_fonts::FontRef::new(&bytes).expect("reopen");
    assert!(
        font.table_data(read_fonts::types::Tag::new(b"glyf"))
            .is_some()
    );
    assert!(
        font.table_data(read_fonts::types::Tag::new(b"cmap"))
            .is_some()
    );

    // Two runs of the same input have to agree, since the page has no other
    // way to tell a good build from a corrupted one.
    let again = lemmata_web::build(&capture, "").expect("build again");
    assert_eq!(bytes, again, "the build is not deterministic");

    // A plan reaches the font, so the page's controls are not decoration.
    let taller = lemmata_web::build(&capture, r#"{"capHeight": 800}"#).expect("build with a plan");
    assert_ne!(bytes, taller, "the plan did not reach the font");

    eprintln!("web build: {} bytes", bytes.len());
}

/// `r[plan.iosevka-keys]`: a plan's character choice reaches the font, and the
/// page's list of choices comes from the capture rather than from the page.
#[test]
fn a_plan_reaches_a_character_variant() {
    let Ok(capture) = fs::read(capture_path()) else {
        eprintln!("skipping: no capture");
        return;
    };

    // The page offers what the capture holds, spelled the way a plan takes it.
    let listed = lemmata_web::variants_of(&capture, "a");
    for name in ["double-storey-serifed", "single-storey-serifed"] {
        assert!(listed.contains(name), "{name} is not in {listed}");
    }
    assert_eq!(
        lemmata_web::variants_of(&capture, "no-such-group"),
        "[]",
        "a group the capture does not have offers nothing"
    );

    let plain = lemmata_web::build(&capture, "").expect("build");
    let chosen = lemmata_web::build(&capture, r#"{"variants": {"a": "single-storey-serifed"}}"#)
        .expect("build with a variant");
    assert_ne!(plain, chosen, "the variant did not reach the font");

    // The codepoint still lands on the glyph named `a`, because every feature
    // the font writes names glyphs and a cmap that pointed at the variant's own
    // name would leave them all matching nothing.
    let font = read_fonts::FontRef::new(&chosen).expect("reopen");
    assert!(
        font.cmap()
            .expect("cmap")
            .map_codepoint(u32::from('a'))
            .is_some(),
        "the choice lost the codepoint"
    );
    eprintln!("web variants: {}", listed.len());
}
