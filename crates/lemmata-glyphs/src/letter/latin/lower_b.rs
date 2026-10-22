//! The lower-case `b` from `letter/latin/lower-b.ptl`.
//!
//! The file builds a bowl over a stem. Five bodies differ in how the bowl's
//! foot meets the stem, and four serif settings add the serifs the variant
//! selects. The variants are the weave of those two tables: `b.serifless`,
//! `b.bottomCutSerifless`, and so on. `bStroke`, `bSlash` and `latn/be` start
//! from the same bowl, and `bHookTop` adds a hook where the stem ends at the
//! x-height.
//!
//! The stem is `VBar.l`, the bowl is `OBarLeft`, and both come from the shared
//! shape layer. What is left of the file is the table and the four derived
//! forms, which is why it is the second definition ported.
//!
//! `MarkSet.b` and `LeaningAnchor.Above` place anchors rather than geometry, so
//! they are not ported here. The accented compositions the file derives need
//! glyphs outside it and are not either.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::{
    letter_model::DivFrame,
    metrics::Metrics,
    shapes::{self, Design, HSerifSide, OBarContext, OBarRing, VSerifSide},
    spiro_kit::Kit,
    util::mix,
};

/// The five bodies the variant table selects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    /// `ToothedBody`: the bowl's foot meets the stem square.
    Toothed,
    /// `BottomCutBody`: the foot is cut at the half stroke.
    BottomCut,
    /// `FlatBottomBody`: the foot is flat into the stem.
    FlatBottom,
    /// `ToothlessCornerBody`: the foot has a corner at the rise.
    ToothlessCorner,
    /// `ToothlessRoundedBody`: the foot is rounded into the stem.
    ToothlessRounded,
}

/// The four serif settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serif {
    /// `serifless`: no serifs.
    Serifless,
    /// `motionSerifed`: the top serif only, the stem reaching the ascender.
    MotionSerifed,
    /// `bottomSerifed`: the foot serif only.
    BottomSerifed,
    /// `serifed`: both serifs.
    Serifed,
}

impl Serif {
    /// Whether the stem reaches the ascender, which the table calls `doST`.
    #[must_use]
    pub const fn reaches_ascender(self) -> bool {
        matches!(self, Self::MotionSerifed | Self::Serifed)
    }
}

/// The bodies, in the table's order.
pub const BODIES: &[(&str, Body)] = &[
    ("", Body::Toothed),
    ("bottomCut", Body::BottomCut),
    ("flatBottom", Body::FlatBottom),
    ("toothlessCorner", Body::ToothlessCorner),
    ("toothlessRounded", Body::ToothlessRounded),
];

/// The serifs, in the table's order.
pub const SERIFS: &[(&str, Serif)] = &[
    ("serifless", Serif::Serifless),
    ("motionSerifed", Serif::MotionSerifed),
    ("bottomSerifed", Serif::BottomSerifed),
    ("serifed", Serif::Serifed),
];

