//! The small numeric helpers the glyph definitions are written with.
//!
//! Ported from `packages/util/src/index.mjs`. Only the numeric part: the string,
//! suffix and compiler helpers in that package serve naming and code generation,
//! which do not reach the outlines.
//!
//! `quantize` and the waveforms carry JavaScript's rounding and its `|| 0`
//! fallbacks, which differ from Rust's at the half and at zero.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]
// The parameter names are upstream's, where `a`, `b`, `c` and `d` are the
// control points of a Bezier.
#![allow(clippy::many_single_char_names)]

/// `mix`.
#[must_use]
pub const fn mix(a: f64, b: f64, p: f64) -> f64 {
    a + (b - a) * p
}

/// `barMixL`: mix between two ends with a bar in the middle.
#[must_use]
pub fn bar_mix_l(l: f64, r: f64, b: f64, p: f64) -> f64 {
    if l > r {
        bar_mix_l(r, l, b, p)
    } else {
        l + b + p * (r - l - b * 3.0)
    }
}

/// `linreg`: linear interpolation, extrapolating outside the range.
#[must_use]
pub fn linreg(x0: f64, y0: f64, x1: f64, y1: f64, x: f64) -> f64 {
    y0 + ((x - x0) * (y1 - y0)) / (x1 - x0)
}

/// `clamp`.
#[must_use]
pub const fn clamp(l: f64, h: f64, x: f64) -> f64 {
    if x < l {
        l
    } else if x > h {
        h
    } else {
        x
    }
}

/// JavaScript's `Math.round`: halfway rounds toward positive infinity, where
/// Rust's `round` rounds away from zero.
#[must_use]
pub fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// `quantize`.
#[must_use]
pub fn quantize(x: f64, step_size: f64) -> f64 {
    step_size * js_round(x / step_size)
}

/// `fallback`: the first argument that is defined.
#[must_use]
pub fn fallback(values: &[Option<f64>]) -> Option<f64> {
    values.iter().copied().flatten().next()
}

/// `bez2`.
#[must_use]
pub fn bez2(a: f64, b: f64, c: f64, t: f64) -> f64 {
    (1.0 - t) * (1.0 - t) * a + 2.0 * (1.0 - t) * t * b + t * t * c
}

/// `bez3`.
#[must_use]
pub fn bez3(a: f64, b: f64, c: f64, d: f64, t: f64) -> f64 {
    (1.0 - t) * (1.0 - t) * (1.0 - t) * a
        + 3.0 * (1.0 - t) * (1.0 - t) * t * b
        + 3.0 * t * t * (1.0 - t) * c
        + t * t * t * d
}

/// `boole`.
#[must_use]
pub const fn boole(b: bool) -> f64 {
    if b { 1.0 } else { 0.0 }
}

/// `boolePn`.
#[must_use]
pub const fn boole_pn(b: bool) -> f64 {
    if b { 1.0 } else { -1.0 }
}

/// `strokeOffset`: offset a point along the normal of a direction.
#[must_use]
pub fn stroke_offset(x: f64, y: f64, dx: f64, dy: f64, offset: f64, contrast: f64) -> (f64, f64) {
    let r = dx.hypot(dy);
    (x + (dy / r) * offset * contrast, y - (dx / r) * offset)
}

/// `slY`: the y at `x1` on a line of the given slope through `(x0, y0)`.
#[must_use]
pub const fn sl_y(x0: f64, y0: f64, x1: f64, slope: f64) -> f64 {
    y0 + slope * (x1 - x0)
}

/// `slX`: the x at `y1` on a line of the given slope through `(x0, y0)`.
#[must_use]
pub const fn sl_x(x0: f64, y0: f64, y1: f64, slope: f64) -> f64 {
    x0 + (y1 - y0) / slope
}

/// `distP`: the parameter step that covers `dist` along a segment.
#[must_use]
pub fn dist_p(x0: f64, y0: f64, x1: f64, y1: f64, dist: f64) -> f64 {
    dist / (x1 - x0).hypot(y1 - y0)
}

/// Which waveform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Waveform {
    Sine,
    Square,
    Triangle,
    Sawtooth,
}

impl Waveform {
    #[must_use]
    pub fn eval(self, x: f64) -> f64 {
        match self {
            Self::Sine => (std::f64::consts::PI / 2.0 * x).sin(),
            // The `|| 2` and `|| 1` are JavaScript's: a zero becomes the
            // fallback rather than zero.
            Self::Square => {
                let value = mod_(x - 1.0, 4.0);
                let value = if value == 0.0 { 2.0 } else { value };

                js_sign(value - 2.0)
            }
            Self::Triangle => (mod_(x - 1.0, 4.0) - 2.0).abs() - 1.0,
            Self::Sawtooth => {
                let value = mod_(x - 1.0, 2.0);
                let value = if value == 0.0 { 1.0 } else { value };

                value - 1.0
            }
        }
    }
}

/// JavaScript's `%` on the way to a positive modulus.
fn mod_(n: f64, d: f64) -> f64 {
    ((n % d) + d) % d
}

/// JavaScript's `Math.sign`, which returns zero for zero where Rust's `signum`
/// returns one.
#[must_use]
pub fn js_sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}
