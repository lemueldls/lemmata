//! The lower-case `m` from `letter/latin/lower-m.ptl`, the arches body.
//!
//! `m` is a left stem and two shoulders drawn as closed spi ro outlines. The
//! file's table has a hundred entries across five bodies, two leg heights, two
//! tails and five serif settings; this module ports the arches body with the
//! serifless and serifed settings.
//!
//! The other bodies, the tails, `meng`, `mCrossedTail` and the turned forms are
//! not ported. The anchors are not either.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops, clippy::similar_names)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::control::Item;
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::serif::SerifFrame;
use lemmata_ptl::shapes::{self, Design};
use lemmata_ptl::spiro_kit::{Kit, SpiroOutline};
use lemmata_ptl::util::mix;

/// The two serif settings ported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serif {
    Serifless,
    Serifed,
}

/// The variants, in the table's order.
pub const VARIANTS: &[(&str, Serif)] =
    &[("serifless", Serif::Serifless), ("serifed", Serif::Serifed)];

/// One shoulder: the parameters its outline reads.
#[derive(Clone, Copy, Debug)]
struct Shoulder {
    left: f64,
    right: f64,
    fine: f64,
    top: f64,
    bottom: f64,
    ada: f64,
    adb: f64,
    stroke: f64,
    left_y0: Option<f64>,
}

