//! The upper-case `P` from `letter/latin/upper-p.ptl`.
//!
//! A stem and a bowl that closes on the crossbar, with two openness settings and
//! three serif settings. The open forms cut a gap in the stem.
//!
//! `grek/Rho`, `PHookLeft`, the stroke variants, the ruble sign and the rotunda
//! and reversed bodies need hooks, masks or glyphs outside this file and are not
//! ported. The anchors are not either.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    control::Item,
    letter_model::DivFrame,
    metrics::{Metrics, WeightControl},
    shapes::{self, Design, HSerifSide},
    spiro_kit::Kit,
    util::{join_camel, mix},
};

/// The two openness settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Openness {
    Closed,
    Open,
}

/// The three serif settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serif {
    Serifless,
    MotionSerifed,
    Serifed,
}

impl Serif {
    const fn has_slab(self) -> bool {
        !matches!(self, Self::Serifless)
    }

    const fn has_foot(self) -> bool {
        matches!(self, Self::Serifed)
    }
}

/// The openness settings, in the table's order.
pub const OPENNESS: &[(&str, Openness)] = &[("closed", Openness::Closed), ("open", Openness::Open)];

/// The serifs, in the table's order.
pub const SERIFS: &[(&str, Serif)] = &[
    ("serifless", Serif::Serifless),
    ("motionSerifed", Serif::MotionSerifed),
    ("serifed", Serif::Serifed),
];

