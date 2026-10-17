//! Geometry primitives for glyph construction.

pub mod boolean;
pub mod spiro;

pub use spiro::{Knot, KnotType, SpiroArc, spiro_arcs, spiro_path};
