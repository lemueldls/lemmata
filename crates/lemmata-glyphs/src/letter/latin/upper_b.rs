//! The upper-case `B` from `letter/latin/upper-b.ptl`.
//!
//! A stem and two bowls. Four bodies differ in the bar's position and in
//! whether a gap interrupts the stem, and three serif settings add the outer
//! serifs. `BStroke` adds a bar and `smcpB` is the small capital.
//!
//! `BHookLeft`, `BBar`, the currency signs, `latn/Beta` and the Cyrillic ve
//! shapes need hooks, overlays or glyphs outside this file and are not ported.
//! The anchors are not either.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops, clippy::similar_names)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::control::Item;
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::{Metrics, WeightControl};
use lemmata_ptl::serif::SerifFrame;
use lemmata_ptl::shapes::{self, Design};
use lemmata_ptl::spiro_kit::{Dispiro, Kit};
use lemmata_ptl::util::{join_camel, mix};

/// The four bodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Standard,
    StandardInterrupted,
    MoreAsymmetric,
    MoreAsymmetricInterrupted,
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
    ("standardInterrupted", Body::StandardInterrupted),
    ("moreAsymmetric", Body::MoreAsymmetric),
    ("moreAsymmetricInterrupted", Body::MoreAsymmetricInterrupted),
];

/// The serifs, in the table's order.
pub const SERIFS: &[(&str, Serif)] = &[
    ("serifless", Serif::Serifless),
    ("unilateralSerifed", Serif::Unilateral),
    ("bilateralSerifed", Serif::Bilateral),
];

const BAR_POS: f64 = 0.52;
const ASYMMETRIC_BAR_POS: f64 = 0.60;
const ARC_MIX: f64 = 0.93;

