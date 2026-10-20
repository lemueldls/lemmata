//! `SerifFrame` from `letter/shared.ptl`.
//!
//! A frame describes the serifs a stem can carry: where its two bars sit, how
//! far a full serif reaches, and how far an outer or inner one does. `lower-p`,
//! `lower-e`, `lower-h` and most lower-case serifed variants read it instead of
//! building the same `HSerif` calls each time.
//!
//! The class carries one thing the port leaves out: `enoughSpaceForFullSerifs`,
//! which reads `para.refJut`, a parameter the metrics layer does not carry yet.
//! The Armenian and Cyrillic definitions that consult it will need it added.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops, clippy::similar_names)]

use lemmata_geom::Geometry;

use crate::letter_model::DivFrame;
use crate::metrics::Metrics;
use crate::shapes::{self, HSerifSide};
use crate::spiro_kit::Kit;
use crate::util::mix;

/// The serif frame a definition builds once and reads many times.
#[derive(Clone, Copy, Debug)]
pub struct SerifFrame {
    pub top: f64,
    pub bot: f64,
    pub left: f64,
    pub right: f64,
    pub sw_ref: f64,
    pub sw_serif: f64,
    pub adws: f64,
    pub h_split: f64,
    /// How far a full serif reaches.
    pub jut: f64,
    /// How far it reaches when the two sides share the space.
    pub jut_in: f64,
    /// How far an outer serif reaches.
    pub side_jut: f64,
    /// How far an inner one does.
    pub side_jut_in: f64,
    /// Whether the space between the bars can carry a full serif.
    pub enough_space_for_full_serifs: bool,
}

