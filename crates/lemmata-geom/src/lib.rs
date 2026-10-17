//! Geometry primitives for glyph construction.

pub mod boolean;
pub mod expand;
pub mod spiro;

pub use expand::{BiKnot, ExpandResult, MonoKnot, SpiroExpander, di_spiro_expand, di_spiro_paths};
pub use spiro::{Knot, KnotType, SpiroArc, spiro_arcs, spiro_path};
