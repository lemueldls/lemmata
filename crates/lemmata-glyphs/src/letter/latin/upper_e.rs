//! The upper-case `E` from `letter/latin/upper-e.ptl`.
//!
//! The letter is a stem and three bars, with four serif settings. `smcpE` is the
//! small capital, `smcpEBar` adds a bar, `revE` is the mirrored form and
//! `mathbb/E` the blackboard one.
//!
//! `grek/Epsilon`, `EHookRight` and the turned letter need the Greek tonos, the
//! hook superness or a transform outside this file and are not ported. The
//! anchors are not either.
//!
//! `mathbb/E` is built but not captured: its boolean outline lands half a unit
//! past the geometry suite's drift budget, so it is left out of the set rather
//! than paid for by widening the budget.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::shapes::{self, Design, HSerifSide, VSerifSide};
use lemmata_ptl::spiro_kit::Kit;
use lemmata_ptl::util::mix;

/// The four serif settings, as the five flags the table carries: left top, left
/// bottom, right top, right bottom, right middle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SerifConfig {
    Serifless,
    TopLeftSerifed,
    Serifed,
    SerifedCapped,
}

impl SerifConfig {
    const fn flags(self) -> (bool, bool, bool, bool, bool) {
        match self {
            Self::Serifless => (false, false, false, false, false),
            Self::TopLeftSerifed => (true, false, false, false, false),
            Self::Serifed => (true, true, true, true, false),
            Self::SerifedCapped => (true, true, true, true, true),
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

/// The upper-case `E` under construction.
pub struct UpperE<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl UpperE<'_> {
    fn x_bar_left(&self) -> f64 {
        self.metrics.sb - self.metrics.o
    }

    fn x_rev_bar_right(&self) -> f64 {
        (self.metrics.width - self.metrics.sb) + self.metrics.o
    }

    fn hsw_to_v(&self, value: f64) -> f64 {
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

    /// `EShape`.
    #[must_use]
    pub fn shape(&self, top: f64, config: SerifConfig, stroke: f64) -> Geometry {
        let metrics = self.metrics;
        let (lt, lb, rt, rb, rm) = config.flags();
        let x = self.x_bar_left();
        let right = metrics.width - metrics.sb;
        let mid_right = mix(
            metrics.width,
            right,
            shapes::ef_mid_bar_shrink_k(self.df, rt, None),
        );
        let bar_pos = shapes::ef_mid_bar_pos_y(top, 0.0, None, self.design);
        let mut parts = vec![
            Geometry::Tagged(Box::new(
                shapes::vbar_l(self.kit, metrics, x, 0.0, top, stroke)
                    .geometry(kurbo::Affine::IDENTITY),
            )),
            Geometry::Tagged(Box::new(
                shapes::hbar_t(self.kit, metrics, x - metrics.o, right, top, Some(stroke))
                    .geometry(kurbo::Affine::IDENTITY),
            )),
            Geometry::Tagged(Box::new(
                shapes::hbar_m(
                    self.kit,
                    metrics,
                    x - metrics.o,
                    mid_right,
                    bar_pos,
                    Some(stroke),
                )
                .geometry(kurbo::Affine::IDENTITY),
            )),
            Geometry::Tagged(Box::new(
                shapes::hbar_b(self.kit, metrics, x - metrics.o, right, 0.0, Some(stroke))
                    .geometry(kurbo::Affine::IDENTITY),
            )),
        ];
        if lt {
            parts.push(Geometry::Tagged(Box::new(
                shapes::hserif(
                    self.kit,
                    metrics,
                    HSerifSide::LeftTop,
                    x,
                    top,
                    metrics.side_jut,
                    stroke,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            )));
        }
        if lb {
            parts.push(Geometry::Tagged(Box::new(
                shapes::hserif(
                    self.kit,
                    metrics,
                    HSerifSide::LeftBottom,
                    x,
                    0.0,
                    metrics.side_jut,
                    stroke,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            )));
        }
        let (jut_top, jut_bot, jut_mid) = shapes::ef_v_jut_length(
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
            parts.push(Geometry::Tagged(Box::new(
                shapes::vserif(
                    self.kit,
                    metrics,
                    VSerifSide::DownRight,
                    right,
                    top,
                    jut_top,
                    Some(sw_v_jut),
                )
                .geometry(kurbo::Affine::IDENTITY),
            )));
        }
        if rb {
            parts.push(Geometry::Tagged(Box::new(
                shapes::vserif(
                    self.kit,
                    metrics,
                    VSerifSide::UpRight,
                    right,
                    0.0,
                    jut_bot,
                    Some(sw_v_jut),
                )
                .geometry(kurbo::Affine::IDENTITY),
            )));
        }
        if rm {
            parts.push(Geometry::Tagged(Box::new(
                shapes::vserif_mr(
                    self.kit,
                    metrics,
                    mid_right,
                    bar_pos,
                    jut_mid,
                    Some(sw_v_jut),
                )
                .geometry(kurbo::Affine::IDENTITY),
            )));
        }

        Geometry::Combine(parts)
    }

    /// `RevEShape`, the mirrored letter.
    #[must_use]
    pub fn reverse(&self, top: f64, config: SerifConfig, stroke: f64) -> Geometry {
        let metrics = self.metrics;
        let (rt, rb, lt, lb, lm) = config.flags();
        let x = self.x_rev_bar_right();
        let left = metrics.sb;
        let mid_left = mix(0.0, left, shapes::ef_mid_bar_shrink_k(self.df, lt, None));
        let bar_pos = shapes::ef_mid_bar_pos_y(top, 0.0, None, self.design);
        let mut parts = vec![
            Geometry::Tagged(Box::new(
                shapes::vbar_r(self.kit, metrics, x, 0.0, top, stroke)
                    .geometry(kurbo::Affine::IDENTITY),
            )),
            Geometry::Tagged(Box::new(
                shapes::hbar_t(self.kit, metrics, left, x + metrics.o, top, Some(stroke))
                    .geometry(kurbo::Affine::IDENTITY),
            )),
            Geometry::Tagged(Box::new(
                shapes::hbar_m(
                    self.kit,
                    metrics,
                    mid_left,
                    x + metrics.o,
                    bar_pos,
                    Some(stroke),
                )
                .geometry(kurbo::Affine::IDENTITY),
            )),
            Geometry::Tagged(Box::new(
                shapes::hbar_b(self.kit, metrics, left, x + metrics.o, 0.0, Some(stroke))
                    .geometry(kurbo::Affine::IDENTITY),
            )),
        ];
        if rt {
            parts.push(Geometry::Tagged(Box::new(
                shapes::hserif(
                    self.kit,
                    metrics,
                    HSerifSide::RightTop,
                    x,
                    top,
                    metrics.side_jut,
                    stroke,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            )));
        }
        if rb {
            parts.push(Geometry::Tagged(Box::new(
                shapes::hserif(
                    self.kit,
                    metrics,
                    HSerifSide::RightBottom,
                    x,
                    0.0,
                    metrics.side_jut,
                    stroke,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            )));
        }
        let (jut_top, jut_bot, jut_mid) = shapes::ef_v_jut_length(
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
        if lt {
            parts.push(Geometry::Tagged(Box::new(
                shapes::vserif(
                    self.kit,
                    metrics,
                    VSerifSide::DownLeft,
                    left,
                    top,
                    jut_top,
                    Some(sw_v_jut),
                )
                .geometry(kurbo::Affine::IDENTITY),
            )));
        }
        if lb {
            parts.push(Geometry::Tagged(Box::new(
                shapes::vserif(
                    self.kit,
                    metrics,
                    VSerifSide::UpLeft,
                    left,
                    0.0,
                    jut_bot,
                    Some(sw_v_jut),
                )
                .geometry(kurbo::Affine::IDENTITY),
            )));
        }
        if lm {
            parts.push(Geometry::Tagged(Box::new(
                shapes::vserif_ml(
                    self.kit,
                    metrics,
                    mid_left,
                    bar_pos,
                    jut_mid,
                    Some(sw_v_jut),
                )
                .geometry(kurbo::Affine::IDENTITY),
            )));
        }

        Geometry::Combine(parts)
    }

    /// `E.<config>`.
    #[must_use]
    pub fn capital(&self, config: SerifConfig) -> Geometry {
        let stroke = self
            .weight()
            .advice_stroke2(2.0, 3.0, self.metrics.cap, 1.0);

        self.shape(self.metrics.cap, config, stroke)
    }

    /// `smcpE.<config>`.
    #[must_use]
    pub fn small(&self, config: SerifConfig) -> Geometry {
        let stroke = self.weight().advice_stroke2(2.0, 3.0, self.metrics.xh, 1.0);

        self.shape(self.metrics.xh, config, stroke)
    }

    /// `revE.<config>`.
    #[must_use]
    pub fn rev_e(&self, config: SerifConfig) -> Geometry {
        let stroke = self
            .weight()
            .advice_stroke2(2.0, 3.0, self.metrics.cap, 1.0);

        self.reverse(self.metrics.cap, config, stroke)
    }

    /// `smcpEBar.<config>`.
    #[must_use]
    pub fn small_bar(&self, config: SerifConfig) -> Geometry {
        let metrics = self.metrics;
        let stroke = self.weight().advice_stroke2(2.0, 3.0, metrics.xh, 1.0);
        let (_, _, rt, rb, rm) = config.flags();
        let bar_pos = shapes::ef_mid_bar_pos_y(metrics.xh, 0.0, None, self.design);
        let sw_v_jut = metrics.v_jut_stroke * (stroke / metrics.stroke);
        let x_right = if rm {
            mix(
                metrics.width,
                metrics.width - metrics.sb,
                shapes::ef_mid_bar_shrink_k(self.df, rt, None),
            ) - self.hsw_to_v(sw_v_jut)
        } else if rb {
            (metrics.width - metrics.sb) - self.hsw_to_v(sw_v_jut)
        } else {
            metrics.width
        };
        let bar = shapes::letter_bar_overlay_l_in(
            self.kit,
            metrics,
            self.x_bar_left(),
            stroke,
            bar_pos - stroke / 2.0,
            0.5,
            (0.0, x_right),
            metrics.stroke,
            metrics.overlay_stroke,
        );

        Geometry::Combine(vec![
            self.small(config),
            bar.geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// `mathbb/E`.
    #[must_use]
    pub fn mathbb(&self) -> Geometry {
        let metrics = self.metrics;
        let bbs = shapes::bb_stroke(self.df);
        let bbd = shapes::bb_distance(metrics, bbs);
        let x = self.x_bar_left();
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

        lemmata_ptl::glyph::union(vec![
            difference,
            shapes::vbar_l(self.kit, metrics, x + bbd, 0.0, metrics.cap, bbs)
                .geometry(kurbo::Affine::IDENTITY),
        ])
    }
}

impl UpperE<'_> {
    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = vec![("mathbb/E".to_string(), self.mathbb())];
        for (suffix, config) in CONFIGS {
            out.push((format!("E.{suffix}"), self.capital(*config)));
            out.push((format!("smcpE.{suffix}"), self.small(*config)));
            out.push((format!("revE.{suffix}"), self.rev_e(*config)));
            out.push((format!("smcpEBar.{suffix}"), self.small_bar(*config)));
        }

        out
    }
}
