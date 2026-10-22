//! The dot-spaced math signs from `symbol/math/dots-and-colons.ptl`.
//!
//! The file's dot table gives two variants, round and square. Each sign is
//! built from the same radius, so `therefore` and `because` are mirror images
//! and `mathcolon`/`coloncolon` are the same dots at different positions.
//!
//! `mathCDotInner`, the wide `mathCDotBig` and the derived `mathBigCircled`
//! forms are drawn here where the file defines them, but only the names the
//! default plan builds are compared against the capture.

use lemmata_geom::Geometry;
use lemmata_ptl::{metrics::Metrics, shapes, spiro_kit::Kit};

/// The two dot shapes, as `DotVariants` names them.
pub const DOT_VARIANTS: &[(&str, bool)] = &[("round", false), ("square", true)];

/// The dot-spaced signs this module builds.
pub struct DotsAndColons<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
}

impl DotsAndColons<'_> {
    /// The dot radius for a variant. `radius1` is the design radius the wide
    /// form keeps, and `radius` is clamped to the space the small signs have.
    fn radii(&self, square: bool) -> (f64, f64) {
        let metrics = self.metrics;
        let k = if square {
            metrics.square_dot_scalar
        } else {
            1.0
        };
        let radius1 = metrics.period_radius * k * metrics.operator_stroke / metrics.stroke;
        let radius = radius1.min(
            0.5 * metrics.advice_stroke(3.5, metrics.width / k, 1.0) * metrics.period_size * k
                / metrics.stroke,
        );

        (radius1, radius)
    }

    /// One `DotAt` or `SquareAt`.
    fn dot(&self, square: bool, x: f64, y: f64, r: f64) -> Geometry {
        let outline = if square {
            shapes::rect(self.kit, y + r, y - r, x - r, x + r)
        } else {
            shapes::dot_at(self.kit, self.metrics, x, y, r)
        };

        Geometry::Spiro {
            gizmo: kurbo::Affine::IDENTITY,
            closed: outline.closed,
            knots: outline.knots,
        }
    }

    /// `therefore`, `because`, `mathcolon`, `coloncolon`, `mathCDot` and
    /// `mathCDotInner` for one dot variant.
    fn variant(&self, suffix: &str, square: bool) -> Vec<(String, Geometry)> {
        let metrics = self.metrics;
        let (_, radius) = self.radii(square);
        let overshoot = if square { 0.0 } else { metrics.o };
        let r = radius - overshoot;
        let left = metrics.width / 4.0;
        let right = metrics.width * 0.75;
        let y_upper = metrics.symbol_mid + metrics.xh / 2.0 - radius;
        let y_lower = metrics.symbol_mid - metrics.xh / 2.0 + radius;
        let middle = metrics.width / 2.0;
        let sign = |points: &[(f64, f64, f64)]| {
            Geometry::Combine(
                points
                    .iter()
                    .map(|(x, y, radius)| self.dot(square, *x, *y, *radius))
                    .collect(),
            )
        };

        vec![
            (
                format!("therefore.{suffix}"),
                sign(&[
                    (middle, y_upper, r),
                    (left, y_lower, r),
                    (right, y_lower, r),
                ]),
            ),
            (
                format!("because.{suffix}"),
                sign(&[
                    (middle, y_lower, r),
                    (left, y_upper, r),
                    (right, y_upper, r),
                ]),
            ),
            (
                format!("mathcolon.{suffix}"),
                sign(&[(middle, y_lower, r), (middle, y_upper, r)]),
            ),
            (
                format!("coloncolon.{suffix}"),
                sign(&[
                    (left, y_upper, r),
                    (right, y_upper, r),
                    (left, y_lower, r),
                    (right, y_lower, r),
                ]),
            ),
            (
                format!("mathCDot.{suffix}"),
                sign(&[(middle, metrics.symbol_mid, r)]),
            ),
            (
                format!("mathCDotInner.{suffix}"),
                sign(&[(
                    middle,
                    metrics.symbol_mid,
                    radius * metrics.operator_stroke / metrics.stroke - overshoot,
                )]),
            ),
        ]
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for (suffix, square) in DOT_VARIANTS {
            out.extend(self.variant(suffix, *square));
        }

        out
    }
}
