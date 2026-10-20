//! The upper-case `D` from `letter/latin/upper-d.ptl`.
//!
//! The letter is a stem and a closed arc. Two bodies differ in whether the
//! bowl's right side is rounded or flat-sided, and three serif settings add the
//! stem's serifs. `smcpD` is the small capital, `Eth` and `smcpEth` add a bar,
//! and `mathbb/D` is the blackboard form.
//!
//! `Dhookleft`, the roman numerals and `TauGallicum` need `LeftHook` or
//! transforms and are not ported. The anchors are not either.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::control::Item;
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::shapes::{self, Design, FlatSide, HSerifSide};
use lemmata_ptl::spiro_kit::{Dispiro, Kit, SpiroOutline};
use lemmata_ptl::util::clamp;

/// The two bodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    /// `standard`: the bowl's right side is flat.
    Standard,
    /// `moreRounded`: the bowl's right side is a single arc.
    MoreRounded,
}

/// The three serif settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serif {
    Serifless,
    Unilateral,
    Bilateral,
}

impl Serif {
    const fn flags(self) -> (bool, bool) {
        match self {
            Self::Serifless => (false, false),
            Self::Unilateral => (true, false),
            Self::Bilateral => (true, true),
        }
    }
}

/// The bodies, in the table's order.
pub const BODIES: &[(&str, Body)] = &[
    ("standard", Body::Standard),
    ("moreRounded", Body::MoreRounded),
];

/// The serifs, in the table's order.
pub const SERIFS: &[(&str, Serif)] = &[
    ("serifless", Serif::Serifless),
    ("unilateralSerifed", Serif::Unilateral),
    ("bilateralSerifed", Serif::Bilateral),
];