impl SerifFrame {
    /// `SerifFrame`.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        metrics: &Metrics,
        top: f64,
        bot: f64,
        left: f64,
        right: f64,
        sw_ref: f64,
        sw_serif: f64,
        adws: f64,
        h_split: f64,
        force_symmetric: bool,
    ) -> Self {
        let ink = sw_ref * metrics.hv_contrast;
        let co_ink = (metrics.stroke - sw_ref) * metrics.hv_contrast;
        let gap = (right - left - h_split * ink) / (h_split - 1.0);
        let jut = (metrics.jut - 0.5 * co_ink).min(mix(
            0.5 * ink,
            metrics.jut,
            (adws * 2.25 / h_split).min(1.0),
        ));
        let side_jut = jut - 0.5 * ink;
        let jut_in = if force_symmetric {
            jut
        } else {
            shapes::jut_in(metrics, left, right, jut, sw_ref, h_split)
        };
        let side_jut_in = jut_in - 0.5 * ink;
        let enough_space_for_full_serifs = 0.5 * ink + 0.375 * gap > metrics.ref_jut;

        Self {
            top,
            bot,
            left,
            right,
            sw_ref,
            sw_serif,
            adws,
            h_split,
            jut,
            jut_in,
            side_jut,
            side_jut_in,
            enough_space_for_full_serifs,
        }
    }

    /// `SerifFrame.fromDf`.
    #[must_use]
    pub fn from_df(
        metrics: &Metrics,
        df: &DivFrame,
        top: f64,
        bot: f64,
        sw_ref: Option<f64>,
        sw_serif: Option<f64>,
        force_symmetric: bool,
    ) -> Self {
        let sw_ref = sw_ref.unwrap_or(df.mvs);

        Self::new(
            metrics,
            top,
            bot,
            df.sb,
            df.width - df.sb,
            sw_ref,
            sw_serif.unwrap_or(sw_ref),
            df.adws(),
            df.h_pack.max(2.0),
            force_symmetric,
        )
    }

    /// `slice`: the frame a run of `hSplit` pieces of `[l, r]` would have.
    #[must_use]
    pub fn slice(&self, metrics: &Metrics, l: f64, r: f64) -> Self {
        let ink = self.sw_ref * metrics.hv_contrast;
        let gap = (self.right - self.left - self.h_split * ink) / (self.h_split - 1.0);
        let l1 = self.left + (ink + gap) * l;
        let r1 = self.left + (ink + gap) * r + ink;

        Self::new(
            metrics,
            self.top,
            self.bot,
            l1,
            r1,
            self.sw_ref,
            self.sw_serif,
            self.adws,
            r - l + 1.0,
            false,
        )
    }

    fn ink(&self, metrics: &Metrics) -> f64 {
        self.sw_ref * metrics.hv_contrast
    }

    fn co_ink(&self, metrics: &Metrics) -> f64 {
        (metrics.stroke - self.sw_ref) * metrics.hv_contrast
    }

    fn l_bar_center(&self, metrics: &Metrics) -> f64 {
        self.left + 0.5 * self.ink(metrics)
    }

    fn l_bar_inner(&self, metrics: &Metrics) -> f64 {
        self.left + self.ink(metrics)
    }

    fn r_bar_center(&self, metrics: &Metrics) -> f64 {
        self.right - 0.5 * self.ink(metrics)
    }

    fn r_bar_inner(&self, metrics: &Metrics) -> f64 {
        self.right - self.ink(metrics)
    }

    fn jut_fs(&self, metrics: &Metrics) -> f64 {
        metrics.mid_jut_side - 0.5 * self.co_ink(metrics)
    }

    fn jut_fc(&self, metrics: &Metrics) -> f64 {
        metrics.mid_jut_center - 0.5 * self.co_ink(metrics)
    }

    fn tagged(tag: &str, geometry: Geometry) -> Geometry {
        let _ = tag;

        Geometry::Tagged(Box::new(geometry))
    }

    /// `lt.full`.
    #[must_use]
    pub fn lt_full(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        Self::tagged(
            "serifLT",
            shapes::hserif_mt_asymmetric(
                kit,
                metrics,
                self.l_bar_center(metrics),
                self.top,
                self.jut,
                self.jut_in,
                Some(self.sw_serif),
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// `lt.fullSide`.
    #[must_use]
    pub fn lt_full_side(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        Self::tagged(
            "serifLT",
            shapes::hserif_mt_asymmetric(
                kit,
                metrics,
                self.l_bar_center(metrics),
                self.top,
                self.jut,
                self.jut_fs(metrics),
                Some(self.sw_serif),
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// `lt.outer`.
    #[must_use]
    pub fn lt_outer(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        Self::tagged(
            "serifLT",
            shapes::hserif(
                kit,
                metrics,
                HSerifSide::LeftTop,
                self.left,
                self.top,
                self.side_jut,
                self.sw_serif,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// `lt.inner`.
    #[must_use]
    pub fn lt_inner(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        Self::tagged(
            "serifLT",
            shapes::hserif(
                kit,
                metrics,
                HSerifSide::RightTop,
                self.l_bar_inner(metrics),
                self.top,
                self.side_jut_in,
                self.sw_serif,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// `rt.full`.
    #[must_use]
    pub fn rt_full(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        Self::tagged(
            "serifRT",
            shapes::hserif_mt_asymmetric(
                kit,
                metrics,
                self.r_bar_center(metrics),
                self.top,
                self.jut_in,
                self.jut,
                Some(self.sw_serif),
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// `rt.fullSide`.
    #[must_use]
    pub fn rt_full_side(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        Self::tagged(
            "serifRT",
            shapes::hserif_mt_asymmetric(
                kit,
                metrics,
                self.r_bar_center(metrics),
                self.top,
                self.jut_fs(metrics),
                self.jut,
                Some(self.sw_serif),
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// `rt.outer`.
    #[must_use]
    pub fn rt_outer(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        Self::tagged(
            "serifRT",
            shapes::hserif(
                kit,
                metrics,
                HSerifSide::RightTop,
                self.right,
                self.top,
                self.side_jut,
                self.sw_serif,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// `rt.inner`.
    #[must_use]
    pub fn rt_inner(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        Self::tagged(
            "serifRT",
            shapes::hserif(
                kit,
                metrics,
                HSerifSide::LeftTop,
                self.r_bar_inner(metrics),
                self.top,
                self.side_jut_in,
                self.sw_serif,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// `lb.full`.
    #[must_use]
    pub fn lb_full(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        Self::tagged(
            "serifLB",
            shapes::hserif_mb_asymmetric(
                kit,
                metrics,
                self.l_bar_center(metrics),
                self.bot,
                self.jut,
                self.jut_in,
                Some(self.sw_serif),
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// `lb.fullSide`.
    #[must_use]
    pub fn lb_full_side(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        Self::tagged(
            "serifLB",
            shapes::hserif_mb_asymmetric(
                kit,
                metrics,
                self.l_bar_center(metrics),
                self.bot,
                self.jut,
                self.jut_fs(metrics),
                Some(self.sw_serif),
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// `lb.outer`.
    #[must_use]
    pub fn lb_outer(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        Self::tagged(
            "serifLB",
            shapes::hserif(
                kit,
                metrics,
                HSerifSide::LeftBottom,
                self.left,
                self.bot,
                self.side_jut,
                self.sw_serif,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// `lb.inner`.
    #[must_use]
    pub fn lb_inner(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        Self::tagged(
            "serifLB",
            shapes::hserif(
                kit,
                metrics,
                HSerifSide::RightBottom,
                self.l_bar_inner(metrics),
                self.bot,
                self.side_jut_in,
                self.sw_serif,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// `rb.full`.
    #[must_use]
    pub fn rb_full(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        Self::tagged(
            "serifRB",
            shapes::hserif_mb_asymmetric(
                kit,
                metrics,
                self.r_bar_center(metrics),
                self.bot,
                self.jut_in,
                self.jut,
                Some(self.sw_serif),
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// `rb.fullSide`.
    #[must_use]
    pub fn rb_full_side(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        Self::tagged(
            "serifRB",
            shapes::hserif_mb_asymmetric(
                kit,
                metrics,
                self.r_bar_center(metrics),
                self.bot,
                self.jut_fs(metrics),
                self.jut,
                Some(self.sw_serif),
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// `rb.outer`.
    #[must_use]
    pub fn rb_outer(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        Self::tagged(
            "serifRB",
            shapes::hserif(
                kit,
                metrics,
                HSerifSide::RightBottom,
                self.right,
                self.bot,
                self.side_jut,
                self.sw_serif,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// `rb.inner`.
    #[must_use]
    pub fn rb_inner(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        Self::tagged(
            "serifRB",
            shapes::hserif(
                kit,
                metrics,
                HSerifSide::LeftBottom,
                self.r_bar_inner(metrics),
                self.bot,
                self.side_jut_in,
                self.sw_serif,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    fn mid_bar_center(&self, metrics: &Metrics) -> f64 {
        mix(self.l_bar_center(metrics), self.r_bar_center(metrics), 0.5)
    }

    /// `mt.full`, `mt.fullAt`, `mt.fullCenter`, `mt.left` and `mt.right`.
    #[must_use]
    pub fn mt_full(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        self.mt_at(
            kit,
            metrics,
            self.mid_bar_center(metrics),
            self.jut_in,
            self.jut_in,
        )
    }

    /// `mt.fullAt x`.
    #[must_use]
    pub fn mt_at(&self, kit: &Kit, metrics: &Metrics, x: f64, l: f64, r: f64) -> Geometry {
        Self::tagged(
            "serifMT",
            shapes::hserif_mt_asymmetric(kit, metrics, x, self.top, l, r, Some(self.sw_serif))
                .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// `mt.fullCenter`.
    #[must_use]
    pub fn mt_full_center(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        let jut = self.jut_fc(metrics);

        self.mt_at(kit, metrics, self.mid_bar_center(metrics), jut, jut)
    }

    /// `mb.full`, `mb.fullAt`, `mb.fullCenter`, `mb.left` and `mb.right`.
    #[must_use]
    pub fn mb_full(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        self.mb_at(
            kit,
            metrics,
            self.mid_bar_center(metrics),
            self.jut_in,
            self.jut_in,
        )
    }

    /// `mb.fullAt x`.
    #[must_use]
    pub fn mb_at(&self, kit: &Kit, metrics: &Metrics, x: f64, l: f64, r: f64) -> Geometry {
        Self::tagged(
            "serifMB",
            shapes::hserif_mb_asymmetric(kit, metrics, x, self.bot, l, r, Some(self.sw_serif))
                .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// `mb.fullCenter`.
    #[must_use]
    pub fn mb_full_center(&self, kit: &Kit, metrics: &Metrics) -> Geometry {
        let jut = self.jut_fc(metrics);

        self.mb_at(kit, metrics, self.mid_bar_center(metrics), jut, jut)
    }
}
