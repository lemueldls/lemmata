//! The upper-case `N` from `letter/latin/upper-n.ptl`.
//!
//! Two straight stems joined by a diagonal, in a symmetric or asymmetric body,
//! with the slab settings the plan's `slab` level selects. `smcpN` is the small
//! capital.
//!
//! `revN`, `Eng.NSM`, `NHookLeft`, the descenders and the Cyrillic bodies need
//! hooks or glyphs outside this file and are not ported. The anchors are not
//! either.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops, clippy::similar_names)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    control::Item,
    letter_model::DivFrame,
    metrics::{Metrics, WeightControl},
    serif::SerifFrame,
    shapes::{self, Design},
    spiro_kit::{Direction, Dispiro, Family, Kit, Position},
    util::{join_camel, mix},
};

/// The two bodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Symmetric,
    Asymmetric,
}

/// The slab settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slab {
    /// `SLAB-AUTO`: what the plan's slab level says.
    Auto,
    None,
    Motion,
    All,
}

impl Slab {
    /// The serifs this setting draws, given the plan's slab level.
    const fn resolved(self, plan_slab: f64) -> Slab {
        match self {
            Self::Auto if plan_slab == 0.0 => Self::None,
            Self::Auto => Self::All,
            other => other,
        }
    }
}

/// The bodies, in the table's order.
pub const BODIES: &[(&str, Body)] = &[
    ("standard", Body::Symmetric),
    ("asymmetric", Body::Asymmetric),
];

/// The slabs, in the table's order.
pub const SLABS: &[(&str, Slab)] = &[
    ("", Slab::Auto),
    ("serifless", Slab::None),
    ("motionSerifed", Slab::Motion),
    ("serifed", Slab::All),
];

/// The upper-case `N` under construction.
pub struct UpperN<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl UpperN<'_> {
    const P_INK_TRAP: f64 = 0.4;

    const fn left_sb(&self) -> f64 {
        self.df.sb
    }

    const fn right_sb(&self) -> f64 {
        self.df.width - self.df.sb
    }

    fn weight(&self) -> WeightControl {
        WeightControl::new(
            self.metrics.stroke,
            self.metrics.width,
            self.metrics.sb,
            self.metrics.hv_contrast,
        )
    }

    /// The two stems: straight strands with a trap when symmetric, plain bars
    /// otherwise.
    fn stems(&self, top: f64, body: Body, fine: f64, stroke: f64) -> (Dispiro, Dispiro) {
        let metrics = self.metrics;
        if body == Body::Symmetric {
            let up = self.kit.heading(shapes::upward(metrics.hv_contrast));
            let down = self.kit.heading(shapes::downward(metrics.hv_contrast));
            let left = self.kit.dispiro(vec![
                Item::Af(self.kit.widths_rhs(Some(stroke))),
                self.kit
                    .knot(KnotType::Left, self.left_sb(), 0.0, up.clone()),
                self.kit.knot(
                    KnotType::Right,
                    self.left_sb(),
                    mix(0.0, top, Self::P_INK_TRAP),
                    up,
                ),
                self.kit.pair(
                    Family::Straight,
                    Direction::Up,
                    Position::End,
                    self.left_sb(),
                    top,
                    self.kit
                        .widths_rhs_heading(Some(fine), shapes::upward(metrics.hv_contrast)),
                ),
            ]);
            let right = self.kit.dispiro(vec![
                Item::Af(self.kit.widths_rhs(Some(stroke))),
                self.kit
                    .knot(KnotType::Left, self.right_sb(), top, down.clone()),
                self.kit.knot(
                    KnotType::Right,
                    self.right_sb(),
                    mix(top, 0.0, Self::P_INK_TRAP),
                    down,
                ),
                self.kit.pair(
                    Family::Straight,
                    Direction::Down,
                    Position::End,
                    self.right_sb(),
                    0.0,
                    self.kit
                        .widths_rhs_heading(Some(fine), shapes::downward(metrics.hv_contrast)),
                ),
            ]);

            return (left, right);
        }

        (
            shapes::vbar_l(self.kit, metrics, self.left_sb(), 0.0, top, fine),
            shapes::vbar_r(self.kit, metrics, self.right_sb(), 0.0, top, fine),
        )
    }

    /// `NShape`.
    #[must_use]
    pub fn shape(&self, top: f64, body: Body, slab: Slab) -> Geometry {
        let metrics = self.metrics;
        let stroke = self.weight().advice_stroke(2.0, 1.0, 1.0);
        let sw_diag =
            self.weight()
                .advice_stroke(if body == Body::Symmetric { 4.0 } else { 3.0 }, 1.0, 1.0);
        let fine = if body == Body::Symmetric {
            sw_diag
        } else {
            stroke
        };
        let y_end = if body == Body::Asymmetric {
            mix(0.0, top, 0.375)
        } else {
            0.0
        };
        let y_start = top;
        let x_end = self.right_sb() - sw_diag;
        let x_start = self.left_sb() + sw_diag;
        let left = self.left_sb();
        let right = self.right_sb();
        let rect = shapes::rect(self.kit, top, 0.0, left, right);
        let line = shapes::ext_line_center(
            self.kit,
            metrics,
            x_start,
            y_start,
            x_end,
            y_end,
            Some(sw_diag),
            Some(2.0),
        );
        let diagonal = lemmata_ptl::glyph::intersection(vec![
            Geometry::Spiro {
                gizmo: kurbo::Affine::IDENTITY,
                closed: rect.closed,
                knots: rect.knots,
            },
            line.geometry(kurbo::Affine::IDENTITY),
        ]);

        let (first, second) = self.stems(top, body, fine, stroke);
        let mut parts = vec![
            first.geometry(kurbo::Affine::IDENTITY),
            second.geometry(kurbo::Affine::IDENTITY),
        ];
        parts.push(diagonal);
        let frame = SerifFrame::from_df(metrics, self.df, top, 0.0, Some(stroke), None, false);
        match slab.resolved(metrics.slab) {
            Slab::All => {
                parts.push(frame.lt_outer(self.kit, metrics));
                parts.push(frame.lb_full(self.kit, metrics));
                parts.push(frame.rt_full(self.kit, metrics));
            }
            Slab::Motion => parts.push(frame.lt_outer(self.kit, metrics)),
            Slab::None | Slab::Auto => {}
        }

        Geometry::Combine(parts)
    }

    /// `N.<suffix>`.
    #[must_use]
    pub fn capital(&self, body: Body, slab: Slab) -> Geometry {
        self.shape(self.metrics.cap, body, slab)
    }

    /// `smcpN.<suffix>`.
    #[must_use]
    pub fn small(&self, body: Body, slab: Slab) -> Geometry {
        self.shape(self.metrics.xh, body, slab)
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for (body_key, body) in BODIES {
            for (slab_key, slab) in SLABS {
                let suffix = join_camel(body_key, slab_key);
                out.push((format!("N.{suffix}"), self.capital(*body, *slab)));
                out.push((format!("smcpN.{suffix}"), self.small(*body, *slab)));
            }
        }

        out
    }
}
