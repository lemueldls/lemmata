//! The per-glyph frame.
//!
//! Ported from `font-kits/letter-model.mjs`. A glyph definition asks for a frame
//! at some division of the em (`DivFrame 1`), and reads its sidebearings, its
//! middle, its stroke width and its arch depths from it. Slicing a frame gives
//! the sub-frames a glyph uses for its parts.
//!
//! The mark set is not ported: it carries anchors for combining marks, which
//! the layout tables use, not the outlines.

#![allow(
    clippy::similar_names,
    clippy::suboptimal_flops,
    clippy::too_many_arguments
)]

use crate::metrics::Metrics;

/// The frame a glyph is built in.
#[derive(Clone, Debug)]
pub struct DivFrame {
    metrics: Metrics,
    pub h_pack: f64,
    pub width: f64,
    pub sb: f64,
    pub mvs: f64,
    pub ox: f64,
    pub shoulder_fine: f64,
    pub arch_depth: f64,
    pub arch_depth_a: f64,
    pub arch_depth_b: f64,
    pub small_arch_depth: f64,
    pub small_arch_depth_a: f64,
    pub small_arch_depth_b: f64,
}

impl DivFrame {
    /// `DivFrame(adws, hPack, sbMul)`.
    #[must_use]
    pub fn new(
        metrics: Metrics,
        adws: Option<f64>,
        h_pack: Option<f64>,
        sb_mul: Option<f64>,
    ) -> Self {
        let adws = adws.unwrap_or(1.0);
        let width = metrics.width * adws;
        let h_pack = (h_pack.unwrap_or(0.0)).max(1.0);
        let weight = crate::metrics::WeightControl::new(
            metrics.stroke,
            metrics.width,
            metrics.sb,
            metrics.hv_contrast,
        );
        let mvs = weight.advice_stroke_in_space(
            width - 2.0 * metrics.sb,
            h_pack,
            metrics.hv_contrast,
            1.0,
        );
        let sb = metrics.sb
            * sb_mul.unwrap_or_else(|| {
                (0.0_f64).max(1.0_f64.min((width / h_pack - mvs) / (2.0 * metrics.sb)))
            });

        Self::with_params(metrics, width, sb, h_pack, mvs, 0.0)
    }

    /// A frame from explicit parameters, which is what `slice` and `rest` build.
    #[must_use]
    pub fn with_params(
        metrics: Metrics,
        width: f64,
        sb: f64,
        h_pack: f64,
        mvs: f64,
        ox: f64,
    ) -> Self {
        let mut frame = Self {
            metrics,
            h_pack,
            width,
            sb,
            mvs,
            ox,
            shoulder_fine: 0.0,
            arch_depth: 0.0,
            arch_depth_a: 0.0,
            arch_depth_b: 0.0,
            small_arch_depth: 0.0,
            small_arch_depth_a: 0.0,
            small_arch_depth_b: 0.0,
        };
        frame.shoulder_fine = frame.metrics.shoulder_fine * (mvs / frame.metrics.stroke);
        frame.arch_depth = frame.arch_depth_of(frame.metrics.arch_depth, None);
        frame.arch_depth_a = frame.arch_depth_a_of(frame.metrics.arch_depth, None);
        frame.arch_depth_b = frame.arch_depth_b_of(frame.metrics.arch_depth, None);
        frame.small_arch_depth = frame.arch_depth_of(frame.metrics.small_arch_depth, None);
        frame.small_arch_depth_a = frame.arch_depth_a_of(frame.metrics.small_arch_depth, None);
        frame.small_arch_depth_b = frame.arch_depth_b_of(frame.metrics.small_arch_depth, None);

        frame
    }

    #[must_use]
    pub const fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    #[must_use]
    pub const fn middle(&self) -> f64 {
        0.5 * self.width
    }

    #[must_use]
    pub const fn left_sb(&self) -> f64 {
        self.sb
    }

    #[must_use]
    pub const fn right_sb(&self) -> f64 {
        self.width - self.sb
    }

    #[must_use]
    pub fn adws(&self) -> f64 {
        self.width / self.metrics.width
    }

    #[must_use]
    pub fn width_no_overshoot(&self) -> f64 {
        self.width - self.ox
    }

