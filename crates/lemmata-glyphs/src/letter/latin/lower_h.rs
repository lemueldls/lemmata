//! The lower-case `h` from `letter/latin/lower-h.ptl`.
//!
//! A stem and a shoulder, with a tail or not and four serif settings. `hStroke`
//! adds a bar, `hHookTop` turns the stem's top into a hook, `heng` adds the eng
//! hook and `mathbb/h` is the blackboard form.
//!
//! The derived descenders, palatal hooks, dots and cedillas need glyphs outside
//! this file and are not ported. The frame is `DivFrame 1 2` for `mathbb/h` and
//! the default for the rest.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::{
    control::Item,
    letter_model::DivFrame,
    metrics::Metrics,
    shapes::{self, Design, HSerifSide},
    shoulder::NShoulder,
    spiro_kit::Kit,
    util::{join_camel, mix},
};

/// Whether the shoulder has a tail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tail {
    Straight,
    Tailed,
}

/// The four serif settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serif {
    Serifless,
    Serifed,
    MotionSerifed,
    TopLeftSerifed,
}

impl Serif {
    /// Which serif function the setting names.
    const fn index(self) -> usize {
        match self {
            Self::Serifless => 0,
            Self::Serifed => 1,
            Self::MotionSerifed => 2,
            Self::TopLeftSerifed => 3,
        }
    }

    /// Whether the setting carries a top-left serif.
    const fn has_top(self) -> bool {
        !matches!(self, Self::Serifless)
    }
}

/// The tails, in the table's order.
pub const TAILS: &[(&str, Tail)] = &[("straight", Tail::Straight), ("tailed", Tail::Tailed)];

/// The serifs, in the table's order.
pub const SERIFS: &[(&str, Serif)] = &[
    ("serifless", Serif::Serifless),
    ("serifed", Serif::Serifed),
    ("motionSerifed", Serif::MotionSerifed),
    ("topLeftSerifed", Serif::TopLeftSerifed),
];

