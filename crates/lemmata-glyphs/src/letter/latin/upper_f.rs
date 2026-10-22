//! The upper-case `F` from `letter/latin/upper-f.ptl`.
//!
//! The file builds one letter with four serif settings: the stem, the top bar,
//! a crossbar whose right end pulls in, and the serifs the setting selects. The
//! same drawing at cap height is `F`, at x-height `smcpF`; `FBar` overlays the
//! stem and `currency/frenchFrancSign` replaces the crossbar with two bars.
//!
//! `FHookBottom` and the turned letters need shapes outside this file and are
//! not ported. The anchors are not either.

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

/// The four serif settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SerifConfig {
    Serifless,
    TopLeftSerifed,
    Serifed,
    SerifedCapped,
}

impl SerifConfig {
    /// The tuple the table carries: left top, left bottom, right top, right
    /// middle.
    const fn flags(self) -> (bool, bool, bool, bool) {
        match self {
            Self::Serifless => (false, false, false, false),
            Self::TopLeftSerifed => (true, false, false, false),
            Self::Serifed => (true, true, true, false),
            Self::SerifedCapped => (true, true, true, true),
        }
    }
}

/// The settings, in the table's order.
pub const CONFIGS: &[(&str, SerifConfig)] = &[
    ("serifless", SerifConfig::Serifless),
    ("topLeftSerifed", SerifConfig::TopLeftSerifed),
    ("serifed", SerifConfig::Serifed),
    ("serifedCapped", SerifConfig::SerifedCapped),
];

