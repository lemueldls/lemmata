//! The upper-case `I` from `letter/latin/upper-i.ptl`.
//!
//! The file draws one stem at cap height with three serif settings: none, a long
//! serif and a short one. `ILonga` is the long-s form, `smcpI` the small capital
//! and `mathbb/I` the blackboard one. `ISideways`, `AiDiphthong` and the Greek
//! iota need transforms or glyphs outside this file and are not ported.
//!
//! The frame's advance scale is the default one: the parameter layer the port
//! carries fixes the scales at one, which is the plan the capture was built from.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::shapes::{self, Design};
use lemmata_ptl::spiro_kit::Kit;
use lemmata_ptl::util::mix;

/// The three serif settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serif {
    None,
    Long,
    Short,
}

/// The settings, in the table's order.
pub const CONFIGS: &[(&str, Serif)] = &[
    ("straight", Serif::None),
    ("serifless", Serif::None),
    ("serifed", Serif::Long),
    ("shortSerifed", Serif::Short),
];

/// The upper-case `I` under construction.
pub struct UpperI<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl UpperI<'_> {
    const fn middle(&self) -> f64 {
        self.df.width / 2.0
    }

    const fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    /// The serif's reach, which the table varies with the frame's width.
    #[must_use]
    pub fn jut(&self, serif: Serif) -> f64 {
        let metrics = self.metrics;
        let half = self.hsw_to_v(metrics.half_stroke);
        match serif {
            Serif::None => half,
            Serif::Long => metrics
                .mid_jut_center
                .max(mix(half, metrics.long_jut, self.df.adws())),
            Serif::Short => metrics
                .jut
                .max(mix(half, metrics.mid_jut_center, self.df.adws())),
        }
    }

    /// `IShape`: the stem.
    #[must_use]
    pub fn shape(&self, top: f64) -> Geometry {
        shapes::vbar_m(
            self.kit,
            self.metrics,
            self.middle(),
            0.0,
            top,
            self.metrics.stroke,
        )
        .geometry(kurbo::Affine::IDENTITY)
    }

    /// `ISerifs`: the two centered serifs.
    #[must_use]
    pub fn serifs(&self, serif: Serif, top: f64) -> Vec<Geometry> {
        if serif == Serif::None {
            return Vec::new();
        }
        let jut = self.jut(serif);
        vec![
            Geometry::Tagged(Box::new(
                shapes::hserif_mb(self.kit, self.metrics, self.middle(), 0.0, jut, None)
                    .geometry(kurbo::Affine::IDENTITY),
            )),
            Geometry::Tagged(Box::new(
                shapes::hserif_mt(self.kit, self.metrics, self.middle(), top, jut, None)
                    .geometry(kurbo::Affine::IDENTITY),
            )),
        ]
    }

    /// `I.<suffix>` at cap height.
    #[must_use]
    pub fn capital(&self, serif: Serif) -> Geometry {
        let mut parts = vec![self.shape(self.metrics.cap)];
        parts.extend(self.serifs(serif, self.metrics.cap));

        Geometry::Combine(parts)
    }

    /// `ILonga.<suffix>`: the long s's height.
    #[must_use]
    pub fn longa(&self, serif: Serif) -> Geometry {
        let top = self.metrics.cap + (self.metrics.ascender - self.metrics.xh);
        let mut parts = vec![self.shape(top)];
        parts.extend(self.serifs(serif, top));

        Geometry::Combine(parts)
    }

    /// `smcpI`: the small capital, whose serifs are the middle jut.
    #[must_use]
    pub fn small(&self) -> Geometry {
        let mut parts = vec![self.shape(self.metrics.xh)];
        parts.push(Geometry::Tagged(Box::new(
            shapes::hserif_mb(
                self.kit,
                self.metrics,
                self.middle(),
                0.0,
                self.metrics.mid_jut_center,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        )));
        parts.push(Geometry::Tagged(Box::new(
            shapes::hserif_mt(
                self.kit,
                self.metrics,
                self.middle(),
                self.metrics.xh,
                self.metrics.mid_jut_center,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        )));

        Geometry::Combine(parts)
    }

    /// `mathbb/I`.
    #[must_use]
    pub fn mathbb(&self) -> Geometry {
        let metrics = self.metrics;
        let bbs = shapes::bb_stroke(self.df);
        let bbd = shapes::bb_distance(metrics, bbs);
        let middle = metrics.width / 2.0;
        let jut = metrics.jut;
        let mut parts =
            shapes::bb_bar_center(self.kit, metrics, middle, 0.0, metrics.cap, bbs, bbd);
        parts.push(
            shapes::hbar_t(
                self.kit,
                metrics,
                middle - bbd / 2.0 - jut,
                middle + bbd / 2.0 + jut,
                metrics.cap,
                Some(bbs),
            )
            .geometry(kurbo::Affine::IDENTITY),
        );
        parts.push(
            shapes::hbar_b(
                self.kit,
                metrics,
                middle - bbd / 2.0 - jut,
                middle + bbd / 2.0 + jut,
                0.0,
                Some(bbs),
            )
            .geometry(kurbo::Affine::IDENTITY),
        );

        Geometry::Combine(parts)
    }
}

impl UpperI<'_> {
    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = vec![
            ("smcpI".to_string(), self.small()),
            ("mathbb/I".to_string(), self.mathbb()),
        ];
        for (suffix, serif) in CONFIGS {
            out.push((format!("I.{suffix}"), self.capital(*serif)));
            out.push((format!("ILonga.{suffix}"), self.longa(*serif)));
        }

        out
    }
}
