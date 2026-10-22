//! The small punctuation from `symbol/punctuation/small.ptl`.
//!
//! Ported: the period, colon, comma and semicolon in their round and square
//! forms. The raised, stacked, reversed and multi-dot forms are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    control::Item,
    metrics::Metrics,
    shapes::{self, Design},
    spiro_kit::Kit,
    util::mix,
};

/// The period and colon glyphs this module builds.
pub struct Small<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl Small<'_> {
    /// `period.<form>`.
    fn period(&self, square: bool) -> Geometry {
        let metrics = self.metrics;
        let middle = metrics.middle;
        let radius = metrics.period_radius
            * if square {
                metrics.square_dot_scalar
            } else {
                1.0
            };
        let centre = metrics.period_radius
            * if square {
                metrics.square_dot_scalar
            } else {
                1.0
            };
        let radius = if square { centre } else { radius - metrics.o };

        if square {
            spiro_geometry(shapes::rect(
                self.kit,
                centre + radius,
                centre - radius,
                middle - radius,
                middle + radius,
            ))
        } else {
            spiro_geometry(shapes::dot_at(self.kit, metrics, middle, centre, radius))
        }
    }

    /// `xhPeriod.<form>`.
    fn xh_period(&self, square: bool) -> Geometry {
        let metrics = self.metrics;
        let middle = metrics.middle;
        let centre = if square {
            metrics.xh - metrics.period_radius * metrics.square_dot_scalar
        } else {
            metrics.xh - metrics.period_radius
        };
        let radius = if square {
            metrics.period_radius * metrics.square_dot_scalar
        } else {
            metrics.period_radius - metrics.o
        };

        if square {
            spiro_geometry(shapes::rect(
                self.kit,
                centre + radius,
                centre - radius,
                middle - radius,
                middle + radius,
            ))
        } else {
            spiro_geometry(shapes::dot_at(self.kit, metrics, middle, centre, radius))
        }
    }

    /// `CommaShape`: the round fill and its tail, or the square block and its
    /// tail, translated to the middle of the frame.
    fn comma(&self, square: bool) -> Geometry {
        let metrics = self.metrics;
        let k = if square {
            metrics.square_dot_scalar
        } else {
            1.0
        };
        let r = metrics.period_radius * k;
        let ro = r - if square { 0.0 } else { metrics.o };
        let sw = metrics.advice_stroke(4.0, 1.0, 1.0).min(r * 0.8);
        let sw_end = metrics.advice_stroke(2.0, 1.0, 1.0).min(r);
        let comma_overflow = r - sw_end / 2.0;
        let r_fill = ro - sw / 4.0;
        let knot = |ty, x, y, af| self.kit.knot(ty, x, y, af);
        let tail_end = mix(0.0, metrics.descender, metrics.tan_slope);
        let quad = |sign: f64| {
            self.kit.quad_controls(
                sign * metrics.tan_slope / 2.0,
                0.5 - metrics.tan_slope * (r / metrics.descender),
                12,
                Vec::new(),
            )
        };

        let body = if square {
            let block = spiro_geometry(shapes::rect(self.kit, 2.0 * r, 0.0, -r, r));
            let tail = self
                .kit
                .dispiro(vec![
                    knot(KnotType::Left, r, r, self.kit.widths_rhs(Some(sw))),
                    knot(KnotType::Right, r, 0.0, self.kit.widths_rhs(Some(sw))),
                    quad(1.0),
                    knot(
                        KnotType::G4,
                        tail_end + comma_overflow,
                        metrics.descender,
                        self.kit.widths_rhs(Some(sw_end)),
                    ),
                ])
                .geometry(kurbo::Affine::IDENTITY);
            Geometry::Combine(vec![block, tail])
        } else {
            let fill = spiro_geometry(self.kit.spiro_outline(vec![
                knot(KnotType::G4, r_fill, r_fill, Vec::new()),
                self.kit.arcvh(None, None),
                knot(KnotType::G4, 0.0, 0.0, Vec::new()),
                self.kit.archv(None, None),
                knot(KnotType::G4, -r_fill, r_fill, Vec::new()),
                self.kit.arcvh(None, None),
                knot(KnotType::G4, 0.0, 2.0 * r_fill, Vec::new()),
                self.kit.archv(None, None),
                Item::Close,
            ]));
            let tail = self
                .kit
                .dispiro(vec![
                    knot(KnotType::G4, r, r, self.kit.widths_rhs(Some(sw / 4.0))),
                    self.kit.arcvh(None, None),
                    knot(
                        KnotType::G4,
                        0.0,
                        metrics.o,
                        self.kit.widths_rhs(Some(sw / 2.0)),
                    ),
                    self.kit.archv(None, None),
                    knot(KnotType::G4, -ro, r, Vec::new()),
                    self.kit.arcvh(None, None),
                    knot(KnotType::G4, 0.0, r + ro, Vec::new()),
                    self.kit.archv(None, None),
                    knot(KnotType::G4, ro, r, self.kit.widths_rhs(Some(sw))),
                    quad(1.0),
                    knot(
                        KnotType::G4,
                        tail_end - comma_overflow,
                        metrics.descender,
                        self.kit.widths_rhs(Some(sw_end)),
                    ),
                ])
                .geometry(kurbo::Affine::IDENTITY);
            Geometry::Combine(vec![fill, tail])
        };

        Geometry::Transformed {
            tf: kurbo::Affine::translate((metrics.middle, 0.0)),
            geom: Box::new(body),
        }
    }

    /// `semicolon`: the comma and the x-height period.
    fn semicolon(&self, square: bool) -> Geometry {
        Geometry::Combine(vec![self.comma(square), self.xh_period(square)])
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for (suffix, square) in [("round", false), ("square", true)] {
            out.push((format!("comma.{suffix}"), self.comma(square)));
            out.push((format!("semicolon.{suffix}"), self.semicolon(square)));
            out.push((format!("period.{suffix}"), self.period(square)));
            out.push((format!("xhPeriod.{suffix}"), self.xh_period(square)));
            out.push((
                format!("colon.{suffix}"),
                Geometry::Combine(vec![self.period(square), self.xh_period(square)]),
            ));
        }

        out
    }
}

fn spiro_geometry(outline: lemmata_ptl::spiro_kit::SpiroOutline) -> Geometry {
    Geometry::Spiro {
        gizmo: kurbo::Affine::IDENTITY,
        closed: outline.closed,
        knots: outline.knots,
    }
}
