//! The upper-case `L` from `letter/latin/upper-l.ptl`.
//!
//! The letter is a stem and a foot bar. Three serif grades add nothing, a foot
//! serif, or the foot and shoulder serifs. `smcpL` is the small capital,
//! `LSlash` and `smcpLSlash` overlay it, `LHighBar` adds a bar and `mathbb/L` is
//! the blackboard form.
//!
//! `LLWelsh` needs the mask shapes, and the turned letters, accents and belt
//! overlays need glyphs outside this file, so they are not ported. The anchors
//! are not either.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::{
    letter_model::DivFrame,
    metrics::Metrics,
    shapes::{self, Design, HSerifSide, VSerifSide},
    spiro_kit::Kit,
    util::mix,
};

/// The three serif grades.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serif {
    Serifless,
    MotionSerifed,
    Serifed,
}

impl Serif {
    const fn grade(self) -> i32 {
        match self {
            Self::Serifless => 0,
            Self::MotionSerifed => 1,
            Self::Serifed => 2,
        }
    }
}

/// The settings, in the table's order.
pub const CONFIGS: &[(&str, Serif)] = &[
    ("serifless", Serif::Serifless),
    ("motionSerifed", Serif::MotionSerifed),
    ("serifed", Serif::Serifed),
];

/// The upper-case `L` under construction.
pub struct UpperL<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl UpperL<'_> {
    const K_SB: f64 = 1.5;

    const fn x_bar_left(df: &DivFrame) -> f64 {
        mix(0.0, df.sb, Self::K_SB)
    }

    fn x_right(&self, df: &DivFrame) -> f64 {
        (df.width - df.sb) - 0.75 * self.metrics.ox
    }

    const fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    /// `LShape`: the stem, the foot bar and the serifs.
    #[must_use]
    pub fn shape(&self, df: &DivFrame, top: f64, serif: Serif, sw: f64) -> Geometry {
        let metrics = self.metrics;
        let x_left = Self::x_bar_left(df);
        let x_right = self.x_right(df);
        let grade = serif.grade();
        let mut parts = vec![
            shapes::vbar_l(self.kit, metrics, x_left, 0.0, top, sw)
                .geometry(kurbo::Affine::IDENTITY),
            shapes::hbar_b(self.kit, metrics, x_left - metrics.o, x_right, 0.0, None)
                .geometry(kurbo::Affine::IDENTITY),
        ];
        if grade > 0 {
            parts.push(
                shapes::vserif(
                    self.kit,
                    metrics,
                    VSerifSide::UpRight,
                    x_right,
                    0.0,
                    metrics.v_jut,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            );
        }
        if grade > 1 {
            let shrink = self.hsw_to_v(0.5 * (metrics.stroke - sw));
            let x = x_left + self.hsw_to_v(0.5 * sw);
            parts.push(
                shapes::hserif(
                    self.kit,
                    metrics,
                    HSerifSide::LeftBottom,
                    x,
                    0.0,
                    metrics.jut - shrink,
                    sw,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            );
            parts.push(Geometry::Combine(vec![
                shapes::hserif(
                    self.kit,
                    metrics,
                    HSerifSide::LeftTop,
                    x,
                    top,
                    metrics.jut - shrink,
                    sw,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
                shapes::hserif(
                    self.kit,
                    metrics,
                    HSerifSide::RightTop,
                    x,
                    top,
                    metrics.mid_jut_side - shrink,
                    sw,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            ]));
        }

        Geometry::Combine(parts)
    }

    /// `LSlashOverlay`.
    #[must_use]
    pub fn slash_overlay(&self, df: &DivFrame, top: f64, serif: Serif) -> Geometry {
        let metrics = self.metrics;
        let fine = 0.5 * metrics.overlay_stroke;
        let top_gap = if serif.grade() > 1 {
            metrics.stroke
        } else {
            0.0
        };
        let y_middle = mix(metrics.stroke, top - top_gap, 0.5);
        let x = mix(0.0, df.sb, 0.5);
        let slash = self.kit.dispiro(vec![
            lemmata_ptl::control::Item::Af(self.kit.widths_center(Some(2.0 * fine))),
            self.kit.knot(
                lemmata_geom::KnotType::Left,
                x,
                y_middle - metrics.long_v_jut * 0.4,
                Vec::new(),
            ),
            self.kit.knot(
                lemmata_geom::KnotType::Right,
                x + 1.6 * metrics.long_jut,
                y_middle + metrics.long_v_jut * 0.4,
                Vec::new(),
            ),
        ]);

        slash.geometry(kurbo::Affine::IDENTITY)
    }

    /// `L.<suffix>`.
    #[must_use]
    pub fn capital(&self, serif: Serif) -> Geometry {
        self.shape(self.df, self.metrics.cap, serif, self.metrics.stroke)
    }

    /// `LSlash.<suffix>`.
    #[must_use]
    pub fn slash(&self, serif: Serif) -> Geometry {
        Geometry::Combine(vec![
            self.capital(serif),
            self.slash_overlay(self.df, self.metrics.cap, serif),
        ])
    }

    /// `smcpL.<suffix>`.
    #[must_use]
    pub fn small(&self, serif: Serif) -> Geometry {
        self.shape(self.df, self.metrics.xh, serif, self.metrics.stroke)
    }

    /// `smcpLSlash.<suffix>`.
    #[must_use]
    pub fn small_slash(&self, serif: Serif) -> Geometry {
        Geometry::Combine(vec![
            self.small(serif),
            self.slash_overlay(self.df, self.metrics.xh, serif),
        ])
    }

    /// `LHighBar.<suffix>`.
    #[must_use]
    pub fn high_bar(&self, serif: Serif) -> Geometry {
        let metrics = self.metrics;
        let top = metrics.cap
            - if serif.grade() > 1 {
                metrics.stroke
            } else {
                0.0
            };
        let bar = shapes::letter_bar_overlay_l_in(
            self.kit,
            metrics,
            Self::x_bar_left(self.df),
            metrics.xh,
            top,
            0.5,
            (0.0, metrics.width),
            metrics.stroke,
            metrics.overlay_stroke,
        );

        Geometry::Combine(vec![
            self.capital(serif),
            bar.geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// `mathbb/L`.
    #[must_use]
    pub fn mathbb(&self) -> Geometry {
        let metrics = self.metrics;
        let bbs = shapes::bb_stroke(self.df);
        let bbd = shapes::bb_distance(metrics, bbs);
        let x_left = Self::x_bar_left(self.df);
        let x_right = self.x_right(self.df);

        Geometry::Combine(vec![
            shapes::vbar_l(self.kit, metrics, x_left, 0.0, metrics.cap, bbs)
                .geometry(kurbo::Affine::IDENTITY),
            shapes::vbar_l(self.kit, metrics, x_left + bbd, 0.0, metrics.cap, bbs)
                .geometry(kurbo::Affine::IDENTITY),
            shapes::hbar_b(
                self.kit,
                metrics,
                x_left - metrics.o,
                x_right,
                0.0,
                Some(bbs),
            )
            .geometry(kurbo::Affine::IDENTITY),
            shapes::hbar_t(
                self.kit,
                metrics,
                x_left,
                x_left + bbd,
                metrics.cap,
                Some(bbs),
            )
            .geometry(kurbo::Affine::IDENTITY),
        ])
    }
}

impl UpperL<'_> {
    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = vec![("mathbb/L".to_string(), self.mathbb())];
        for (suffix, serif) in CONFIGS {
            out.push((format!("L.{suffix}"), self.capital(*serif)));
            out.push((format!("LSlash.{suffix}"), self.slash(*serif)));
            out.push((format!("smcpL.{suffix}"), self.small(*serif)));
            out.push((format!("smcpLSlash.{suffix}"), self.small_slash(*serif)));
            out.push((format!("LHighBar.{suffix}"), self.high_bar(*serif)));
        }

        out
    }
}