/// The lower-case `h` under construction.
pub struct LowerH<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerH<'_> {
    const fn left_sb(&self) -> f64 {
        self.df.sb
    }

    const fn right_sb(&self) -> f64 {
        self.df.width - self.df.sb
    }

    const fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    fn shoulder(&self, tail: Tail, top: f64, fine: f64) -> NShoulder {
        let metrics = self.metrics;

        NShoulder {
            left: self.left_sb() + self.hsw_to_v(metrics.stroke),
            right: self.right_sb(),
            fine,
            top,
            bottom: if tail == Tail::Tailed {
                metrics.xh - metrics.small_arch_depth_b + metrics.o
            } else {
                0.0
            },
            ada: metrics.small_arch_depth_a,
            adb: metrics.small_arch_depth_b,
            stroke: metrics.stroke,
            left_y0: metrics.xh - metrics.small_arch_depth_a - metrics.tiny,
        }
    }

    fn stem(&self, top: f64) -> Geometry {
        shapes::vbar_l(
            self.kit,
            self.metrics,
            self.left_sb(),
            0.0,
            top,
            self.metrics.stroke,
        )
        .geometry(kurbo::Affine::IDENTITY)
    }

    fn hook_top(&self) -> Vec<Geometry> {
        shapes::top_hook_l_bar_inner(
            self.kit,
            self.metrics,
            self.left_sb(),
            0.0,
            self.metrics.ascender,
            self.metrics.stroke,
            1.0,
        )
    }

    /// The serif function the table names.
    #[must_use]
    pub fn serifs(&self, tail: Tail, serif: Serif, hook_top: bool) -> Vec<Geometry> {
        let metrics = self.metrics;
        if serif == Serif::Serifless {
            return Vec::new();
        }
        let mut parts = Vec::new();
        if !hook_top {
            parts.push(Geometry::Tagged(Box::new(
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
            )));
        }
        match (serif.index(), tail) {
            (0, _) => {}
            (1, Tail::Straight) => {
                parts.push(Geometry::Tagged(Box::new(
                    shapes::hserif_mb(
                        self.kit,
                        metrics,
                        self.left_sb() + self.hsw_to_v(metrics.half_stroke),
                        0.0,
                        metrics.jut,
                        Some(metrics.stroke),
                    )
                    .geometry(kurbo::Affine::IDENTITY),
                )));
                parts.push(Geometry::Tagged(Box::new(
                    shapes::hserif_mb(
                        self.kit,
                        metrics,
                        self.right_sb() - self.hsw_to_v(metrics.half_stroke),
                        0.0,
                        metrics.jut,
                        Some(metrics.stroke),
                    )
                    .geometry(kurbo::Affine::IDENTITY),
                )));
            }
            (1, Tail::Tailed) => {
                parts.push(Geometry::Tagged(Box::new(
                    shapes::hserif_mb(
                        self.kit,
                        metrics,
                        self.left_sb() + self.hsw_to_v(metrics.half_stroke),
                        0.0,
                        metrics.jut,
                        Some(metrics.stroke),
                    )
                    .geometry(kurbo::Affine::IDENTITY),
                )));
            }
            (2, _) => {
                if tail == Tail::Straight {
                    parts.push(Geometry::Tagged(Box::new(
                        shapes::hserif(
                            self.kit,
                            metrics,
                            HSerifSide::RightBottom,
                            self.right_sb(),
                            0.0,
                            metrics.side_jut,
                            metrics.stroke,
                            None,
                        )
                        .geometry(kurbo::Affine::IDENTITY),
                    )));
                }
            }
            (3, _) => {}
            _ => {}
        }

        parts
    }

    fn shoulder_geometry(&self, tail: Tail, top: f64, fine: f64) -> Geometry {
        Geometry::Tagged(Box::new(
            self.shoulder(tail, top, fine)
                .shape(self.kit, self.metrics, self.design)
                .geometry(kurbo::Affine::IDENTITY),
        ))
    }

    fn tail_bar(&self, tail: Tail) -> Option<Geometry> {
        if tail == Tail::Tailed {
            Some(
                shapes::rightward_tailed_bar(
                    self.kit,
                    self.metrics,
                    self.df,
                    self.right_sb(),
                    0.0,
                    self.metrics.xh - self.metrics.small_arch_depth_b,
                    self.metrics.stroke,
                )
                .geometry(kurbo::Affine::IDENTITY),
            )
        } else {
            None
        }
    }

    /// `h.<suffix>`.
    #[must_use]
    pub fn glyph(&self, tail: Tail, serif: Serif) -> Geometry {
        let metrics = self.metrics;
        let mut parts = vec![
            self.stem(metrics.ascender),
            self.shoulder_geometry(tail, metrics.xh, metrics.shoulder_fine),
        ];
        parts.extend(self.tail_bar(tail));
        parts.extend(self.serifs(tail, serif, false));

        Geometry::Combine(parts)
    }

    /// `hHookTop.<suffix>`.
    #[must_use]
    pub fn hook_top_glyph(&self, tail: Tail, serif: Serif) -> Geometry {
        let metrics = self.metrics;
        let mut parts = self.hook_top();
        parts.push(self.shoulder_geometry(tail, metrics.xh, metrics.shoulder_fine));
        parts.extend(self.tail_bar(tail));
        parts.extend(self.serifs(tail, serif, true));

        Geometry::Combine(parts)
    }

    /// `heng.<suffix>`.
    #[must_use]
    pub fn eng(&self, serif: Serif) -> Geometry {
        let metrics = self.metrics;
        let mut parts = vec![
            self.stem(metrics.ascender),
            self.shoulder_geometry(Tail::Straight, metrics.xh, metrics.shoulder_fine),
            shapes::eng_hook(
                self.kit,
                metrics,
                self.right_sb(),
                0.0,
                metrics.descender,
                metrics.stroke,
            )
            .geometry(kurbo::Affine::IDENTITY),
        ];
        parts.extend(self.serifs(Tail::Tailed, serif, false));

        Geometry::Combine(parts)
    }

    /// `hengHookTop.<suffix>`.
    #[must_use]
    pub fn eng_hook_top(&self, serif: Serif) -> Geometry {
        let metrics = self.metrics;
        let mut parts = self.hook_top();
        parts.push(self.shoulder_geometry(Tail::Straight, metrics.xh, metrics.shoulder_fine));
        parts.extend(self.serifs(Tail::Tailed, serif, true));
        parts.push(
            shapes::eng_hook(
                self.kit,
                metrics,
                self.right_sb(),
                0.0,
                metrics.descender,
                metrics.stroke,
            )
            .geometry(kurbo::Affine::IDENTITY),
        );

        Geometry::Combine(parts)
    }

    /// `HBar.mOverlay`: the bar a `Stroke` variant draws.
    fn bar_overlay(&self, has_top_serif: bool) -> Vec<Geometry> {
        let metrics = self.metrics;
        let bar_space_top = metrics.ascender - if has_top_serif { metrics.stroke } else { 0.0 };
        let s = metrics
            .overlay_stroke
            .min(0.625 * (bar_space_top - metrics.xh));
        let x_start = mix(self.left_sb(), 0.0, 0.7);
        let x_end = mix(self.left_sb(), self.right_sb(), 0.5);
        let y = mix(metrics.xh, bar_space_top, 0.45) - s / 2.0;
        let x_mid = self.left_sb() + self.hsw_to_v(metrics.half_stroke);

        vec![
            self.kit
                .dispiro(vec![
                    Item::Af(self.kit.widths_lhs(Some(s))),
                    self.kit
                        .knot(lemmata_geom::KnotType::Corner, x_start, y, Vec::new()),
                    self.kit
                        .knot(lemmata_geom::KnotType::Corner, x_mid, y, Vec::new()),
                ])
                .geometry(kurbo::Affine::IDENTITY),
            self.kit
                .dispiro(vec![
                    Item::Af(self.kit.widths_lhs(Some(metrics.overlay_stroke))),
                    self.kit
                        .knot(lemmata_geom::KnotType::Corner, x_mid, y, Vec::new()),
                    self.kit
                        .knot(lemmata_geom::KnotType::Corner, x_end, y, Vec::new()),
                ])
                .geometry(kurbo::Affine::IDENTITY),
        ]
    }

    /// `hStroke.<suffix>`.
    #[must_use]
    pub fn stroke_glyph(&self, tail: Tail, serif: Serif) -> Geometry {
        let mut parts = vec![self.glyph(tail, serif)];
        parts.extend(self.bar_overlay(serif.has_top()));

        Geometry::Combine(parts)
    }

    /// `hengStroke.<suffix>`.
    #[must_use]
    pub fn eng_stroke(&self, serif: Serif) -> Geometry {
        let mut parts = vec![self.eng(serif)];
        parts.extend(self.bar_overlay(serif.has_top()));

        Geometry::Combine(parts)
    }

    /// `mathbb/h`.
    #[must_use]
    pub fn mathbb(&self) -> Geometry {
        let metrics = self.metrics;
        let bbs = shapes::bb_stroke(self.df);
        let bbd = shapes::bb_distance(metrics, bbs);
        let left = self.left_sb();
        let right = self.right_sb();
        let mut parts =
            shapes::bb_bar_left(self.kit, metrics, left, 0.0, metrics.ascender, bbs, bbd);
        let shoulder = NShoulder {
            left: left + bbd + self.hsw_to_v(bbs),
            right,
            fine: metrics.shoulder_fine,
            top: metrics.xh,
            bottom: 0.0,
            ada: metrics.small_arch_depth_a,
            adb: metrics.small_arch_depth_b,
            stroke: bbs,
            left_y0: metrics.xh - metrics.small_arch_depth_a - metrics.tiny,
        };
        let mask = NShoulder {
            left: left + bbd + self.hsw_to_v(bbs) + 1.0,
            right: right - 1.0,
            ..shoulder
        };
        parts.push(
            shapes::hbar_b(self.kit, metrics, right - bbd, right, 0.0, Some(bbs))
                .geometry(kurbo::Affine::IDENTITY),
        );
        parts.push(
            shoulder
                .shape(self.kit, metrics, self.design)
                .geometry(kurbo::Affine::IDENTITY),
        );
        let mask_shape = mask.mask(self.kit, metrics, self.design);
        parts.push(lemmata_ptl::glyph::intersection(vec![
            shapes::vbar_r(self.kit, metrics, right - bbd, 0.0, metrics.xh, bbs)
                .geometry(kurbo::Affine::IDENTITY),
            Geometry::Spiro {
                gizmo: kurbo::Affine::IDENTITY,
                closed: mask_shape.closed,
                knots: mask_shape.knots,
            },
        ]));

        Geometry::Combine(parts)
    }
}