/// The upper-case `D` under construction.
pub struct UpperD<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl UpperD<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    fn adjust_x_top(&self, x: f64, sw: f64) -> f64 {
        x - self.metrics.correction_o_mid_x * sw
    }

    fn adjust_x_bot(&self, x: f64, sw: f64) -> f64 {
        x + self.metrics.correction_o_mid_x * sw
    }

    /// `DArcShapeT`: the bowl, as an item sequence so the caller can sink it
    /// into a di-spiro or a spiro outline.
    fn arc_items(
        &self,
        top: f64,
        right: f64,
        offset: f64,
        sw: f64,
        round: Body,
        smooth: Option<f64>,
    ) -> Vec<Item> {
        let metrics = self.metrics;
        let df = self.df;
        let left = df.sb;
        let mock_width = right + df.sb;
        let max_x_smooth_width = mock_width - 2.0 * left - self.hsw_to_v(sw);
        let dsmooth = smooth.unwrap_or_else(|| {
            clamp(
                sw * 1.5,
                top / 2.0 - metrics.tiny,
                (top / metrics.cap) * metrics.arch_depth * 1.1,
            )
        });
        let bsmooth = clamp(
            sw * 1.5,
            max_x_smooth_width,
            (mock_width / metrics.width) * metrics.arch_depth * 1.1,
        );
        let barch_depth_rd = clamp(
            sw * 1.5,
            max_x_smooth_width,
            (mock_width / metrics.width) * metrics.arch_depth * 1.25,
        );
        let ada = metrics.arch_depth_a_of(dsmooth, Some(mock_width));
        let adb = metrics.arch_depth_b_of(dsmooth, Some(mock_width));

        let start_ty = if offset == 0.0 {
            lemmata_geom::KnotType::Left
        } else {
            lemmata_geom::KnotType::Corner
        };
        let mut items = vec![
            Item::Af(self.kit.widths_rhs(Some(sw))),
            self.kit.knot(
                start_ty,
                left - metrics.o,
                top - offset,
                self.kit.heading(shapes::rightward()),
            ),
        ];
        if round == Body::MoreRounded {
            items.push(self.kit.knot(
                lemmata_geom::KnotType::Right,
                self.adjust_x_top(right - barch_depth_rd, sw),
                top - offset,
                Vec::new(),
            ));
            items.push(self.kit.archv(None, None));
            items.push(self.kit.knot(
                lemmata_geom::KnotType::G4,
                right - metrics.ox - offset,
                shapes::ysmooth_mid_r(top, 0.0, Some(ada), Some(adb), ada, adb),
                Vec::new(),
            ));
            items.push(self.kit.arcvh(None, None));
            items.push(self.kit.knot(
                lemmata_geom::KnotType::Left,
                self.adjust_x_bot(right - barch_depth_rd, sw),
                offset,
                Vec::new(),
            ));
        } else {
            items.push(self.kit.knot(
                lemmata_geom::KnotType::Right,
                self.adjust_x_top(right - bsmooth, sw),
                top - offset,
                Vec::new(),
            ));
            items.push(self.kit.archv(None, None));
            items.extend(shapes::flatside_rd(
                self.kit,
                &FlatSide {
                    x: right - offset,
                    d: 0.0,
                    u: top,
                    ada,
                    adb,
                    ox: metrics.ox,
                    af: Vec::new(),
                },
            ));
            items.push(self.kit.arcvh(None, None));
            items.push(self.kit.knot(
                lemmata_geom::KnotType::Left,
                self.adjust_x_bot(right - bsmooth, sw),
                offset,
                Vec::new(),
            ));
        }
        let end_ty = if offset == 0.0 {
            lemmata_geom::KnotType::Right
        } else {
            lemmata_geom::KnotType::Corner
        };
        items.push(self.kit.knot(
            end_ty,
            left - metrics.o,
            offset,
            self.kit.heading(shapes::leftward()),
        ));

        items
    }

    fn arc(&self, top: f64, right: f64, offset: f64, sw: f64, round: Body) -> Dispiro {
        self.kit
            .dispiro(self.arc_items(top, right, offset, sw, round, None))
    }

    fn arc_outline(&self, top: f64, right: f64, offset: f64, sw: f64, round: Body) -> SpiroOutline {
        self.kit
            .spiro_outline(self.arc_items(top, right, offset, sw, round, None))
    }

    /// `DShape`: the stem and the bowl.
    #[must_use]
    pub fn shape(
        &self,
        top: f64,
        right: f64,
        round: Body,
        smooth: Option<f64>,
        slab_top: bool,
        slab_bot: bool,
    ) -> Geometry {
        let metrics = self.metrics;
        let sw = self.df.mvs;
        let mut parts = vec![
            shapes::vbar_l(self.kit, metrics, self.df.sb, 0.0, top, sw)
                .geometry(kurbo::Affine::IDENTITY),
            self.kit
                .dispiro(self.arc_items(top, right, 0.0, sw, round, smooth))
                .geometry(kurbo::Affine::IDENTITY),
        ];
        if slab_top {
            parts.push(Geometry::Tagged(Box::new(
                shapes::hserif(
                    self.kit,
                    metrics,
                    HSerifSide::LeftTop,
                    self.df.sb,
                    top,
                    metrics.side_jut,
                    metrics.stroke,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            )));
        }
        if slab_bot {
            parts.push(Geometry::Tagged(Box::new(
                shapes::hserif(
                    self.kit,
                    metrics,
                    HSerifSide::LeftBottom,
                    self.df.sb,
                    0.0,
                    metrics.side_jut,
                    metrics.stroke,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            )));
        }

        Geometry::Combine(parts)
    }

    /// `D.<suffix>`.
    #[must_use]
    pub fn capital(&self, body: Body, serif: Serif) -> Geometry {
        let (top, bot) = serif.flags();

        self.shape(
            self.metrics.cap,
            self.metrics.width - self.metrics.sb,
            body,
            None,
            top,
            bot,
        )
    }

    /// `smcpD.<suffix>`: the rounded body at x-height.
    #[must_use]
    pub fn small(&self, serif: Serif) -> Geometry {
        let (top, bot) = serif.flags();

        self.shape(
            self.metrics.xh,
            self.metrics.width - self.metrics.sb,
            Body::MoreRounded,
            Some(self.metrics.arch_depth),
            top,
            bot,
        )
    }

    /// `Eth` or `smcpEth`: the base glyph with a bar across the stem.
    fn with_bar(&self, base: Geometry, top: f64) -> Geometry {
        let metrics = self.metrics;
        let bar = shapes::letter_bar_overlay_l_in(
            self.kit,
            metrics,
            self.df.sb,
            metrics.stroke,
            top - metrics.stroke,
            0.5,
            (
                0.0,
                (metrics.width - metrics.sb) - self.hsw_to_v(metrics.stroke),
            ),
            metrics.stroke,
            metrics.overlay_stroke,
        );

        Geometry::Combine(vec![base, bar.geometry(kurbo::Affine::IDENTITY)])
    }

    /// `Eth`.
    #[must_use]
    pub fn eth(&self, body: Body, serif: Serif) -> Geometry {
        let base = self.capital(body, serif);

        self.with_bar(base, self.metrics.cap)
    }

    /// `smcpEth`.
    #[must_use]
    pub fn smcp_eth(&self, serif: Serif) -> Geometry {
        let base = self.small(serif);

        self.with_bar(base, self.metrics.xh)
    }

    /// `mathbb/D`.
    #[must_use]
    pub fn mathbb(&self) -> Geometry {
        let metrics = self.metrics;
        let bbs = shapes::bb_stroke(self.df);
        let bbd = shapes::bb_distance(metrics, bbs);
        let left = self.df.sb;
        let right = metrics.width - metrics.sb;
        let mut parts = vec![
            shapes::vbar_l(self.kit, metrics, left, 0.0, metrics.cap, bbs)
                .geometry(kurbo::Affine::IDENTITY),
            self.arc(metrics.cap, right, 0.0, bbs, Body::Standard)
                .geometry(kurbo::Affine::IDENTITY),
        ];
        let outline = self.arc_outline(metrics.cap, right, 1.0, bbs, Body::Standard);
        parts.push(lemmata_ptl::glyph::intersection(vec![
            Geometry::Spiro {
                gizmo: kurbo::Affine::IDENTITY,
                closed: outline.closed,
                knots: outline.knots,
            },
            lemmata_ptl::glyph::union(vec![
                shapes::vbar_l(self.kit, metrics, left + bbd, 0.0, metrics.cap, bbs)
                    .geometry(kurbo::Affine::IDENTITY),
                shapes::vbar_r(
                    self.kit,
                    metrics,
                    right - bbd - metrics.ox,
                    0.0,
                    metrics.cap,
                    bbs,
                )
                .geometry(kurbo::Affine::IDENTITY),
            ]),
        ]));

        Geometry::Combine(parts)
    }
}

/// The variant name `SuffixCfg.weave` gives a body and a serif.
#[must_use]
pub fn suffix(body: &str, serif: &str) -> String {
    lemmata_ptl::util::join_camel(body, serif)
}

/// Every variant the table names, as `(suffix, body, serif)`.
#[must_use]
pub fn variants() -> Vec<(String, Body, Serif)> {
    let mut out = Vec::with_capacity(BODIES.len() * SERIFS.len());
    for (body_key, body) in BODIES {
        for (serif_key, serif) in SERIFS {
            out.push((suffix(body_key, serif_key), *body, *serif));
        }
    }

    out
}
