//! Reading reference fonts for drift comparison.

pub mod compare;
pub mod extract;
pub mod flatten;

pub use extract::{Error, GlyphOutline, glyph_count, outline, outlines};