    #[must_use]
    pub fn adws_no_overshoot(&self) -> f64 {
        self.width_no_overshoot() / self.metrics.width
    }

    #[must_use]
    pub fn arch_depth_of(&self, d: f64, stroke: Option<f64>) -> f64 {
        (d * self.adws_no_overshoot()).max(1.125 * stroke.unwrap_or(self.mvs))
    }

    #[must_use]
    pub fn arch_depth_a_of(&self, d: f64, stroke: Option<f64>) -> f64 {
        self.metrics.arch_depth_a_of(
            self.arch_depth_of(d, stroke),
            Some(self.width_no_overshoot()),
        )
    }

    #[must_use]
    pub fn arch_depth_b_of(&self, d: f64, stroke: Option<f64>) -> f64 {
        self.metrics.arch_depth_b_of(
            self.arch_depth_of(d, stroke),
            Some(self.width_no_overshoot()),
        )
    }

    /// A sub-frame keeping `keeps` strokes of `hpk`.
    #[must_use]
    pub fn slice(&self, hpk: Option<f64>, keeps: Option<f64>, o: Option<f64>) -> Self {
        let hpk = hpk.unwrap_or(self.h_pack);
        let keeps = keeps.unwrap_or(2.0);

        self.slice_fine(
            hpk,
            keeps,
            (keeps - 1.0) / (hpk - 1.0),
            0.0,
            o.unwrap_or(0.0),
        )
    }

    #[must_use]
    pub fn slice_fine(&self, hpk: f64, keeps: f64, p_gap: f64, extra_gap: f64, o: f64) -> Self {
        let one_stroke = self.metrics.hv_contrast * self.mvs;
        let total_gap = self.right_sb() - self.left_sb() - hpk * one_stroke - 2.0 * o;
        let width = 2.0 * self.sb + 2.0 * o + total_gap * p_gap + extra_gap + one_stroke * keeps;

        Self::with_params(self.metrics.clone(), width, self.sb, keeps, self.mvs, o)
    }

    /// The frame left over after a sub-frame.
    #[must_use]
    pub fn rest(&self, sub: &Self, h_pack: Option<f64>) -> Self {
        Self::with_params(
            self.metrics.clone(),
            self.width - sub.width,
            self.sb,
            h_pack.unwrap_or(self.h_pack - sub.h_pack + 1.0),
            self.mvs,
            self.ox,
        )
    }

    #[must_use]
    pub fn rest_compact(&self, k_kern: f64, sub: &Self, h_pack: Option<f64>) -> Self {
        let kern = k_kern * (self.width - self.right_sb() + sub.left_sb());
        Self::with_params(
            self.metrics.clone(),
            self.width - sub.width + kern,
            self.sb,
            h_pack.unwrap_or(self.h_pack - sub.h_pack + 1.0),
            self.mvs,
            self.ox,
        )
    }

    fn weight(&self) -> crate::metrics::WeightControl {
        crate::metrics::WeightControl::new(
            self.metrics.stroke,
            self.metrics.width,
            self.metrics.sb,
            self.metrics.hv_contrast,
        )
    }

    #[must_use]
    pub fn advice_stroke(&self, c: f64, extra_scalar: f64) -> f64 {
        self.weight().advice_stroke_in_space(
            extra_scalar * self.width - 2.0 * self.metrics.sb,
            c,
            self.metrics.hv_contrast,
            1.0,
        )
    }

    #[must_use]
    pub fn advice_thinner_stroke(&self, c: f64, extra_scalar: f64) -> f64 {
        self.mvs.min(self.advice_stroke(c, extra_scalar))
    }

    #[must_use]
    pub fn advice_stroke2(&self, c: f64, d: f64, h: f64, extra_scalar: f64) -> f64 {
        let weight = self.weight();
        weight
            .advice_stroke_in_space(h, d, 1.0, 1.0)
            .min(weight.advice_stroke_in_space(
                extra_scalar * self.width - 2.0 * self.metrics.sb,
                c,
                self.metrics.hv_contrast,
                1.0,
            ))
    }

    #[must_use]
    pub fn advice_thinner_stroke2(&self, c: f64, d: f64, h: f64, extra_scalar: f64) -> f64 {
        self.mvs.min(self.advice_stroke2(c, d, h, extra_scalar))
    }
}
