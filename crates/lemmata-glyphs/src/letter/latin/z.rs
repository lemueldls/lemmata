//! The letters `Z` and `z` from `letter/latin/z.ptl`.
//!
//! Ported: the straight and curly bodies over the four serif settings, for the
//! capital and the lower case. The cursive body, the crossbar overlays, the
//! hooked and swash forms, `Z.PLK` and the Cyrillic forms are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    metrics::Metrics,
    shapes::{self, Design, VSerifSide},
    spiro_kit::{Dispiro, Kit},
};

use super::x::x_strand;

/// The body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Straight,
    Curly,
}

/// The serif setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serif {
    Serifless,
    Serifed,
    BottomSerifed,
    TopSerifed,
}

/// The `Z` and `z` families this module builds.
pub struct LetterZ<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LetterZ<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    /// `ZBaseShape`.
    fn straight(&self, top: f64) -> Geometry {
        let metrics = self.metrics;
        let sb = metrics.sb;
        let right_sb = metrics.width - metrics.sb;
        let cor = 1.15 * metrics.hv_contrast;
        Geometry::Combine(vec![
            shapes::hbar_t(self.kit, metrics, sb, right_sb, top, None)
                .geometry(kurbo::Affine::IDENTITY),
            spiro_geometry(self.kit.spiro_outline(vec![
                self.kit.knot(KnotType::Corner, sb, metrics.stroke, Vec::new()),
                self.kit.knot(
                    KnotType::Corner,
                    right_sb - metrics.stroke * cor,
                    top - metrics.stroke,
                    Vec::new(),
                ),
                self.kit.knot(
                    KnotType::Corner,
                    right_sb,
                    top - metrics.stroke,
                    Vec::new(),
                ),
                self.kit.knot(
                    KnotType::Corner,
                    sb + metrics.stroke * cor,
                    metrics.stroke,
                    Vec::new(),
                ),
            ])),
            shapes::hbar_b(self.kit, metrics, sb, right_sb, 0.0, None)
                .geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// `ZCurlyShape`.
    fn curly(&self, top: f64) -> Geometry {
        let metrics = self.metrics;
        let sb = metrics.sb;
        let right_sb = metrics.width - metrics.sb;
        Geometry::Combine(vec![
            shapes::hbar_t(
                self.kit,
                metrics,
                sb,
                right_sb - self.hsw_to_v(metrics.stroke),
                top,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
            x_strand(
                self.kit,
                metrics,
                false,
                metrics.slab != 0.0,
                sb,
                0.0,
                right_sb,
                top,
                0.125,
                0.4,
                0.32,
                None,
            ),
            shapes::hbar_b(
                self.kit,
                metrics,
                sb + self.hsw_to_v(metrics.stroke),
                right_sb,
                0.0,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// `SmallZBaseShape`.
    fn small_straight(&self, top: f64) -> Geometry {
        let metrics = self.metrics;
        let sb = metrics.sb;
        let right_sb = metrics.width - metrics.sb;
        Geometry::Combine(vec![
            shapes::hbar_t(self.kit, metrics, sb, right_sb, top, None)
                .geometry(kurbo::Affine::IDENTITY),
            ext_line_rhs_to_lhs(
                self.kit,
                sb,
                metrics.stroke,
                right_sb,
                top - metrics.stroke,
                metrics.stroke / metrics.hv_contrast,
            )
            .geometry(kurbo::Affine::IDENTITY),
            shapes::hbar_b(self.kit, metrics, sb, right_sb, 0.0, None)
                .geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// `SmallZCurlyShape`.
    fn small_curly(&self, top: f64) -> Geometry {
        let metrics = self.metrics;
        let sb = metrics.sb;
        let right_sb = metrics.width - metrics.sb;
        Geometry::Combine(vec![
            shapes::hbar_t(
                self.kit,
                metrics,
                sb,
                right_sb - self.hsw_to_v(metrics.stroke),
                top,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
            x_strand(
                self.kit,
                metrics,
                false,
                metrics.slab != 0.0,
                sb,
                0.0,
                right_sb,
                top,
                0.125,
                0.4,
                0.36,
                None,
            ),
            shapes::hbar_b(
                self.kit,
                metrics,
                sb + self.hsw_to_v(metrics.stroke),
                right_sb,
                0.0,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// `ZStdSerifs`, `ZMotionSerifsB` and `ZMotionSerifsT`.
    fn serif(&self, serif: Serif, top: f64) -> Option<Geometry> {
        let metrics = self.metrics;
        let sb = metrics.sb;
        let right_sb = metrics.width - metrics.sb;
        let dl = || {
            shapes::vserif(
                self.kit,
                metrics,
                VSerifSide::DownLeft,
                sb,
                top,
                metrics.v_jut,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY)
        };
        let ur = || {
            shapes::vserif(
                self.kit,
                metrics,
                VSerifSide::UpRight,
                right_sb,
                0.0,
                metrics.v_jut,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY)
        };

        Some(match serif {
            Serif::Serifless => return None,
            Serif::Serifed => Geometry::Combine(vec![dl(), ur()]),
            Serif::BottomSerifed => ur(),
            Serif::TopSerifed => dl(),
        })
    }

    /// One `Z.<suffix>` or `z.<suffix>`.
    #[must_use]
    pub fn glyph(&self, body: Body, serif: Serif, capital: bool) -> Geometry {
        let top = if capital {
            self.metrics.cap
        } else {
            self.metrics.xh
        };
        let mut parts = vec![match (body, capital) {
            (Body::Straight, true) => self.straight(top),
            (Body::Straight, false) => self.small_straight(top),
            (Body::Curly, true) => self.curly(top),
            (Body::Curly, false) => self.small_curly(top),
        }];
        if let Some(serif) = self.serif(serif, top) {
            parts.push(serif);
        }

        Geometry::Combine(parts)
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for (body_key, body) in BODIES {
            for (serif_key, serif) in SERIFS {
                out.push((
                    format!("Z.{body_key}{serif_key}"),
                    self.glyph(*body, *serif, true),
                ));
                out.push((
                    format!("z.{body_key}{serif_key}"),
                    self.glyph(*body, *serif, false),
                ));
            }
        }

        out
    }
}

/// The body suffixes.
pub const BODIES: &[(&str, Body)] = &[("straight", Body::Straight), ("curly", Body::Curly)];

/// The serif suffixes.
pub const SERIFS: &[(&str, Serif)] = &[
    ("Serifless", Serif::Serifless),
    ("Serifed", Serif::Serifed),
    ("BottomSerifed", Serif::BottomSerifed),
    ("TopSerifed", Serif::TopSerifed),
];

/// `ExtLineRhsToLhs`: a line whose right end is the reference stroke.
#[allow(clippy::too_many_arguments)]
fn ext_line_rhs_to_lhs(kit: &Kit, x1: f64, y1: f64, x2: f64, y2: f64, sw: f64) -> Dispiro {
    let items = vec![
        kit.knot(KnotType::Corner, x1, y1, kit.widths_rhs(Some(sw))),
        kit.knot(KnotType::Corner, x2, y2, kit.widths_lhs(Some(sw))),
    ];

    kit.dispiro(items)
}

fn spiro_geometry(outline: lemmata_ptl::spiro_kit::SpiroOutline) -> Geometry {
    Geometry::Spiro {
        gizmo: kurbo::Affine::IDENTITY,
        closed: outline.closed,
        knots: outline.knots,
    }
}