/// The upper-case `F` under construction.
pub struct UpperF<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl UpperF<'_> {
    const K_SB: f64 = 1.5;

    const fn x_f_bar_left(&self) -> f64 {
        mix(0.0, self.metrics.sb, Self::K_SB)
    }

    const fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    fn weight(&self) -> lemmata_ptl::metrics::WeightControl {
        lemmata_ptl::metrics::WeightControl::new(
            self.metrics.stroke,
            self.metrics.width,
            self.metrics.sb,
            self.metrics.hv_contrast,
        )
    }

    /// `FShape`: the stem, the top bar, the crossbar and the serifs.
    #[must_use]
    pub fn shape(&self, top: f64, config: SerifConfig, stroke: f64) -> Geometry {
        self.shape_with(top, config, stroke, true)
    }

    /// `FShape`, with the crossbar optional so the French franc sign can leave
    /// it out rather than tag and eject it. `ejectTagged` upstream compares the
    /// tag; the port's geometry carries none, so building it out is exact where
    /// ejecting would also drop the tagged foot serif.
    fn shape_with(&self, top: f64, config: SerifConfig, stroke: f64, crossbar: bool) -> Geometry {
        let metrics = self.metrics;
        let (lt, lb, rt, rm) = config.flags();
        let x_bar_left = self.x_f_bar_left();
        let mid_right = mix(
            metrics.width,
            metrics.width - metrics.sb,
            shapes::ef_mid_bar_shrink_k(self.df, rt, None),
        );
        let bar_pos = shapes::ef_mid_bar_pos_y(top, 0.0, None, self.design);
        let mut parts = vec![
            shapes::vbar_l(self.kit, metrics, x_bar_left, 0.0, top, stroke)
                .geometry(kurbo::Affine::IDENTITY),
            shapes::hbar_t(
                self.kit,
                metrics,
                x_bar_left - metrics.o,
                metrics.width - metrics.sb,
                top,
                Some(stroke),
            )
            .geometry(kurbo::Affine::IDENTITY),
        ];
        if crossbar {
            parts.push(Geometry::Tagged(Box::new(
                shapes::hbar_m(
                    self.kit,
                    metrics,
                    x_bar_left - metrics.o,
                    mid_right,
                    bar_pos,
                    Some(stroke),
                )
                .geometry(kurbo::Affine::IDENTITY),
            )));
        }
        if lt {
            parts.push(
                shapes::hserif(
                    self.kit,
                    metrics,
                    HSerifSide::LeftTop,
                    x_bar_left,
                    top,
                    metrics.side_jut,
                    stroke,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            );
        }
        if lb {
            let x = x_bar_left + self.hsw_to_v(0.5 * stroke);
            let shrink = self.hsw_to_v(0.5 * (metrics.stroke - stroke));
            let base = shapes::hserif(
                self.kit,
                metrics,
                HSerifSide::LeftBottom,
                x,
                0.0,
                metrics.jut - shrink,
                stroke,
                None,
            );
            let side = shapes::hserif(
                self.kit,
                metrics,
                HSerifSide::RightBottom,
                x,
                0.0,
                metrics.mid_jut_side - shrink,
                stroke,
                None,
            );
            parts.push(Geometry::Tagged(Box::new(Geometry::Combine(vec![
                base.geometry(kurbo::Affine::IDENTITY),
                side.geometry(kurbo::Affine::IDENTITY),
            ]))));
        }
        let (jut_top, _jut_bot, jut_mid) = shapes::ef_v_jut_length(
            metrics,
            self.design,
            top,
            0.0,
            None,
            Some(stroke),
            None,
            None,
        );
        let sw_v_jut = metrics.v_jut_stroke * (stroke / metrics.stroke);
        if rt {
            parts.push(
                shapes::vserif(
                    self.kit,
                    metrics,
                    VSerifSide::DownRight,
                    metrics.width - metrics.sb,
                    top,
                    jut_top,
                    Some(sw_v_jut),
                )
                .geometry(kurbo::Affine::IDENTITY),
            );
        }
        if rm {
            parts.push(
                shapes::vserif_mr(
                    self.kit,
                    metrics,
                    mid_right,
                    bar_pos,
                    jut_mid,
                    Some(sw_v_jut),
                )
                .geometry(kurbo::Affine::IDENTITY),
            );
        }

        Geometry::Combine(parts)
    }

    /// `F.<config>` at cap height.
    #[must_use]
    pub fn capital(&self, config: SerifConfig) -> Geometry {
        let stroke = self
            .weight()
            .advice_stroke2(2.0, 3.0, self.metrics.cap, 1.0);

        self.shape(self.metrics.cap, config, stroke)
    }

    /// `smcpF.<config>` at x-height.
    #[must_use]
    pub fn small(&self, config: SerifConfig) -> Geometry {
        let stroke = self.weight().advice_stroke2(2.0, 3.0, self.metrics.xh, 1.0);

        self.shape(self.metrics.xh, config, stroke)
    }

    /// `FBar.<config>`: `F` with a bar over the stem.
    #[must_use]
    pub fn bar(&self, config: SerifConfig) -> Geometry {
        let metrics = self.metrics;
        let (_lt, _lb, rt, _rm) = config.flags();
        let bar_pos = shapes::ef_mid_bar_pos_y(metrics.cap, 0.0, None, self.design);
        let space = (
            0.0,
            (metrics.width - metrics.sb)
                - if rt {
                    self.hsw_to_v(metrics.v_jut_stroke)
                } else {
                    0.0
                },
        );
        let overlay = shapes::letter_bar_overlay_l_in(
            self.kit,
            metrics,
            self.x_f_bar_left(),
            bar_pos + metrics.half_stroke,
            metrics.cap - metrics.stroke,
            0.5,
            space,
            metrics.stroke,
            metrics.overlay_stroke,
        );

        Geometry::Combine(vec![
            self.capital(config),
            overlay.geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// `currency/frenchFrancSign.<config>`: `F` with the crossbar replaced by
    /// two bars.
    #[must_use]
    pub fn french_franc(&self, config: SerifConfig) -> Geometry {
        let metrics = self.metrics;
        let stroke = self
            .weight()
            .advice_stroke2(2.0, 3.0, self.metrics.cap, 1.0);
        let base = self.shape_with(self.metrics.cap, config, stroke, false);
        let mut parts = vec![base];
        for (py, top) in [(0.375, metrics.cap), (0.625, metrics.cap - metrics.stroke)] {
            parts.push(
                shapes::letter_bar_overlay_l_in(
                    self.kit,
                    metrics,
                    self.x_f_bar_left(),
                    0.0,
                    top,
                    py,
                    (0.0, metrics.width),
                    metrics.stroke,
                    metrics.overlay_stroke,
                )
                .geometry(kurbo::Affine::IDENTITY),
            );
        }

        Geometry::Combine(parts)
    }

    /// `mathbb/F`.
    #[must_use]
    pub fn mathbb(&self) -> Geometry {
        let metrics = self.metrics;
        let bbs = shapes::bb_stroke(self.df);
        let bbd = shapes::bb_distance(metrics, bbs);
        let x = self.x_f_bar_left();
        let rect = shapes::rect(
            self.kit,
            metrics.cap - bbs,
            bbs,
            x + self.hsw_to_v(bbs),
            x + bbd,
        );
        let difference = lemmata_ptl::glyph::difference(vec![
            self.shape(metrics.cap, SerifConfig::Serifless, bbs),
            Geometry::Spiro {
                gizmo: kurbo::Affine::IDENTITY,
                closed: rect.closed,
                knots: rect.knots,
            },
        ]);
        let parts = vec![
            shapes::vbar_l(self.kit, metrics, x + bbd, 0.0, metrics.cap, bbs)
                .geometry(kurbo::Affine::IDENTITY),
            shapes::hbar_b(self.kit, metrics, x, x + bbd, 0.0, Some(bbs))
                .geometry(kurbo::Affine::IDENTITY),
            difference,
        ];

        Geometry::Combine(parts)
    }
}

impl UpperF<'_> {
    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = vec![("mathbb/F".to_string(), self.mathbb())];
        for (suffix, config) in CONFIGS {
            out.push((format!("F.{suffix}"), self.capital(*config)));
            out.push((format!("FBar.{suffix}"), self.bar(*config)));
            out.push((format!("smcpF.{suffix}"), self.small(*config)));
            if *config != SerifConfig::SerifedCapped {
                out.push((
                    format!("currency/frenchFrancSign.{suffix}"),
                    self.french_franc(*config),
                ));
            }
        }

        out
    }
}