/// The upper-case `P` under construction.
pub struct UpperP<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl UpperP<'_> {
    const DEFAULT_MUL: f64 = 1.25;
    const SW_BELOW_BAR: f64 = 0.25;

    const fn left_sb(&self) -> f64 {
        self.df.sb
    }

    const fn right_sb(&self) -> f64 {
        self.df.width - self.df.sb
    }

    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    fn adjust_x_top(&self, x: f64, sw: f64) -> f64 {
        x - self.metrics.correction_o_mid_x * sw
    }

    fn adjust_x_bot(&self, x: f64, sw: f64) -> f64 {
        x + self.metrics.correction_o_mid_x * sw
    }

    fn weight(&self) -> WeightControl {
        WeightControl::new(
            self.metrics.stroke,
            self.metrics.width,
            self.metrics.sb,
            self.metrics.hv_contrast,
        )
    }

    /// `HBarPos`, the crossbar's fraction.
    fn h_bar_pos(&self) -> f64 {
        self.design.h_bar_pos - 0.09 * self.metrics.stroke / self.metrics.cap
    }

    /// `PBarPosY`: where the crossbar sits.
    fn bar_pos_y(&self, top: f64, sw: f64) -> f64 {
        let bp = 1.0 - self.h_bar_pos();

        mix(0.0, top - sw, bp) - sw * Self::SW_BELOW_BAR
    }

    /// `PShapeOutlineKnots`.
    fn outline_items(&self, top: f64, sw: f64) -> Vec<Item> {
        let metrics = self.metrics;
        let mul = Self::DEFAULT_MUL;
        let overshoot = metrics.ox * 2.0;
        let bowl_top = top;
        let bowl_bottom = self.bar_pos_y(top, sw);
        let turn_radius = shapes::bowl_x_depth(
            metrics,
            bowl_top,
            bowl_bottom,
            self.left_sb(),
            self.right_sb(),
            sw,
        );
        let ada = metrics.arch_depth_a_of(metrics.arch_depth * 0.9, Some(self.df.width));
        let adb = metrics.arch_depth_b_of(metrics.arch_depth * 0.9, Some(self.df.width));
        let left = mix(0.0, self.left_sb(), mul);
        let right = self.right_sb();
        let turn = shapes::ysmooth_mid_r(bowl_top, bowl_bottom, Some(ada), Some(adb), ada, adb);

        vec![
            self.kit.knot(
                KnotType::Left,
                left - metrics.o,
                bowl_top,
                self.kit.heading(shapes::rightward()),
            ),
            self.kit.knot(
                KnotType::Right,
                self.adjust_x_top(right - turn_radius, sw),
                bowl_top,
                Vec::new(),
            ),
            self.kit.archv(None, None),
            self.kit
                .knot(KnotType::G4, right - overshoot, turn, Vec::new()),
            self.kit.arcvh(None, None),
            self.kit.knot(
                KnotType::Left,
                self.adjust_x_bot(right - turn_radius, sw),
                bowl_bottom,
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Right,
                left - metrics.o,
                bowl_bottom,
                self.kit.heading(shapes::leftward()),
            ),
        ]
    }

    fn slab(&self, top: f64, sw: f64, serif: Serif) -> Vec<Geometry> {
        let metrics = self.metrics;
        let left = mix(0.0, self.left_sb(), Self::DEFAULT_MUL);
        let mut parts = Vec::new();
        if !serif.has_slab() {
            return parts;
        }
        parts.push(Geometry::Tagged(Box::new(
            shapes::hserif(
                self.kit,
                metrics,
                HSerifSide::LeftTop,
                left,
                top,
                metrics.side_jut,
                sw,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        )));
        if serif.has_foot() {
            let shrink = self.hsw_to_v(0.5 * (metrics.stroke - sw));
            let x = left + self.hsw_to_v(0.5 * sw);
            parts.push(Geometry::Tagged(Box::new(Geometry::Combine(vec![
                shapes::hserif(
                    self.kit,
                    metrics,
                    HSerifSide::LeftBottom,
                    left,
                    0.0,
                    metrics.side_jut,
                    sw,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
                shapes::hserif(
                    self.kit,
                    metrics,
                    HSerifSide::RightBottom,
                    x,
                    0.0,
                    metrics.mid_jut_side - shrink,
                    sw,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            ]))));
        }

        parts
    }

    /// `PShape`.
    #[must_use]
    pub fn shape(&self, top: f64, sw: f64, serif: Serif) -> Geometry {
        let metrics = self.metrics;
        let mul = Self::DEFAULT_MUL;
        let mut parts = vec![
            Geometry::Tagged(Box::new(
                self.kit
                    .dispiro({
                        let mut items = vec![Item::Af(self.kit.widths_rhs(Some(sw)))];
                        items.extend(self.outline_items(top, sw));
                        items
                    })
                    .geometry(kurbo::Affine::IDENTITY),
            )),
            Geometry::Tagged(Box::new(
                shapes::vbar_l(
                    self.kit,
                    metrics,
                    mix(0.0, self.left_sb(), mul),
                    0.0,
                    top,
                    sw,
                )
                .geometry(kurbo::Affine::IDENTITY),
            )),
        ];
        parts.extend(self.slab(top, sw, serif));

        Geometry::Combine(parts)
    }

    /// `PShape.OpenGap`.
    fn open_gap(&self, top: f64, bot: f64, sw: f64) -> Geometry {
        let metrics = self.metrics;
        let mul = Self::DEFAULT_MUL;
        let bar = self.bar_pos_y(top, sw);
        let width = (0.2 * (self.right_sb() - self.left_sb())) / metrics.hv_contrast
            * (self.weight().advice_stroke(5.0, 1.0, 1.0) / metrics.stroke);
        let x = mix(0.0, self.left_sb(), mul) + self.hsw_to_v(sw);
        let y_down = (bar - 0.5 * sw - metrics.tiny).min(bot);
        let y_up = (bar + 0.5 * sw + metrics.tiny).max(mix(top, bar, 0.5));

        shapes::vbar_l(self.kit, metrics, x, y_down, y_up, width).geometry(kurbo::Affine::IDENTITY)
    }

    /// `P.<suffix>`.
    #[must_use]
    pub fn capital(&self, openness: Openness, serif: Serif) -> Geometry {
        let sw = self
            .weight()
            .advice_stroke2(2.0, 3.0, self.metrics.cap, 1.0);
        let foot = if serif.has_foot() { sw } else { 0.0 };
        let shape = self.shape(self.metrics.cap, sw, serif);

        if openness == Openness::Open {
            lemmata_ptl::glyph::difference(vec![shape, self.open_gap(self.metrics.cap, foot, sw)])
        } else {
            shape
        }
    }

    /// `smcpP.<suffix>`.
    #[must_use]
    pub fn small(&self, openness: Openness, serif: Serif) -> Geometry {
        let sw = self.weight().advice_stroke2(2.0, 3.0, self.metrics.xh, 1.0);
        let foot = if serif.has_foot() { sw } else { 0.0 };
        let shape = self.shape(self.metrics.xh, sw, serif);

        if openness == Openness::Open {
            lemmata_ptl::glyph::difference(vec![shape, self.open_gap(self.metrics.xh, foot, sw)])
        } else {
            shape
        }
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for (openness_key, openness) in OPENNESS {
            for (serif_key, serif) in SERIFS {
                let suffix = join_camel(openness_key, serif_key);
                out.push((format!("P.{suffix}"), self.capital(*openness, *serif)));
                out.push((format!("smcpP.{suffix}"), self.small(*openness, *serif)));
            }
        }

        out
    }
}