/// The lower-case `m` under construction.
pub struct LowerM<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerM<'_> {
    /// The frame the definition draws in: `dfM`, `DivFrame para.advanceScaleMM 3`.
    fn frame(&self) -> DivFrame {
        DivFrame::new(self.metrics.clone(), Some(1.0), Some(3.0), None)
    }

    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    fn middle(&self) -> f64 {
        self.frame().width / 2.0
    }

    fn right_sb(&self) -> f64 {
        let df = self.frame();

        df.width - df.sb
    }

    /// `SmallMSmoothA`.
    fn smooth_a(&self) -> f64 {
        let df = self.frame();

        df.slice(Some(3.0), Some(2.0), None)
            .arch_depth_a_of(self.metrics.small_arch_depth, Some(df.mvs))
    }

    /// `SmallMSmoothB`.
    fn smooth_b(&self) -> f64 {
        let df = self.frame();

        df.slice(Some(3.0), Some(2.0), None)
            .arch_depth_b_of(self.metrics.small_arch_depth, Some(df.mvs))
    }

    /// `SmallMShoulderSpiro`.
    fn shoulder_items(&self, shoulder: &Shoulder) -> Vec<Item> {
        let metrics = self.metrics;
        let stroke = shoulder.stroke;
        let fix = self.hsw_to_v(metrics.tan_slope * stroke);
        let right_inner = shoulder.right - self.hsw_to_v(stroke);
        let left_inner = shoulder.left + self.hsw_to_v(shoulder.fine);
        let left_y0 = |fallback: f64| shoulder.left_y0.unwrap_or(fallback);

        vec![
            self.kit
                .knot(KnotType::Corner, right_inner, shoulder.bottom, Vec::new()),
            self.kit.knot(
                KnotType::Right,
                right_inner,
                (shoulder.top - shoulder.adb + fix).max(shoulder.bottom + metrics.tiny),
                Vec::new(),
            ),
            self.kit.arcvh(Some(8), None),
            self.kit.pair(
                lemmata_ptl::spiro_kit::Family::G2,
                lemmata_ptl::spiro_kit::Direction::Right,
                lemmata_ptl::spiro_kit::Position::Mid,
                mix(right_inner, left_inner, 0.5),
                (shoulder.top - metrics.o) - stroke,
                Vec::new(),
            ),
            self.kit.archv(Some(8), None),
            self.kit.knot(
                KnotType::Left,
                left_inner,
                (left_y0(shoulder.bottom) + metrics.tiny).max(shoulder.top - shoulder.ada - fix),
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Corner,
                left_inner,
                left_y0(
                    shoulder
                        .bottom
                        .max((shoulder.top - shoulder.ada - fix) - metrics.tiny),
                ),
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Corner,
                shoulder.left,
                left_y0(
                    shoulder
                        .bottom
                        .max((shoulder.top - shoulder.ada) - metrics.tiny),
                ),
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Right,
                shoulder.left,
                (left_y0(shoulder.bottom) + metrics.tiny).max(shoulder.top - shoulder.ada),
                Vec::new(),
            ),
            self.kit.arcvh(Some(8), None),
            self.kit.pair(
                lemmata_ptl::spiro_kit::Family::G2,
                lemmata_ptl::spiro_kit::Direction::Right,
                lemmata_ptl::spiro_kit::Position::Mid,
                mix(shoulder.left, shoulder.right, 0.5),
                shoulder.top - metrics.o,
                Vec::new(),
            ),
            self.kit.archv(Some(8), None),
            self.kit.knot(
                KnotType::Left,
                shoulder.right,
                (shoulder.top - shoulder.adb).max(shoulder.bottom + metrics.tiny),
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Corner,
                shoulder.right,
                shoulder.bottom,
                Vec::new(),
            ),
            self.kit.close(),
        ]
    }

    fn shoulder(&self, shoulder: &Shoulder) -> SpiroOutline {
        self.kit.spiro_outline(self.shoulder_items(shoulder))
    }

    /// `SmallMArches`: the left stem and the two shoulders.
    #[must_use]
    pub fn arches(&self, top: f64, lbot: f64, mbot: f64, rbot: f64) -> Vec<Geometry> {
        let metrics = self.metrics;
        let df = self.frame();
        let sw = df.mvs;
        let fine = sw * (metrics.shoulder_fine / metrics.stroke);
        let stem = shapes::vbar_l(self.kit, metrics, df.sb, lbot, top, sw)
            .geometry(kurbo::Affine::IDENTITY);
        let left_shoulder = self.shoulder(&Shoulder {
            left: df.sb + self.hsw_to_v(sw - fine),
            right: self.middle() + self.hsw_to_v(0.5 * sw),
            fine,
            top,
            bottom: mbot,
            ada: self.smooth_a(),
            adb: self.smooth_b(),
            stroke: sw,
            left_y0: None,
        });
        let right_shoulder = self.shoulder(&Shoulder {
            left: self.middle() + self.hsw_to_v(0.5 * sw - fine),
            right: self.right_sb(),
            fine,
            top,
            bottom: rbot,
            ada: self.smooth_a(),
            adb: self.smooth_b(),
            stroke: sw,
            left_y0: Some(mbot),
        });

        vec![
            stem,
            Geometry::Tagged(Box::new(Geometry::Spiro {
                gizmo: kurbo::Affine::IDENTITY,
                closed: left_shoulder.closed,
                knots: left_shoulder.knots,
            })),
            Geometry::Spiro {
                gizmo: kurbo::Affine::IDENTITY,
                closed: right_shoulder.closed,
                knots: right_shoulder.knots,
            },
        ]
    }

    /// `MEnoughSpaceForFullSerifs`.
    fn enough_space_for_full_serifs(&self) -> bool {
        let df = self.frame();
        let ink = self.hsw_to_v(df.mvs);
        let gap = (self.right_sb() - df.sb) / 2.0 - 1.5 * ink;

        0.5 * ink + 0.375 * gap > self.metrics.ref_jut
    }

    fn serif_frame(&self, top: f64, bot: f64) -> SerifFrame {
        SerifFrame::from_df(self.metrics, &self.frame(), top, bot, None, None, false)
    }

    /// `FullSerifs` for the plain body.
    fn full_serifs(&self, top: f64, lbot: f64, mbot: f64, rbot: f64) -> Vec<Geometry> {
        let metrics = self.metrics;
        let full = self.enough_space_for_full_serifs();
        let mut parts = vec![self.serif_frame(top, 0.0).lt_outer(self.kit, metrics)];
        parts.push(if full {
            self.serif_frame(top, lbot).lb_full(self.kit, metrics)
        } else {
            self.serif_frame(top, lbot).lb_outer(self.kit, metrics)
        });
        // `SmallMBottomMiddleSerif` draws only when the space carries a full
        // serif; otherwise the middle bar has none.
        if full {
            parts.push(self.serif_frame(top, mbot).mb_full(self.kit, metrics));
        }
        parts.push(if full {
            self.serif_frame(top, rbot).rb_full(self.kit, metrics)
        } else {
            self.serif_frame(top, rbot).rb_outer(self.kit, metrics)
        });

        parts
    }

    /// `m.<suffix>`.
    #[must_use]
    pub fn glyph(&self, serif: Serif) -> Geometry {
        let metrics = self.metrics;
        let top = metrics.xh;
        let mut parts = self.arches(top, 0.0, 0.0, 0.0);
        if serif == Serif::Serifed {
            parts.extend(self.full_serifs(top, 0.0, 0.0, 0.0));
        }

        Geometry::Combine(parts)
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for (suffix, serif) in VARIANTS {
            out.push((format!("m.{suffix}"), self.glyph(*serif)));
        }

        out
    }
}
