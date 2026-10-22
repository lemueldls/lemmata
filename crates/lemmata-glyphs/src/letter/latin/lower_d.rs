//! The lower-case `d` from `letter/latin/lower-d.ptl`.
//!
//! The file builds a bowl over a stem raised to the ascender. Six bodies differ
//! in how the bowl's foot meets the stem and four serif settings add the serifs
//! the variant selects. `dStroke`, `latn/de`, `dHookTop` and `mathbb/d` start
//! from the same bowl, and `dCaron/base` is the same drawing in a narrower
//! frame.
//!
//! The bowl is `OBarRight` and the stem `VBar.r`, both from the shared shape
//! layer. The frame is a parameter rather than a field: the variant table asks
//! for `DivFrame 1` and the caron base for `DivFrame 0.95`, so one build covers
//! both.
//!
//! `MarkSet.b` and `LeaningAnchor.Above` place anchors rather than geometry, so
//! they are not ported here. `dCurlyTail` and `cyrl/djeKomi` need shapes outside
//! this file and are not either.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::{
    letter_model::DivFrame,
    metrics::Metrics,
    shapes::{self, Design, HSerifSide, OBarContext, OBarRing, VSerifSide},
    spiro_kit::Kit,
    util::{join_camel, mix},
};

/// The six bodies the variant table selects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    /// `ToothedBody`: the bowl's foot meets the stem square.
    Toothed,
    /// `TailedBody`: the foot turns right.
    Tailed,
    /// `FlatBottomBody`: the foot is flat into the stem.
    FlatBottom,
    /// `ToothlessCornerBody`: the foot has a corner at the rise.
    ToothlessCorner,
    /// `ToothlessCornerRTBBody`: the corner, with the stem reaching the foot.
    ToothlessCornerRtb,
    /// `ToothlessRoundedBody`: the foot is rounded into the stem.
    ToothlessRounded,
}

/// The four serif settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serif {
    Serifless,
    TopSerifed,
    BottomSerifed,
    Serifed,
}

impl Serif {
    /// Whether the variant carries the top serif.
    #[must_use]
    pub const fn has_top(self) -> bool {
        matches!(self, Self::TopSerifed | Self::Serifed)
    }

    /// Whether the variant carries the foot serif.
    #[must_use]
    pub const fn has_bottom(self) -> bool {
        matches!(self, Self::BottomSerifed | Self::Serifed)
    }
}

/// The bodies, in the table's order.
pub const BODIES: &[(&str, Body)] = &[
    ("toothed", Body::Toothed),
    ("tailed", Body::Tailed),
    ("flatBottom", Body::FlatBottom),
    ("toothlessCorner", Body::ToothlessCorner),
    ("toothlessCornerRTB", Body::ToothlessCornerRtb),
    ("toothlessRounded", Body::ToothlessRounded),
];

/// The serifs, in the table's order.
pub const SERIFS: &[(&str, Serif)] = &[
    ("serifless", Serif::Serifless),
    ("topSerifed", Serif::TopSerifed),
    ("bottomSerifed", Serif::BottomSerifed),
    ("serifed", Serif::Serifed),
];

