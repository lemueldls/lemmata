//! The ported glyph definitions.
//!
//! Each module here is one `.ptl` file from Iosevka's `font-glyphs` package,
//! ported to Rust and named after it. The tree mirrors upstream's, so a module
//! can be read against its source and the port's progress can be counted
//! against the same tree:
//!
//! ```text
//! crates/lemmata-glyphs/src/letter/latin/lower_a.rs
//! vendor/Iosevka/packages/font-glyphs/src/letter/latin/lower-a.ptl
//! ```
//!
//! The layer these definitions are written in is [`lemmata_ptl`]. A definition
//! reads its metrics and frame from there, builds shapes with the kit, and
//! accumulates them into a [`lemmata_ptl::glyph::Glyph`].
//!
//! Porting is incremental. A module appears once its shape logic has been
//! compared against the pinned build's own output, which is the only oracle
//! available for a `.ptl` file: the language is compiled, so upstream's code
//! cannot be imported and run the way the geometry and kit packages can.

pub mod letter;
pub mod registry;
pub mod symbol;
