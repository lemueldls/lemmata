//! The lower-case `p` from `letter/latin/lower-p.ptl`.
//!
//! The letter is a bowl over a stem that descends. Four bodies differ in how the
//! bowl's top closes and four serif settings build from one `SerifFrame`.
//! `thorn` uses the taller eared body, and the `Stroke` variants add a bar.
//!
//! `pPalatalHook`, `pHookTop` and the derived tick need masks, hooks or glyphs
//! outside this file and are not ported. The anchors are not either.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::serif::SerifFrame;
use lemmata_ptl::shapes::{self, Design, OBarContext, OBarRing};
use lemmata_ptl::spiro_kit::Kit;
use lemmata_ptl::util::{join_camel, mix};

/// The four bodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Eared,
    FlatTop,
    EarlessCorner,
    EarlessRounded,
}

/// The four serif settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serif {
    Serifless,
    MotionSerifed,
    BottomSerifed,
    Serifed,
}

/// The bodies, in the table's order.
pub const BODIES: &[(&str, Body)] = &[
    ("eared", Body::Eared),
    ("flatTop", Body::FlatTop),
    ("earlessCorner", Body::EarlessCorner),
    ("earlessRounded", Body::EarlessRounded),
];

/// The serifs, in the table's order.
pub const SERIFS: &[(&str, Serif)] = &[
    ("serifless", Serif::Serifless),
    ("motionSerifed", Serif::MotionSerifed),
    ("bottomSerifed", Serif::BottomSerifed),
    ("serifed", Serif::Serifed),
];

