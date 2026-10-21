//! The capital `Y` from `letter/latin/upper-y.ptl`.
//!
//! The eight configs: a curly or straight strand body, and none, base, motion
//! or all serifs. `grek/Upsilon`, `smcpY`, the lower-case `y` forms, the
//! hooked variants and the blackboard form are not ported yet.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops, clippy::similar_names)]

use lemmata_geom::Geometry;
use lemmata_ptl::glyph;
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::serif::SerifFrame;
use lemmata_ptl::shapes::{self, Design};
use lemmata_ptl::spiro_kit::Kit;
use lemmata_ptl::util::mix;

use super::x::{HalfXStrandArgs, half_x_strand};

/// The strand body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyType {
    Curly,
    Straight,
}

/// The serif setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlabType {
    None,
    All,
    Base,
    Motion,
}

/// The `Y` families this module builds.
pub struct UpperY<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl UpperY<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    /// `YCrossPos`.
    const fn cross_pos(top: f64, bot: f64) -> f64 {
        mix(bot, top, 0.4)
    }

    fn df(&self) -> DivFrame {
        DivFrame::new(self.metrics.clone(), Some(1.0), None, None)
    }

    /// `WithSerifOverflowMask`: cut the strand to the body box, widened by the
    /// serif's overflow, and split at the middle so the two halves keep their
    /// own shift.
    fn with_serif_overflow_mask(
        &self,
        enabled: bool,
        top: f64,
        bot: f64,
        geom: Geometry,
    ) -> Geometry {
        if !enabled {
            return geom;
        }
        let metrics = self.metrics;
        let df = self.df();
        let left_center = df.left_sb() + self.hsw_to_v(metrics.half_stroke);
        let right_center = df.right_sb() - self.hsw_to_v(metrics.half_stroke);
        let mid = mix(top, bot, 0.5);
        let shift = metrics.serif_shift_s;
        let mask = glyph::union(vec![
            spiro_geometry(shapes::rect(
                self.kit,
                top,
                mid,
                left_center - metrics.jut,
                right_center + metrics.jut - shift,
            )),
            spiro_geometry(shapes::rect(
                self.kit,
                mid,
                bot,
                left_center - metrics.jut + shift,
                right_center + metrics.jut,
            )),
        ]);

        glyph::intersection(vec![geom, mask])
    }

    /// `YShape`: the two strands and the stem.
    fn shape(&self, body: BodyType, slab: SlabType, top: f64, bot: f64, cross: f64) -> Geometry {
        let metrics = self.metrics;
        let straight = body == BodyType::Straight;
        let mask_lt = matches!(slab, SlabType::All | SlabType::Motion);
        let mask_rt = slab == SlabType::All;
        let mut args = HalfXStrandArgs {
            straight,
            slab: metrics.slab != 0.0,
            x_outer: metrics.sb,
            y_outer: top,
            x_center: metrics.middle,
            y_center: cross,
            turn: 0.1,
            p_straight: 0.4,
            tension: 0.28,
            sw: None,
            p_conn: 0.0,
        };
        let left = half_x_strand(self.kit, metrics, &args);
        args.x_outer = metrics.width - metrics.sb;
        let right = half_x_strand(self.kit, metrics, &args);
        let stem = shapes::vbar_m(
            self.kit,
            metrics,
            metrics.middle,
            bot,
            cross + metrics.half_stroke,
            metrics.stroke,
        )
        .geometry(kurbo::Affine::IDENTITY);

        Geometry::Combine(vec![
            Geometry::Tagged(Box::new(
                self.with_serif_overflow_mask(mask_lt, top, bot, left),
            )),
            Geometry::Tagged(Box::new(
                self.with_serif_overflow_mask(mask_rt, top, bot, right),
            )),
            Geometry::Tagged(Box::new(stem)),
        ])
    }

    /// `YSlabs`.
    fn slabs(&self, slab: SlabType, top: f64, bot: f64) -> Vec<Geometry> {
        let metrics = self.metrics;
        let df = self.df();
        let sf = SerifFrame::from_df(metrics, &df, top, 0.0, None, None, false);
        let middle_serif = || {
            shapes::hserif_mb(
                self.kit,
                metrics,
                df.middle(),
                bot,
                metrics.mid_jut_center,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY)
        };

        match slab {
            SlabType::All => vec![
                sf.lt_full(self.kit, metrics),
                sf.rt_full(self.kit, metrics),
                middle_serif(),
            ],
            SlabType::Motion => vec![sf.lt_outer(self.kit, metrics)],
            SlabType::Base => vec![middle_serif()],
            SlabType::None => Vec::new(),
        }
    }

    /// One `Y.<suffix>`.
    #[must_use]
    pub fn glyph(&self, body: BodyType, slab: SlabType) -> Geometry {
        let top = self.metrics.cap;
        let cross = Self::cross_pos(top, 0.0);
        let mut parts = vec![self.shape(body, slab, top, 0.0, cross)];
        parts.extend(self.slabs(slab, top, 0.0));

        Geometry::Combine(parts)
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for (suffix, body, slab) in CONFIGS {
            out.push((format!("Y.{suffix}"), self.glyph(*body, *slab)));
        }

        out
    }
}

/// A spiro outline as a geometry node.
fn spiro_geometry(outline: lemmata_ptl::spiro_kit::SpiroOutline) -> Geometry {
    Geometry::Spiro {
        gizmo: kurbo::Affine::IDENTITY,
        closed: outline.closed,
        knots: outline.knots,
    }
}

/// The config table, in the order `Object.entries` walks it.
pub const CONFIGS: &[(&str, BodyType, SlabType)] = &[
    ("straightSerifless", BodyType::Straight, SlabType::None),
    ("curlySerifless", BodyType::Curly, SlabType::None),
    ("straightBaseSerifed", BodyType::Straight, SlabType::Base),
    ("curlyBaseSerifed", BodyType::Curly, SlabType::Base),
    (
        "straightMotionSerifed",
        BodyType::Straight,
        SlabType::Motion,
    ),
    ("curlyMotionSerifed", BodyType::Curly, SlabType::Motion),
    ("straightSerifed", BodyType::Straight, SlabType::All),
    ("curlySerifed", BodyType::Curly, SlabType::All),
];