/// The lower-case `d` under construction, in the frame it is drawn in.
pub struct LowerD<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerD<'_> {
    const fn left_sb(&self) -> f64 {
        self.df.sb
    }

    const fn right_sb(&self) -> f64 {
        self.df.width - self.df.sb
    }

    const fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    const fn context(&self) -> OBarContext<'_> {
        OBarContext {
            kit: self.kit,
            metrics: self.metrics,
            design: self.design,
        }
    }

    /// The ring the bowl is drawn in. `OBarRight`'s parameters all default, so
    /// the ring is x-height tall with the frame's stroke and arch depths.
    const fn ring(&self) -> OBarRing {
        OBarRing {
            top: self.metrics.xh,
            bot: 0.0,
            left: self.left_sb(),
            right: self.right_sb(),
            sw: self.df.mvs,
            fine: self.df.shoulder_fine,
            ada: self.df.small_arch_depth_a,
            adb: self.df.small_arch_depth_b,
        }
    }

    /// `ystart`, the point the bowl's cut starts from. The right side reads
    /// `adb` where the left reads `ada`.
    fn ystart(&self) -> f64 {
        (self.metrics.xh - self.df.small_arch_depth_b) - self.metrics.tiny
    }

    fn tagged(geometry: Geometry) -> Geometry {
        Geometry::Tagged(Box::new(geometry))
    }

    /// The bowl and the stem, as the body draws them.
    #[must_use]
    pub fn body(&self, body: Body, y_top: f64) -> Vec<Geometry> {
        let metrics = self.metrics;
        let ring = self.ring();
        let context = self.context();
        let right = self.right_sb();
        let sw = self.df.mvs;
        let stem = |y_down: f64| {
            Self::tagged(
                shapes::vbar_r(self.kit, metrics, right, y_down, y_top, sw)
                    .geometry(kurbo::Affine::IDENTITY),
            )
        };

        match body {
            Body::Toothed => vec![shapes::obar_right_shape(&context, &ring), stem(0.0)],
            Body::Tailed => {
                vec![
                    shapes::obar_right_shape(&context, &ring),
                    Self::tagged(
                        shapes::rightward_tailed_bar(
                            self.kit, metrics, self.df, right, 0.0, y_top, sw,
                        )
                        .geometry(kurbo::Affine::IDENTITY),
                    ),
                ]
            }
            Body::FlatBottom => {
                vec![
                    shapes::obar_right_toothless_corner(&context, &ring, 0.0, self.ystart()),
                    stem(0.0),
                ]
            }
            Body::ToothlessCorner => {
                vec![
                    shapes::obar_right_toothless_corner(
                        &context,
                        &ring,
                        metrics.d_toothless_rise,
                        self.ystart(),
                    ),
                    stem(metrics.d_toothless_rise),
                ]
            }
            Body::ToothlessCornerRtb => {
                vec![
                    shapes::obar_right_toothless_corner(
                        &context,
                        &ring,
                        metrics.d_toothless_rise,
                        self.ystart(),
                    ),
                    stem(0.0),
                ]
            }
            Body::ToothlessRounded => {
                vec![shapes::obar_right_toothless_rounded(
                    &context,
                    &ring,
                    y_top,
                    self.ystart(),
                )]
            }
        }
    }

    /// `TopSerif`.
    #[must_use]
    pub fn top_serif(&self, y_top: f64) -> Geometry {
        Self::tagged(
            shapes::hserif(
                self.kit,
                self.metrics,
                HSerifSide::LeftTop,
                self.right_sb() - self.hsw_to_v(self.df.mvs),
                y_top,
                self.metrics.side_jut,
                self.df.mvs,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// `BaseSerif`.
    #[must_use]
    pub fn base_serif(&self) -> Geometry {
        Self::tagged(
            shapes::hserif(
                self.kit,
                self.metrics,
                HSerifSide::RightBottom,
                self.right_sb(),
                0.0,
                self.metrics.side_jut,
                self.df.mvs,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// The plain `d` variant: the bowl, the stem and the serifs.
    #[must_use]
    pub fn glyph(&self, body: Body, serif: Serif) -> Geometry {
        let mut parts = self.body(body, self.metrics.ascender);
        if serif.has_top() {
            parts.push(self.top_serif(self.metrics.ascender));
        }
        if serif.has_bottom() {
            parts.push(self.base_serif());
        }

        Geometry::Combine(parts)
    }

    /// `dStroke`: the base glyph with a bar across it.
    #[must_use]
    pub fn d_stroke(&self, body: Body, serif: Serif) -> Geometry {
        let metrics = self.metrics;
        let top = if serif.has_top() {
            metrics.ascender - metrics.stroke
        } else {
            metrics.ascender
        };
        let bar = shapes::letter_bar_overlay_r_in(
            self.kit,
            metrics,
            self.right_sb(),
            metrics.xh,
            top,
            0.5,
            (0.0, self.df.width),
            metrics.stroke,
            metrics.overlay_stroke,
        );

        Geometry::Combine(vec![
            self.glyph(body, serif),
            bar.geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// `latn/de`: the base glyph with a top bar from the left.
    #[must_use]
    pub fn latn_de(&self, body: Body, serif: Serif) -> Geometry {
        let metrics = self.metrics;
        let left = self.left_sb();
        let right = self.right_sb();
        let x_top_left = mix(left, right, 0.1);
        let mut parts = self.body(body, metrics.ascender);
        parts.push(
            shapes::hbar_t(
                self.kit,
                metrics,
                x_top_left,
                right + metrics.o,
                metrics.ascender,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        );
        if serif.has_top() {
            parts.push(
                shapes::vserif(
                    self.kit,
                    metrics,
                    VSerifSide::DownLeft,
                    x_top_left,
                    metrics.ascender,
                    metrics
                        .v_jut
                        .min(mix(metrics.stroke, metrics.ascender - metrics.xh, 0.5)),
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            );
            parts.push(
                shapes::hserif(
                    self.kit,
                    metrics,
                    HSerifSide::RightTop,
                    right,
                    metrics.ascender,
                    metrics.side_jut,
                    metrics.stroke,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            );
        }
        if serif.has_bottom() {
            parts.push(self.base_serif());
        }

        Geometry::Combine(parts)
    }

    /// `dHookTop`: the base glyph whose stem stops short and turns into a hook.
    #[must_use]
    pub fn d_hook_top(&self, body: Body, serif: Serif) -> Geometry {
        let metrics = self.metrics;
        let mut parts = self.body(body, metrics.ascender - metrics.hook - metrics.half_stroke);
        parts.extend(shapes::top_hook_r_bar_inner(
            self.kit,
            metrics,
            self.right_sb(),
            0.5 * metrics.xh,
            metrics.ascender,
            self.df.mvs,
            1.0,
        ));
        if serif.has_bottom() {
            parts.push(self.base_serif());
        }

        Geometry::Combine(parts)
    }

    /// `mathbb/d`: the blackboard bold `d`.
    #[must_use]
    pub fn mathbb_d(&self) -> Geometry {
        let metrics = self.metrics;
        let bbs = shapes::bb_stroke(self.df);
        let bbd = shapes::bb_distance(metrics, bbs);
        let left = self.left_sb();
        let right = self.right_sb();
        let mut parts =
            shapes::bb_bar_right(self.kit, metrics, right, 0.0, metrics.ascender, bbs, bbd);
        let ring_shape = shapes::oshape_outline_no_overshoot(
            self.kit,
            metrics,
            self.design,
            metrics.xh,
            0.0,
            left,
            right - bbd - bbs * metrics.hv_contrast,
            None,
            None,
            None,
        );
        parts.push(lemmata_ptl::glyph::intersection(vec![
            Geometry::Spiro {
                gizmo: kurbo::Affine::IDENTITY,
                closed: ring_shape.closed,
                knots: ring_shape.knots,
            },
            shapes::vbar_l(
                self.kit,
                metrics,
                left + bbd + metrics.ox,
                0.0,
                metrics.xh,
                bbs,
            )
            .geometry(kurbo::Affine::IDENTITY),
        ]));
        let ring = OBarRing {
            right: right - bbd,
            sw: bbs,
            ..self.ring()
        };
        parts.push(shapes::obar_right_shape(&self.context(), &ring));

        Geometry::Combine(parts)
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

impl LowerD<'_> {
    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let narrow = DivFrame::new(self.metrics.clone(), Some(0.95), None, None);
        let caron = LowerD {
            df: &narrow,
            metrics: self.metrics,
            kit: self.kit,
            design: self.design,
        };
        let mut out = vec![("mathbb/d".to_string(), self.mathbb_d())];
        for (suffix, body, serif) in variants() {
            out.push((format!("d.{suffix}"), self.glyph(body, serif)));
            out.push((format!("dCaron/base.{suffix}"), caron.glyph(body, serif)));
            out.push((format!("dStroke.{suffix}"), self.d_stroke(body, serif)));
            out.push((format!("latn/de.{suffix}"), self.latn_de(body, serif)));
            if !serif.has_top() {
                out.push((format!("dHookTop.{suffix}"), self.d_hook_top(body, serif)));
            }
        }

        out
    }
}
