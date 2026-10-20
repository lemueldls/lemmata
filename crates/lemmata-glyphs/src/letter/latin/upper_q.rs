//! The upper-case `Q` from `letter/latin/upper-q.ptl`.
//!
//! `Q` is the `O` ring with a tail. Four of the table's tails are ported: the
//! straight one, the diagonal crossing, and the two detached forms. `smcpQ` is
//! the small capital.
//!
//! The curly, swashy and horizontal tails, `QSideways` and `mathbb/Q` need
//! masks, hooks or transforms and are not ported. The anchors are not either.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops, clippy::similar_names)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::control::Item;
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::{Metrics, WeightControl};
use lemmata_ptl::shapes::{self, Design};
use lemmata_ptl::spiro_kit::Kit;
use lemmata_ptl::util::mix;

/// The tails ported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tail {
    Straight,
    Crossing,
    Detached,
    DetachedBend,
}

/// The tails, in the table's order.
pub const TAILS: &[(&str, Tail)] = &[
    ("straight", Tail::Straight),
    ("crossing", Tail::Crossing),
    ("detachedTailed", Tail::Detached),
    ("detachedBendTailed", Tail::DetachedBend),
];

/// The upper-case `Q` under construction.
pub struct UpperQ<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl UpperQ<'_> {
    const fn left_sb(&self) -> f64 {
        self.df.sb
    }

    const fn right_sb(&self) -> f64 {
        self.df.width - self.df.sb
    }

    const fn middle(&self) -> f64 {
        mix(self.left_sb(), self.right_sb(), 0.5)
    }

    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    fn weight(&self) -> WeightControl {
        WeightControl::new(
            self.metrics.stroke,
            self.metrics.width,
            self.metrics.sb,
            self.metrics.hv_contrast,
        )
    }

    fn tail_depth(&self) -> f64 {
        -self.metrics.hook
    }

    /// `QStdBody`.
    #[must_use]
    pub fn body(&self, top: f64, sw: f64) -> Geometry {
        shapes::oshape(
            self.kit,
            self.metrics,
            self.design,
            top,
            0.0,
            self.left_sb(),
            self.right_sb(),
            Some(sw),
            Some(self.metrics.arch_depth_a),
            Some(self.metrics.arch_depth_b),
        )
        .geometry(kurbo::Affine::IDENTITY)
    }

    fn crossing(&self, _top: f64, sw: f64) -> Geometry {
        let metrics = self.metrics;
        let k = 1.5;
        let half = self.right_sb() - self.middle();
        let cor = shapes::diag_cor(k * half, half - metrics.ox, None, None);
        let shift = metrics.xh * -0.1;
        self.kit
            .dispiro(vec![
                Item::Af(self.kit.widths_center(Some(cor * sw))),
                self.kit.knot(
                    KnotType::Left,
                    self.right_sb(),
                    shift,
                    self.kit.heading(shapes::upward(metrics.hv_contrast)),
                ),
                self.kit.knot(
                    KnotType::Right,
                    self.middle() + metrics.ox,
                    k * half + shift,
                    self.kit.heading(shapes::upward(metrics.hv_contrast)),
                ),
            ])
            .geometry(kurbo::Affine::IDENTITY)
    }

    fn straight_tail(&self) -> Geometry {
        let metrics = self.metrics;
        let shift = metrics.stroke_width_blend(0.0, 0.25, None);
        let depth = self.tail_depth();
        self.kit
            .dispiro(vec![
                Item::Af(self.kit.widths_rhs(None)),
                self.kit.knot(
                    KnotType::Left,
                    self.middle() - self.hsw_to_v(metrics.stroke) * shift - depth * 0.25,
                    depth,
                    self.kit.heading(shapes::upward(metrics.hv_contrast)),
                ),
                self.kit.knot(
                    KnotType::Right,
                    self.middle() - self.hsw_to_v(metrics.stroke) * shift,
                    0.0,
                    Vec::new(),
                ),
            ])
            .geometry(kurbo::Affine::IDENTITY)
    }

    fn detached_points(&self) -> (f64, f64, f64, f64) {
        let metrics = self.metrics;
        let gap = ((-0.25) * metrics.descender).max(self.weight().advice_stroke(12.0, 1.0, 1.0));
        let y_start = 0.0 - gap - metrics.stroke * 0.875;
        let y_end = mix(0.0, metrics.descender, 0.75) - metrics.half_stroke;

        (
            y_start,
            y_end,
            mix(self.right_sb(), metrics.width, 0.75),
            0.0,
        )
    }

    fn detached(&self) -> Geometry {
        let metrics = self.metrics;
        let (y_start, y_end, x_end, _) = self.detached_points();
        self.kit
            .dispiro(vec![
                Item::Af(self.kit.widths_lhs(Some(metrics.stroke))),
                self.kit.knot(
                    KnotType::Corner,
                    self.middle(),
                    y_start,
                    self.kit.heading(shapes::rightward()),
                ),
                self.kit.knot(
                    KnotType::Corner,
                    x_end,
                    y_end,
                    self.kit.heading(shapes::rightward()),
                ),
            ])
            .geometry(kurbo::Affine::IDENTITY)
    }

    fn detached_bend(&self) -> Geometry {
        let metrics = self.metrics;
        let (y_start, y_end, _, _) = self.detached_points();
        let x_start = mix(self.middle(), self.left_sb(), 0.5);
        let y_bend = mix(y_start, y_end, 0.5);

        lemmata_ptl::glyph::union(vec![
            self.detached(),
            self.kit
                .dispiro(vec![
                    Item::Af(self.kit.widths_lhs(Some(metrics.stroke))),
                    self.kit.knot(
                        KnotType::Corner,
                        x_start,
                        y_bend,
                        self.kit.heading(shapes::rightward()),
                    ),
                    self.kit.knot(
                        KnotType::Corner,
                        self.middle(),
                        y_start,
                        self.kit.heading(shapes::rightward()),
                    ),
                ])
                .geometry(kurbo::Affine::IDENTITY),
        ])
    }

    fn tail(&self, tail: Tail, top: f64, sw: f64) -> Geometry {
        match tail {
            Tail::Straight => self.straight_tail(),
            Tail::Crossing => self.crossing(top, sw),
            Tail::Detached => self.detached(),
            Tail::DetachedBend => self.detached_bend(),
        }
    }

    /// `Q.<suffix>`.
    #[must_use]
    pub fn capital(&self, tail: Tail) -> Geometry {
        let sw = match tail {
            Tail::Crossing => self.weight().advice_stroke(4.0, 1.0, 1.0),
            _ => self.metrics.stroke,
        };

        Geometry::Combine(vec![
            self.body(self.metrics.cap, self.metrics.stroke),
            self.tail(tail, self.metrics.cap, sw),
        ])
    }

    /// `smcpQ.<suffix>`.
    #[must_use]
    pub fn small(&self, tail: Tail) -> Geometry {
        let sw = match tail {
            Tail::Crossing => self.weight().advice_stroke(4.0, 1.0, 1.0),
            _ => self.metrics.stroke,
        };

        Geometry::Combine(vec![
            self.body(self.metrics.xh, self.metrics.stroke),
            self.tail(tail, self.metrics.xh, sw),
        ])
    }
}

impl UpperQ<'_> {
    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for (suffix, tail) in TAILS {
            out.push((format!("Q.{suffix}"), self.capital(*tail)));
            out.push((format!("smcpQ.{suffix}"), self.small(*tail)));
        }

        out
    }
}
