//! Builds the font the way the browser does and checks it came out whole.
//!
//! Skips without a capture. The same test body runs under `wasm-pack test`
//! when the capture is embedded, and the page's end-to-end run is the other
//! half: the browser produces the bytes and they are compared against the
//! host's.

#![allow(clippy::cast_precision_loss)]

use std::fs;
use std::path::{Path, PathBuf};

fn capture_path() -> PathBuf {
    std::env::var_os("LEMMATA_GLYPH_DUMP").map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("target/glyphs.bin")
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
