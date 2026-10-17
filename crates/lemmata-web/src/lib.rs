//! The browser build.
//!
//! The generator is one Rust core with two front ends, so this crate is the
//! browser one: the same build the CLI runs, behind a `wasm-bindgen` surface.
//! [`build`] is the whole of it and stays free of the wasm types, so it runs
//! under `cargo test` on the host as well as in the browser.
//!
//! The input is a glyph capture, which is scaffolding. Once the definitions
//! generate outlines on their own this takes a plan instead, and the call shape
//! is the part that survives: bytes in, font bytes out.

use lemmata_build::font::{GlyphData, Metrics, Names};
use wasm_bindgen::prelude::*;

/// Rounds a captured advance to the units the font table stores.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn advance_to_u16(advance: f64) -> u16 {
    advance.round().clamp(0.0, f64::from(u16::MAX)) as u16
}

/// Builds a font from a capture.
///
/// # Errors
///
/// Returns a message when the capture does not decode or the font does not
/// assemble.
pub fn build(capture: &[u8]) -> Result<Vec<u8>, String> {
    let captured = lemmata_capture::load(capture)?;
    let glyphs: Vec<GlyphData> = captured
        .iter()
        .map(|glyph| GlyphData {
            name: glyph.name.clone(),
            unicodes: glyph.unicodes.clone(),
            advance: advance_to_u16(glyph.advance),
            contours: lemmata_capture::contours(glyph),
        })
        .collect();

    lemmata_build::font::build(&glyphs, &Metrics::default(), &Names::default())
}

/// The same build, for the page.
///
/// # Errors
///
/// Returns the message from [`build`] as a JavaScript string.
#[wasm_bindgen]
pub fn build_font(capture: &[u8]) -> Result<Vec<u8>, JsValue> {
    build(capture).map_err(|message| JsValue::from_str(&message))
}

/// The name the page offers the download under.
#[must_use]
#[wasm_bindgen]
pub fn output_name() -> String {
    "Lemmata-Regular.ttf".to_string()
}
