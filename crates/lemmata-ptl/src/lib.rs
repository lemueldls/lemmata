//! The PTL runtime: the layer glyph definitions are written in.
//!
//! Iosevka writes its glyphs in PTL, a language compiled by a set of JavaScript
//! packages. This crate is the Rust port of those packages, which is what makes
//! a ported definition a Rust function rather than a build step. The `.ptl`
//! files themselves are `lemmata-glyphs`.
//!
//! The split follows upstream's own: a module here corresponds to a JavaScript
//! package, and everything here is fixed and complete. The content that grows
//! lives in `lemmata-glyphs`.
//!
//! - [`util`] is `@iosevka/util`: the numeric vocabulary definitions are
//!   written with.
//! - [`control`] is `@iosevka/geometry/spiro-control`: controls, the flattener
//!   that resolves interpolators, and the collector that turns them into
//!   bi-knots.
//! - [`spiro_kit`] is `@iosevka/font-kits/spiro-kit`: the knot builders,
//!   amendments and interpolators a definition calls.
//! - [`glyph`] and [`store`] are `@iosevka/glyph`: a glyph under construction,
//!   the procs that defer work, the boolean combinators, and the store a
//!   definition saves into.
//! - [`metrics`] is `meta/aesthetics.ptl`'s `calculateMetrics` together with
//!   `font-kits/weight-control.mjs`: the numbers every definition reads.
//! - [`letter_model`] is `@iosevka/font-kits/letter-model`: the per-glyph frame.
//! - [`shapes`] is the part of `common/shapes.ptl` that definitions call as
//!   vocabulary rather than import as a shape: the hook, the arch and the
//!   flat-sided ring.
//!
//! Each of these is compared against upstream's own code by a test in
//! `tests/`. The definitions are compared against the build's output, which is
//! the only oracle available for `.ptl` files.

pub mod control;
pub mod glyph;
pub mod letter_model;
pub mod metrics;
pub mod param;
pub mod serif;
pub mod shapes;
pub mod shoulder;
pub mod spiro_kit;
pub mod store;
pub mod util;
