//! The percent signs from `symbol/punctuation/percentages.ptl`.
//!
//! Ported: `percent.dots`, `percent.ringsContinuousSlash` and
//! `percent.ringsSegmentedSlash`. The permille, base point, commercial minus
//! and the connected-bar variants are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    control::Item,
    letter_model::DivFrame,
    metrics::Metrics,
    shapes::{self, Design, downward, upward},
    spiro_kit::Kit,
    util::{Waveform, mix},
};

/// The percent sign families this module builds.
pub struct Percentages<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl Percentages<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    fn df(&self) -> DivFrame {
        DivFrame::new(self.metrics.clone(), Some(1.0), None, None)
    }

    /// `PercentBarCor`.
    fn bar_cor(&self, df: &DivFrame, sw: f64) -> f64 {
        let a = Waveform::Sawtooth.eval((df.right_sb() - df.left_sb() - sw) / self.metrics.cap);

        1.0 / (1.0 - a * a).sqrt()
    }

    /// `PercentBarShape`.
    fn bar_shape(&self, df: &DivFrame, sw: f64) -> Geometry {
        let cor = self.bar_cor(df, sw);
        let half = self.hsw_to_v(cor * sw);

        spiro_geometry(self.kit.spiro_outline(vec![
            self.kit
                .knot(KnotType::Corner, df.left_sb(), 0.0, Vec::new()),
            self.kit.knot(
                KnotType::Corner,
                df.right_sb() - half,
                self.metrics.cap,
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Corner,
                df.right_sb(),
                self.metrics.cap,
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Corner,
                df.left_sb() + half,
                0.0,
                Vec::new(),
            ),
        ]))
    }

    /// `percent.dots`.
    fn dots(&self) -> Geometry {
        let metrics = self.metrics;
        let df = self.df();
        let width = metrics.advice_stroke(4.0, 1.0, 1.0) * 1.5;

        Geometry::Combine(vec![
            self.bar_shape(&df, metrics.stroke),
            shapes::vbar_l(
                self.kit,
                metrics,
                df.left_sb(),
                mix(metrics.cap, 0.0, 0.3),
                metrics.cap,
                width,
            )
            .geometry(kurbo::Affine::IDENTITY),
            shapes::vbar_r(
                self.kit,
                metrics,
                df.right_sb(),
                0.0,
                mix(0.0, metrics.cap, 0.3),
                width,
            )
            .geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// `percent.ringsContinuousSlash`.
    fn rings_continuous(&self) -> Geometry {
        let metrics = self.metrics;
        let l = metrics.sb / 2.0;
        let r = metrics.width - l;
        let m = mix(l, r, 0.5);
        let ada = metrics.arch_depth_a_of(metrics.arch_depth, None) * 0.5;
        let adb = metrics.arch_depth_b_of(metrics.arch_depth, None) * 0.5;
        let sw_ring = metrics.advice_stroke(3.75, 1.0, 1.0);
        let sw_bar = metrics.advice_stroke(3.75, 1.0, 1.0);
        let fine_bar = metrics.advice_stroke(4.5, 1.0, 1.0);
        let half_gap = metrics.advice_stroke(6.0, 1.0, 1.0) / 2.0;
        let cor = diag_cor_ds(metrics.cap, r - l, sw_bar);

        Geometry::Combine(vec![
            shapes::oshape(
                self.kit,
                metrics,
                self.design,
                metrics.cap,
                metrics.cap / 2.0 + half_gap,
                l,
                m - half_gap,
                Some(sw_ring),
                Some(ada),
                Some(adb),
            )
            .geometry(kurbo::Affine::IDENTITY),
            shapes::oshape(
                self.kit,
                metrics,
                self.design,
                metrics.cap / 2.0 - half_gap,
                0.0,
                m + half_gap,
                r,
                Some(sw_ring),
                Some(ada),
                Some(adb),
            )
            .geometry(kurbo::Affine::IDENTITY),
            self.kit
                .dispiro(vec![
                    self.kit.knot(
                        KnotType::Corner,
                        r,
                        metrics.cap,
                        self.kit
                            .widths_rhs_heading(Some(sw_bar * cor), downward(metrics.hv_contrast)),
                    ),
                    self.kit.knot(
                        KnotType::Corner,
                        mix(r, l, 0.5),
                        mix(metrics.cap, 0.0, 0.5),
                        self.kit.widths_center_heading(
                            Some(fine_bar * cor),
                            downward(metrics.hv_contrast),
                        ),
                    ),
                    self.kit.knot(
                        KnotType::Corner,
                        l,
                        0.0,
                        self.kit
                            .widths_lhs_heading(Some(sw_bar * cor), downward(metrics.hv_contrast)),
                    ),
                ])
                .geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// `percent.ringsSegmentedSlash`.
    fn rings_segmented(&self) -> Geometry {
        let metrics = self.metrics;
        let l = metrics.sb / 2.0;
        let r = metrics.width - l;
        let m = mix(l, r, 0.5);
        let ada = metrics.arch_depth_a_of(metrics.arch_depth, None) * 0.5;
        let adb = metrics.arch_depth_b_of(metrics.arch_depth, None) * 0.5;
        let sw_ring = metrics.advice_stroke(3.5, 1.0, 1.0);
        let sw_bar = metrics.advice_stroke(3.0, 1.0, 1.0);
        let fine = metrics.advice_stroke(4.0, 1.0, 1.0);
        let cor = diag_cor_ds(metrics.cap, r - l, sw_bar);
        let p_fine = 0.5 - (metrics.cap / 10.0).max(metrics.half_stroke) / metrics.cap.hypot(r - l);

        Geometry::Combine(vec![
            shapes::oshape(
                self.kit,
                metrics,
                self.design,
                metrics.cap,
                metrics.cap / 2.0,
                l,
                m,
                Some(sw_ring),
                Some(ada),
                Some(adb),
            )
            .geometry(kurbo::Affine::IDENTITY),
            shapes::oshape(
                self.kit,
                metrics,
                self.design,
                metrics.cap / 2.0,
                0.0,
                m,
                r,
                Some(sw_ring),
                Some(ada),
                Some(adb),
            )
            .geometry(kurbo::Affine::IDENTITY),
            self.kit
                .dispiro(vec![
                    Item::Af(self.kit.widths_rhs(Some(sw_bar * cor))),
                    self.kit.knot(
                        KnotType::Left,
                        r,
                        metrics.cap,
                        self.kit.heading(downward(metrics.hv_contrast)),
                    ),
                    self.kit.knot(
                        KnotType::Right,
                        mix(r, l, p_fine),
                        mix(metrics.cap, 0.0, p_fine),
                        self.kit
                            .widths_heading(fine * cor, 0.0, downward(metrics.hv_contrast)),
                    ),
                ])
                .geometry(kurbo::Affine::IDENTITY),
            self.kit
                .dispiro(vec![
                    Item::Af(self.kit.widths_rhs(Some(sw_bar * cor))),
                    self.kit.knot(
                        KnotType::Left,
                        l,
                        0.0,
                        self.kit.heading(upward(metrics.hv_contrast)),
                    ),
                    self.kit.knot(
                        KnotType::Right,
                        mix(l, r, p_fine),
                        mix(0.0, metrics.cap, p_fine),
                        self.kit
                            .widths_heading(fine * cor, 0.0, upward(metrics.hv_contrast)),
                    ),
                ])
                .geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        vec![
            ("percent.dots".to_string(), self.dots()),
            (
                "percent.ringsContinuousSlash".to_string(),
                self.rings_continuous(),
            ),
            (
                "percent.ringsSegmentedSlash".to_string(),
                self.rings_segmented(),
            ),
        ]
    }
}

/// `DiagCorDs`: the corner factor after the stroke's inset.
fn diag_cor_ds(dy: f64, dx: f64, ds: f64) -> f64 {
    let mut k = shapes::diag_cor(dy, dx, Some(0.0), Some(0.0));
    for _ in 0..4 {
        k = shapes::diag_cor(dy, dx - ds * k, Some(0.0), Some(0.0));
    }

    k
}

fn spiro_geometry(outline: lemmata_ptl::spiro_kit::SpiroOutline) -> Geometry {
    Geometry::Spiro {
        gizmo: kurbo::Affine::IDENTITY,
        closed: outline.closed,
        knots: outline.knots,
    }
}
