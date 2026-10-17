//! Geometry primitives for glyph construction.

pub mod boolean;
pub mod expand;
pub mod pen;
pub mod quadify;
pub mod spiro;
pub mod stroke;

pub use expand::{BiKnot, ExpandResult, MonoKnot, SpiroExpander, di_spiro_expand, di_spiro_paths};
pub use pen::{PenExpander, PenKnot, pen_spiro_paths};
pub use quadify::{QuadifyKind, QuadifyPoint, QuadifySink, quadify};
pub use spiro::{Knot, KnotType, SpiroArc, spiro_arcs, spiro_path};
pub use stroke::{Bez3, stroke_arcs, stroke_offsets};
