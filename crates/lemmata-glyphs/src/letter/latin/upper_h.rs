//! The upper-case `H` from `letter/latin/upper-h.ptl`.
//!
//! Two stems and a crossbar, with seven serif grades and an optional tail.
//! `smcpH` is the small capital.
//!
//! The turned and half forms, the hooks, the descenders and the strokes need
//! shapes outside this file and are not ported. The anchors are not either.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::serif::SerifFrame;
use lemmata_ptl::shapes::{self, Design};
use lemmata_ptl::spiro_kit::Kit;
use lemmata_ptl::util::mix;

/// The seven serif grades.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slab {
    None,
    TopLeft,
    TopLeftBottomRight,
    Tailed,
    All,
    AllBgr,
    TailedBgr,
}

/// The suffixes the table names, as `(suffix, tailed, slab)`.
#[must_use]
pub fn variants() -> Vec<(&'static str, bool, Slab)> {
    vec![
        ("serifless", false, Slab::None),
        ("topLeftSerifed", false, Slab::TopLeft),
        ("topLeftBottomRightSerifed", false, Slab::TopLeftBottomRight),
        ("serifed", false, Slab::All),
        ("serifedBGR", false, Slab::AllBgr),
        ("tailedSerifless", true, Slab::None),
        ("tailedTopLeftSerifed", true, Slab::TopLeft),
        ("tailedSerifed", true, Slab::Tailed),
        ("tailedSerifedBGR", true, Slab::TailedBgr),
        ("serifedExceptBottomRight", false, Slab::Tailed),
    ]
}

/// The upper-case `H` under construction.
pub struct UpperH<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl UpperH<'_> {
    const fn left_sb(&self) -> f64 {
        self.df.sb
    }

    const fn right_sb(&self) -> f64 {
        self.df.width - self.df.sb
    }

    fn bar_pos(&self, top: f64) -> f64 {
        mix(0.0, top, self.design.h_bar_pos) - 0.09 * self.metrics.stroke * top / self.metrics.cap
    }

    /// `HShape`.
    #[must_use]
    pub fn shape(&self, top: f64, slab: Slab, tailed: bool) -> Geometry {
        let metrics = self.metrics;
        let stroke = self.df.mvs;
        let mut parts = vec![
            Geometry::Tagged(Box::new(
                shapes::vbar_l(self.kit, metrics, self.left_sb(), 0.0, top, stroke)
                    .geometry(kurbo::Affine::IDENTITY),
            )),
            Geometry::Tagged(Box::new(if tailed {
                shapes::rightward_tailed_bar(
                    self.kit,
                    metrics,
                    self.df,
                    self.right_sb(),
                    0.0,
                    top,
                    stroke,
                )
                .geometry(kurbo::Affine::IDENTITY)
            } else {
                shapes::vbar_r(self.kit, metrics, self.right_sb(), 0.0, top, stroke)
                    .geometry(kurbo::Affine::IDENTITY)
            })),
            Geometry::Tagged(Box::new(
                shapes::hbar_m(
                    self.kit,
                    metrics,
                    self.left_sb() - metrics.o,
                    self.right_sb() + metrics.o,
                    self.bar_pos(top),
                    Some(stroke),
                )
                .geometry(kurbo::Affine::IDENTITY),
            )),
        ];
        let frame = SerifFrame::from_df(metrics, self.df, top, 0.0, Some(stroke), None, false);
        parts.extend(match slab {
            Slab::None => Vec::new(),
            Slab::TopLeft => vec![frame.lt_outer(self.kit, metrics)],
            Slab::TopLeftBottomRight => vec![
                frame.lt_outer(self.kit, metrics),
                frame.rb_outer(self.kit, metrics),
            ],
            Slab::Tailed => vec![
                frame.lt_full(self.kit, metrics),
                frame.rt_full(self.kit, metrics),
                frame.lb_full(self.kit, metrics),
            ],
            Slab::All => vec![
                frame.lt_full(self.kit, metrics),
                frame.rt_full(self.kit, metrics),
                frame.lb_full(self.kit, metrics),
                frame.rb_full(self.kit, metrics),
            ],
            Slab::AllBgr => vec![
                frame.lt_outer(self.kit, metrics),
                frame.rt_inner(self.kit, metrics),
                frame.lb_full(self.kit, metrics),
                frame.rb_full(self.kit, metrics),
            ],
            Slab::TailedBgr => vec![
                frame.lt_outer(self.kit, metrics),
                frame.rt_inner(self.kit, metrics),
                frame.lb_full(self.kit, metrics),
            ],
        });

        Geometry::Combine(parts)
    }

    /// `H.<suffix>`.
    #[must_use]
    pub fn capital(&self, slab: Slab, tailed: bool) -> Geometry {
        self.shape(self.metrics.cap, slab, tailed)
    }

    /// `smcpH.<suffix>`.
    #[must_use]
    pub fn small(&self, slab: Slab, tailed: bool) -> Geometry {
        self.shape(self.metrics.xh, slab, tailed)
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for (suffix, tailed, slab) in variants() {
            out.push((format!("H.{suffix}"), self.capital(slab, tailed)));
            out.push((format!("smcpH.{suffix}"), self.small(slab, tailed)));
        }

        out
    }
}
