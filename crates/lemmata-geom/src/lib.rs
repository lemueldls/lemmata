//! Geometry primitives for glyph construction.

pub mod boolean;
pub mod expand;
pub mod fairize;
pub mod fit;
pub mod math;
pub mod outline;
pub mod pen;
pub mod quadify;
pub mod spiro;
pub mod stroke;
pub mod tree;

pub use expand::{BiKnot, ExpandResult, MonoKnot, SpiroExpander, di_spiro_expand, di_spiro_paths};
pub use fairize::{CombinedArc, FairArc, fairize};
pub use pen::{PenExpander, PenKnot, pen_spiro_paths};
pub use quadify::{QuadifyKind, QuadifyPoint, QuadifySink, quadify};
pub use spiro::{Knot, KnotType, SpiroArc, spiro_arcs, spiro_path};
pub use stroke::{Bez3, paths_to_shape, shape_to_paths, stroke_arcs, stroke_offsets};
pub use tree::Geometry;