/// The lower-case `b` under construction.
pub struct LowerB<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerB<'_> {
    const fn left_sb(&self) -> f64 {
        self.df.sb
    }

    const fn right_sb(&self) -> f64 {
        self.df.width - self.df.sb
    }

    const fn context(&self) -> OBarContext<'_> {
        OBarContext {
            kit: self.kit,
            metrics: self.metrics,
            design: self.design,
        }
    }

    /// The ring the bowl is drawn in. `OBarLeft`'s parameters all default, so
    /// the ring is x-height tall with the small arch depths.
    const fn ring(&self) -> OBarRing {
        let metrics = self.metrics;

        OBarRing {
            top: metrics.xh,
            bot: 0.0,
            left: self.left_sb(),
            right: self.right_sb(),
            sw: metrics.stroke,
            fine: metrics.shoulder_fine,
            ada: metrics.small_arch_depth_a,
            adb: metrics.small_arch_depth_b,
        }
    }

    /// `ystart`, the point the bowl's cut starts from.
    fn ystart(&self) -> f64 {
        (self.metrics.xh - self.metrics.small_arch_depth_a) - self.metrics.tiny
    }

    /// The bowl and the stem, as the body draws them.
    #[must_use]
    pub fn body(&self, body: Body, y_top: f64) -> Vec<Geometry> {
        let metrics = self.metrics;
        let ring = self.ring();
        let context = self.context();
        let stem = |y_down: f64| {
            shapes::vbar_l(
                self.kit,
                metrics,
                self.left_sb(),
                y_down,
                y_top,
                metrics.stroke,
            )
            .geometry(kurbo::Affine::IDENTITY)
        };

        match body {
            Body::Toothed => {
                vec![
                    shapes::obar_left_shape(&context, &ring).geometry(kurbo::Affine::IDENTITY),
                    stem(0.0),
                ]
            }
            Body::BottomCut => {
                let cut_x = self.left_sb() + metrics.stroke * metrics.hv_contrast;
                let cut = self.kit.spiro_outline(vec![
                    self.kit.knot(
                        lemmata_geom::KnotType::Corner,
                        self.left_sb(),
                        0.0,
                        Vec::new(),
                    ),
                    self.kit.knot(
                        lemmata_geom::KnotType::Corner,
                        self.left_sb(),
                        metrics.half_stroke,
                        Vec::new(),
                    ),
                    self.kit.knot(
                        lemmata_geom::KnotType::Corner,
                        cut_x,
                        metrics.half_stroke,
                        Vec::new(),
                    ),
                ]);

                vec![
                    shapes::obar_left_shape(&context, &ring).geometry(kurbo::Affine::IDENTITY),
                    stem(metrics.half_stroke),
                    Geometry::Spiro {
                        gizmo: kurbo::Affine::IDENTITY,
                        closed: cut.closed,
                        knots: cut.knots,
                    },
                ]
            }
            Body::FlatBottom => {
                vec![
                    shapes::obar_left_toothless_corner(&context, &ring, 0.0, self.ystart())
                        .geometry(kurbo::Affine::IDENTITY),
                    stem(0.0),
                ]
            }
            Body::ToothlessCorner => {
                vec![
                    shapes::obar_left_toothless_corner(
                        &context,
                        &ring,
                        metrics.d_toothless_rise,
                        self.ystart(),
                    )
                    .geometry(kurbo::Affine::IDENTITY),
                    stem(metrics.d_toothless_rise),
                ]
            }
            Body::ToothlessRounded => {
                vec![
                    shapes::obar_left_toothless_rounded(&context, &ring, y_top, self.ystart())
                        .geometry(kurbo::Affine::IDENTITY),
                ]
            }
        }
    }

    /// `LTSerifs`, `LBSerifs` or `FullSerifs`.
    #[must_use]
    pub fn serifs(&self, serif: Serif) -> Vec<Geometry> {
        let metrics = self.metrics;
        let top = || {
            Geometry::Tagged(Box::new(
                shapes::hserif(
                    self.kit,
                    metrics,
                    HSerifSide::LeftTop,
                    self.left_sb(),
                    metrics.ascender,
                    metrics.side_jut,
                    metrics.stroke,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            ))
        };
        let bottom = || {
            Geometry::Tagged(Box::new(
                shapes::hserif(
                    self.kit,
                    metrics,
                    HSerifSide::LeftBottom,
                    self.left_sb(),
                    0.0,
                    metrics.side_jut,
                    metrics.stroke,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            ))
        };

        match serif {
            Serif::Serifless => Vec::new(),
            Serif::MotionSerifed => vec![top()],
            Serif::BottomSerifed => vec![bottom()],
            Serif::Serifed => vec![top(), bottom()],
        }
    }

    /// The plain `b` variant: the bowl, the stem and the serifs.
    #[must_use]
    pub fn glyph(&self, body: Body, serif: Serif) -> Geometry {
        let mut parts = self.body(body, self.metrics.ascender);
        parts.extend(self.serifs(serif));

        Geometry::Combine(parts)
    }

    /// The overlay the slash and the stroke variants share.
    fn y_overlay(&self, serif: Serif) -> f64 {
        let top = if serif.reaches_ascender() {
            self.metrics.ascender - self.metrics.stroke
        } else {
            self.metrics.ascender
        };

        mix(self.metrics.xh, top, 0.5)
    }

    /// `bSlash`: the base glyph with a diagonal overlay.
    #[must_use]
    pub fn b_slash(&self, body: Body, serif: Serif) -> Geometry {
        let metrics = self.metrics;
        let y = self.y_overlay(serif);
        let rise = metrics.ascender - 0.0;
        let slash = self.kit.dispiro(vec![
            lemmata_ptl::control::Item::Af(self.kit.widths_center(Some(metrics.overlay_stroke))),
            self.kit.knot(
                lemmata_geom::KnotType::Left,
                mix(self.left_sb(), 0.0, 0.7),
                y - 0.10 * rise,
                Vec::new(),
            ),
            self.kit.knot(
                lemmata_geom::KnotType::Right,
                mix(self.left_sb(), self.right_sb(), 0.625),
                y + 0.05 * rise,
                Vec::new(),
            ),
        ]);

        Geometry::Combine(vec![
            self.glyph(body, serif),
            slash.geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// `latn/be`: the base glyph with a top bar and, when the stem reaches the
    /// ascender, a serif on its right.
    #[must_use]
    pub fn latn_be(&self, body: Body, serif: Serif) -> Geometry {
        let metrics = self.metrics;
        let x_top_right = mix(self.left_sb(), self.right_sb(), 0.9);
        let mut parts = vec![
            self.glyph(body, serif),
            shapes::hbar_t(
                self.kit,
                metrics,
                self.left_sb() - metrics.o,
                x_top_right,
                metrics.ascender,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        ];
        if serif.reaches_ascender() {
            parts.push(
                shapes::vserif(
                    self.kit,
                    metrics,
                    VSerifSide::DownRight,
                    x_top_right,
                    metrics.ascender,
                    metrics
                        .v_jut
                        .min(mix(metrics.stroke, metrics.ascender - metrics.xh, 0.5)),
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            );
        }

        Geometry::Combine(parts)
    }

    /// `bStroke`: the base glyph with a bar across it.
    #[must_use]
    pub fn b_stroke(&self, body: Body, serif: Serif) -> Geometry {
        let metrics = self.metrics;
        let top = if serif.reaches_ascender() {
            metrics.ascender - metrics.stroke
        } else {
            metrics.ascender
        };
        let bar = shapes::letter_bar_overlay_l_in(
            self.kit,
            metrics,
            self.left_sb(),
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

    /// `bHookTop`: the base glyph with the stem's top turned into a hook. Only
    /// the variants whose stem stops at the x-height get one.
    #[must_use]
    pub fn b_hook_top(&self, body: Body, serif: Serif) -> Geometry {
        let metrics = self.metrics;
        let mut parts = self.body(body, self.metrics.xh);
        parts.extend(self.serifs(serif));
        parts.extend(shapes::top_hook_l_bar_inner(
            self.kit,
            metrics,
            self.left_sb(),
            0.5 * metrics.xh,
            metrics.ascender,
            metrics.stroke,
            1.0,
        ));

        Geometry::Combine(parts)
    }

    /// `mathbb/b`: the blackboard bold `b`.
    #[must_use]
    pub fn mathbb_b(&self) -> Geometry {
        let metrics = self.metrics;
        let bbs = shapes::bb_stroke(self.df);
        let bbd = shapes::bb_distance(metrics, bbs);
        let left = self.left_sb();
        let right = self.right_sb();

        let mut parts =
            shapes::bb_bar_left(self.kit, metrics, left, 0.0, metrics.ascender, bbs, bbd);
        let ring_shape = shapes::oshape_outline_no_overshoot(
            self.kit,
            metrics,
            self.design,
            metrics.xh,
            0.0,
            left + bbd + bbs * metrics.hv_contrast,
            right,
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
            shapes::vbar_r(
                self.kit,
                metrics,
                right - bbd - metrics.ox,
                0.0,
                metrics.xh,
                bbs,
            )
            .geometry(kurbo::Affine::IDENTITY),
        ]));
        let ring = OBarRing {
            left: left + bbd,
            sw: bbs,
            ..self.ring()
        };
        parts.push(
            shapes::obar_left_shape(&self.context(), &ring).geometry(kurbo::Affine::IDENTITY),
        );

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

impl LowerB<'_> {
    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = vec![("mathbb/b".to_string(), self.mathbb_b())];
        for (suffix, body, serif) in variants() {
            out.push((format!("b.{suffix}"), self.glyph(body, serif)));
            out.push((format!("latn/be.{suffix}"), self.latn_be(body, serif)));
            out.push((format!("bSlash.{suffix}"), self.b_slash(body, serif)));
            out.push((format!("bStroke.{suffix}"), self.b_stroke(body, serif)));
            if !serif.reaches_ascender() {
                out.push((format!("bHookTop.{suffix}"), self.b_hook_top(body, serif)));
            }
        }

        out
    }
}