/// The variant name `SuffixCfg.weave` gives a tail and a serif.
#[must_use]
pub fn suffix(tail: &str, serif: &str) -> String {
    join_camel(tail, serif)
}

/// Every variant the table names, as `(suffix, tail, serif)`.
#[must_use]
pub fn variants() -> Vec<(String, Tail, Serif)> {
    let mut out = Vec::with_capacity(TAILS.len() * SERIFS.len());
    for (tail_key, tail) in TAILS {
        for (serif_key, serif) in SERIFS {
            out.push((suffix(tail_key, serif_key), *tail, *serif));
        }
    }

    out
}

impl LowerH<'_> {
    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = vec![("mathbb/h".to_string(), self.mathbb())];
        for (suffix, tail, serif) in variants() {
            out.push((format!("h.{suffix}"), self.glyph(tail, serif)));
            out.push((format!("hStroke.{suffix}"), self.stroke_glyph(tail, serif)));
            out.push((
                format!("hHookTop.{suffix}"),
                self.hook_top_glyph(tail, serif),
            ));
        }
        for (suffix, serif) in SERIFS {
            out.push((format!("heng.{suffix}"), self.eng(*serif)));
            out.push((format!("hengStroke.{suffix}"), self.eng_stroke(*serif)));
            out.push((format!("hengHookTop.{suffix}"), self.eng_hook_top(*serif)));
        }

        out
    }
}