/// The lower-case `p` under construction.
pub struct LowerP<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerP<'_> {
    const fn left_sb(&self) -> f64 {
        self.df.sb
    }

    fn right_sb(&self) -> f64 {
        self.df.width - self.df.sb
    }

    const fn context(&self) -> OBarContext<'_> {
        OBarContext {
            kit: self.kit,
            metrics: self.metrics,
            design: self.design,
        }
    }

    fn ring(&self, sw: f64) -> OBarRing {
        let metrics = self.metrics;

        OBarRing {
            top: metrics.xh,
            bot: 0.0,
            left: self.left_sb(),
            right: self.right_sb(),
            sw,
            fine: metrics.shoulder_fine * (sw / metrics.stroke),
            ada: metrics.small_arch_depth_a,
            adb: metrics.small_arch_depth_b,
        }
    }

    fn stem(&self, y_up: f64, sw: f64) -> Geometry {
        shapes::vbar_l(
            self.kit,
            self.metrics,
            self.left_sb(),
            self.metrics.descender,
            y_up,
            sw,
        )
        .geometry(kurbo::Affine::IDENTITY)
    }

    /// The bowl and the stem, as the body draws them.
    #[must_use]
    pub fn body(&self, body: Body) -> Vec<Geometry> {
        let metrics = self.metrics;
        let sw = metrics.stroke;
        let ring = self.ring(sw);
        let context = self.context();
        let yend = ring.bot + ring.adb + metrics.tiny;
        let toggle = |geometry: Geometry| Geometry::Tagged(Box::new(geometry));

        match body {
            Body::Eared => vec![
                toggle(shapes::obar_left_shape(&context, &ring).geometry(kurbo::Affine::IDENTITY)),
                toggle(self.stem(metrics.xh, sw)),
            ],
            Body::FlatTop => vec![
                toggle(
                    shapes::obar_left_earless_corner(&context, &ring, 0.0, yend)
                        .geometry(kurbo::Affine::IDENTITY),
                ),
                toggle(self.stem(metrics.xh, sw)),
            ],
            Body::EarlessCorner => vec![
                toggle(
                    shapes::obar_left_earless_corner(
                        &context,
                        &ring,
                        metrics.d_toothless_rise,
                        yend,
                    )
                    .geometry(kurbo::Affine::IDENTITY),
                ),
                toggle(self.stem(metrics.xh - metrics.d_toothless_rise, sw)),
            ],
            Body::EarlessRounded => vec![toggle(
                shapes::obar_left_earless_rounded(&context, &ring, metrics.descender, yend)
                    .geometry(kurbo::Affine::IDENTITY),
            )],
        }
    }

    /// `SmallThornEaredBody`.
    #[must_use]
    pub fn thorn_body(&self) -> Vec<Geometry> {
        let metrics = self.metrics;
        let sw = metrics.stroke;
        let ring = self.ring(sw);
        let toggle = |geometry: Geometry| Geometry::Tagged(Box::new(geometry));

        vec![
            toggle(
                shapes::obar_left_shape(&self.context(), &ring).geometry(kurbo::Affine::IDENTITY),
            ),
            toggle(self.stem(metrics.ascender, sw)),
        ]
    }

    fn frame(&self, top: f64) -> SerifFrame {
        SerifFrame::from_df(
            self.metrics,
            self.df,
            top,
            self.metrics.descender,
            None,
            None,
            false,
        )
    }

    /// `Serifs df top _sw`: the frame's outer top and full-side foot.
    #[must_use]
    pub fn serifs(&self, serif: Serif, top: f64) -> Vec<Geometry> {
        let frame = self.frame(top);
        let mut parts = Vec::new();
        if matches!(serif, Serif::MotionSerifed | Serif::Serifed) {
            parts.push(frame.lt_outer(self.kit, self.metrics));
        }
        if matches!(serif, Serif::BottomSerifed | Serif::Serifed) {
            parts.push(frame.lb_full_side(self.kit, self.metrics));
        }

        parts
    }

    /// `p.<suffix>`.
    #[must_use]
    pub fn glyph(&self, body: Body, serif: Serif) -> Geometry {
        let mut parts = self.body(body);
        parts.extend(self.serifs(serif, self.metrics.xh));

        Geometry::Combine(parts)
    }

    /// `thorn.<suffix>`.
    #[must_use]
    pub fn thorn(&self, serif: Serif) -> Geometry {
        let mut parts = self.thorn_body();
        parts.extend(self.serifs(serif, self.metrics.ascender));

        Geometry::Combine(parts)
    }

    fn bar(&self, bot: f64, top: f64) -> Geometry {
        shapes::letter_bar_overlay_l_in(
            self.kit,
            self.metrics,
            self.left_sb(),
            bot,
            top,
            0.5,
            (0.0, self.df.width),
            self.metrics.stroke,
            self.metrics.overlay_stroke,
        )
        .geometry(kurbo::Affine::IDENTITY)
    }

    /// `pStrokeBottom.<suffix>`.
    #[must_use]
    pub fn stroke_bottom(&self, body: Body, serif: Serif) -> Geometry {
        let gap = if matches!(serif, Serif::BottomSerifed | Serif::Serifed) {
            self.metrics.stroke
        } else {
            0.0
        };

        Geometry::Combine(vec![
            self.glyph(body, serif),
            self.bar(self.metrics.descender + gap, 0.0),
        ])
    }

    /// `thornStroke.<suffix>`.
    #[must_use]
    pub fn thorn_stroke(&self, serif: Serif) -> Geometry {
        let gap = if matches!(serif, Serif::MotionSerifed | Serif::Serifed) {
            self.metrics.stroke
        } else {
            0.0
        };

        Geometry::Combine(vec![
            self.thorn(serif),
            self.bar(self.metrics.xh, self.metrics.ascender - gap),
        ])
    }

    /// `thornStrokeBottom.<suffix>`.
    #[must_use]
    pub fn thorn_stroke_bottom(&self, serif: Serif) -> Geometry {
        let gap = if matches!(serif, Serif::BottomSerifed | Serif::Serifed) {
            self.metrics.stroke
        } else {
            0.0
        };

        Geometry::Combine(vec![
            self.thorn(serif),
            self.bar(self.metrics.descender + gap, 0.0),
        ])
    }

    /// `mathbb/p`.
    #[must_use]
    pub fn mathbb(&self) -> Geometry {
        let metrics = self.metrics;
        let bbs = shapes::bb_stroke(self.df);
        let bbd = shapes::bb_distance(metrics, bbs);
        let left = self.left_sb();
        let right = self.right_sb();
        let mut parts = shapes::bb_bar_left(
            self.kit,
            metrics,
            left,
            metrics.descender,
            metrics.xh,
            bbs,
            bbd,
        );
        let outline = shapes::oshape_outline_no_overshoot(
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
                closed: outline.closed,
                knots: outline.knots,
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
        // `OBarLeft.shape` here overrides only the left edge and the stroke, so
        // the fine width stays the default rather than scaling with `bbs`.
        let ring = OBarRing {
            left: left + bbd,
            sw: bbs,
            fine: metrics.shoulder_fine,
            ..self.ring(bbs)
        };
        parts.push(
            shapes::obar_left_shape(&self.context(), &ring).geometry(kurbo::Affine::IDENTITY),
        );
        let _ = mix;

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
