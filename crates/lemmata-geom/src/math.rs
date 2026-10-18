//! The math the pipeline is allowed to use.
//!
//! Rust's `f64` methods call whatever the target provides. On the host that is
//! the C library, on `wasm32-unknown-unknown` it is Rust's own port of it, and
//! the two disagree in the last bit for most of the transcendental functions:
//! `sin`, `cos`, `tan`, `tanh`, `exp`, `ln`, `log2` and `pow` all differ.
//!
//! That disagreement is not academic here. The stroke width comes out of a
//! `tanh` sigmoid, the spiro solve and the quadifier are driven by `sin`, `cos`
//! and `pow`, and a coordinate that moves by one unit in the last place can
//! land on the other side of a rounding boundary. The outline then subdivides
//! into a different number of points and the font bytes differ.
//!
//! So the pipeline computes these through `libm`, which is one implementation
//! compiled for every target. That is what makes the native build and the
//! browser build byte-identical, which `r[wasm.parity-bytes]` requires.
//!
//! The exact functions stay on the primitive: `sqrt` is correctly rounded by
//! IEEE 754, and `abs`, `floor`, `ceil`, `round` and `trunc` are exact. So does
//! `kurbo`'s `Vec2::hypot`, which is written as `sqrt(x² + y²)` rather than
//! `f64::hypot` on purpose.

/// `sin`.
#[must_use]
pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}

/// `cos`.
#[must_use]
pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}

/// `tan`.
#[must_use]
pub fn tan(x: f64) -> f64 {
    libm::tan(x)
}

/// `atan2`, which the CLI uses for the italic angle.
#[must_use]
pub fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

/// `tanh`, which the weight-control sigmoid is built on.
#[must_use]
pub fn tanh(x: f64) -> f64 {
    libm::tanh(x)
}

/// `atanh`.
#[must_use]
pub fn atanh(x: f64) -> f64 {
    libm::atanh(x)
}

/// `exp`.
#[must_use]
pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}

/// `ln`.
#[must_use]
pub fn ln(x: f64) -> f64 {
    libm::log(x)
}

/// `log2`, which the fairizer uses to snap a length to a power of two.
#[must_use]
pub fn log2(x: f64) -> f64 {
    libm::log2(x)
}

/// `pow`.
#[must_use]
pub fn pow(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}

/// `hypot`, for the few places that need it without a `kurbo` vector.
#[must_use]
pub fn hypot(x: f64, y: f64) -> f64 {
    libm::hypot(x, y)
}