/// The upper-case `B` under construction.
pub struct UpperB<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl UpperB<'_> {
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

    /// `BShapeArcsT`: the two bowls, as two di-spiros.
    fn arcs(
        &self,
        top: f64,
        stroke: f64,
        bar_pos: f64,
        top_arc_shift: f64,
        top_arc_inner_shift: f64,
        offset: f64,
    ) -> (Dispiro, Dispiro) {
        let metrics = self.metrics;
        let sb = self.left_sb();
        let right_sb = self.right_sb();
        let bowl = mix(stroke, top, bar_pos);
        let mock_bowl_depth =
            shapes::bowl_x_depth(metrics, top, bowl - stroke, sb, right_sb, stroke);
        let curve_left_bot =
            mix(metrics.width, right_sb, 1.5) - mock_bowl_depth + top_arc_inner_shift;
        let curve_left_top = curve_left_bot.min(
            mix(metrics.width, right_sb, 1.5) - metrics.ox - stroke * 1.375 + top_arc_inner_shift,
        );
        let ada_bot = metrics.arch_depth_a_of(metrics.arch_depth * 0.9, None);
        let adb_bot = metrics.arch_depth_b_of(metrics.arch_depth * 0.9, None);
        let ada_top = metrics.arch_depth_a_of(
            metrics.arch_depth * 0.9 - (curve_left_bot - curve_left_top),
            None,
        );
        let adb_top = metrics.arch_depth_b_of(
            metrics.arch_depth * 0.9 - (curve_left_bot - curve_left_top),
            None,
        );
        let x_top_arc_right = mix(sb, right_sb, ARC_MIX) - metrics.ox * 2.0 + top_arc_shift;
        let y_top_arc_right = shapes::ysmooth_mid_r(
            top,
            bowl - stroke,
            Some(ada_top),
            Some(adb_top),
            ada_top,
            adb_top,
        );
        let x_bot_arc_right = right_sb - metrics.ox * 2.0;
        let y_bot_arc_right =
            shapes::ysmooth_mid_r(bowl, 0.0, Some(ada_bot), Some(adb_bot), ada_bot, adb_bot);
        let fine = stroke * metrics.c_thin;

        let upper = self.kit.dispiro(vec![
            Item::Af(self.kit.widths_rhs(Some(stroke))),
            self.kit.knot(
                KnotType::Left,
                sb - metrics.o,
                top - offset,
                self.kit.heading(shapes::rightward()),
            ),
            self.kit.knot(
                KnotType::Right,
                self.adjust_x_top(curve_left_top, stroke),
                top - offset,
                Vec::new(),
            ),
            self.kit.archv(None, None),
            self.kit.knot(
                KnotType::G4,
                x_top_arc_right - offset,
                y_top_arc_right,
                Vec::new(),
            ),
            self.kit.arcvh(None, None),
            self.kit.knot(
                KnotType::Left,
                self.adjust_x_bot(curve_left_top, stroke),
                bowl - fine + offset,
                self.kit.widths_rhs(Some(fine)),
            ),
            self.kit.knot(
                KnotType::Right,
                sb - metrics.o,
                bowl - fine + offset,
                self.kit.heading(shapes::leftward()),
            ),
        ]);
        let lower = self.kit.dispiro(vec![
            Item::Af(self.kit.widths_rhs(Some(fine))),
            self.kit.knot(
                KnotType::Left,
                sb - metrics.o,
                bowl - stroke + fine - offset,
                self.kit.heading(shapes::rightward()),
            ),
            self.kit.knot(
                KnotType::Right,
                self.adjust_x_top(curve_left_bot, stroke),
                bowl - stroke + fine - offset,
                Vec::new(),
            ),
            self.kit.archv(None, None),
            self.kit.knot(
                KnotType::G4,
                x_bot_arc_right - offset,
                y_bot_arc_right,
                self.kit.widths_rhs(Some(stroke)),
            ),
            self.kit.arcvh(None, None),
            self.kit.knot(
                KnotType::Left,
                self.adjust_x_bot(curve_left_bot, stroke),
                offset,
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Right,
                sb - metrics.o,
                offset,
                self.kit.heading(shapes::leftward()),
            ),
        ]);

        (upper, lower)
    }

    /// `InterruptShape`: the cut that separates the two bowls.
    fn interrupt(&self, top: f64, stroke: f64, bar_pos: f64) -> Dispiro {
        let metrics = self.metrics;
        let bowl = mix(stroke, top, bar_pos);
        let width = (0.20 * (self.right_sb() - self.left_sb())) / metrics.hv_contrast
            * (self.weight().advice_stroke(5.0, 1.0, 1.0) / metrics.stroke);

        shapes::vbar_l(
            self.kit,
            metrics,
            self.left_sb() + self.hsw_to_v(stroke),
            bowl - stroke - metrics.tiny,
            bowl + metrics.tiny,
            width,
        )
    }

    /// `BShape`: the stem, the two bowls and the serifs.
    // The signature is upstream's: the height, the stroke, the bar and the two
    // arc shifts.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn shape(
        &self,
        top: f64,
        stroke: f64,
        bar_pos: f64,
        top_arc_shift: f64,
        top_arc_inner_shift: f64,
        serif_top: bool,
        serif_bot: bool,
    ) -> Geometry {
        let metrics = self.metrics;
        let (upper, lower) = self.arcs(
            top,
            stroke,
            bar_pos,
            top_arc_shift,
            top_arc_inner_shift,
            0.0,
        );
        let mut parts = vec![
            shapes::vbar_l(self.kit, metrics, self.left_sb(), 0.0, top, stroke)
                .geometry(kurbo::Affine::IDENTITY),
            upper.geometry(kurbo::Affine::IDENTITY),
            lower.geometry(kurbo::Affine::IDENTITY),
        ];
        let frame = SerifFrame::from_df(metrics, self.df, top, 0.0, None, None, false);
        if serif_top {
            parts.push(frame.lt_outer(self.kit, metrics));
        }
        if serif_bot {
            parts.push(frame.lb_outer(self.kit, metrics));
        }

        Geometry::Combine(parts)
    }

    /// The body the table selects, at one height.
    fn build(&self, body: Body, top: f64, stroke: f64, serif: Serif) -> Geometry {
        let (serif_top, serif_bot) = serif.flags();
        let (bar_pos, shift, inner_shift) = match body {
            Body::Standard | Body::StandardInterrupted => (BAR_POS, 0.0, 0.0),
            Body::MoreAsymmetric | Body::MoreAsymmetricInterrupted => (
                ASYMMETRIC_BAR_POS,
                -0.05 * (self.right_sb() - self.left_sb()),
                -0.10 * (self.right_sb() - self.left_sb()),
            ),
        };
        let shape = self.shape(
            top,
            stroke,
            bar_pos,
            shift,
            inner_shift,
            serif_top,
            serif_bot,
        );
        if matches!(
            body,
            Body::StandardInterrupted | Body::MoreAsymmetricInterrupted
        ) {
            lemmata_ptl::glyph::difference(vec![
                shape,
                self.interrupt(top, stroke, bar_pos)
                    .geometry(kurbo::Affine::IDENTITY),
            ])
        } else {
            shape
        }
    }

    /// `B.<suffix>`: the body the table selects.
    #[must_use]
    pub fn capital(&self, body: Body, serif: Serif) -> Geometry {
        let stroke = self
            .weight()
            .advice_stroke2(2.0, 3.0, self.metrics.cap, 1.0);

        self.build(body, self.metrics.cap, stroke, serif)
    }

    /// `smcpB.<suffix>`.
    #[must_use]
    pub fn small(&self, body: Body, serif: Serif) -> Geometry {
        let stroke = self.weight().advice_stroke2(2.0, 3.0, self.metrics.xh, 1.0);

        self.build(body, self.metrics.xh, stroke, serif)
    }

    /// `BStroke.<suffix>`.
    #[must_use]
    pub fn stroke(&self, body: Body, serif: Serif) -> Geometry {
        let metrics = self.metrics;
        let stroke = self.weight().advice_stroke2(2.0, 3.0, metrics.cap, 1.0);
        let (bar_pos, _, _) = match body {
            Body::Standard | Body::StandardInterrupted => (BAR_POS, 0.0, 0.0),
            Body::MoreAsymmetric | Body::MoreAsymmetricInterrupted => {
                (ASYMMETRIC_BAR_POS, 0.0, 0.0)
            }
        };
        let bar = shapes::letter_bar_overlay_l_in(
            self.kit,
            metrics,
            self.left_sb(),
            stroke,
            mix(stroke, metrics.cap, bar_pos) - stroke,
            0.5,
            (0.0, self.right_sb() - self.hsw_to_v(stroke)),
            stroke,
            metrics.overlay_stroke,
        );

        Geometry::Combine(vec![
            self.capital(body, serif),
            bar.geometry(kurbo::Affine::IDENTITY),
        ])
    }
}

/// The variant name `SuffixCfg.weave` gives a body and a serif.
#[must_use]
pub fn suffix(body: &str, serif: &str) -> String {
    join_camel(body, serif)
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

impl UpperB<'_> {
    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for (suffix, kind, serif) in variants() {
            out.push((format!("B.{suffix}"), self.capital(kind, serif)));
            out.push((format!("BStroke.{suffix}"), self.stroke(kind, serif)));
            out.push((format!("smcpB.{suffix}"), self.small(kind, serif)));
        }

        out
    }
}
