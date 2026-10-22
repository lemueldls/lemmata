//! The lowercase w from `letter/latin/w.ptl`.
//!
//! Ported: the straight serifless form, which the default sans plan selects.
//! The curly, asymmetric, double-V, vertical-sided, rounded and cursive bodies
//! and their serifs are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    control::Item,
    letter_model::DivFrame,
    metrics::Metrics,
    shapes::{self, Design},
    spiro_kit::Kit,
    util::mix,
};

/// The lowercase w forms this module builds.
pub struct LowerW<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerW<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    /// `DiagCorDs`.
    fn diag_cor_ds(&self, dy: f64, dx: f64, ds: f64) -> f64 {
        let mut k = shapes::diag_cor(dy, dx, None, None);
        for _ in 0..4 {
            k = shapes::diag_cor(dy, dx - ds * k, None, None);
        }

        k
    }

    /// `WShapeImpl` for the straight serifless body.
    fn straight_serifless(&self) -> Geometry {
        let metrics = self.metrics;
        let df = DivFrame::new(metrics.clone(), Some(1.0), Some(3.0), None);
        let top = metrics.xh;
        let stroke_outer = metrics.advice_stroke(2.5, 1.0, 1.0);
        let fine_outer = metrics.advice_stroke(5.5, 1.0, 1.0);
        let w_co = metrics.advice_stroke(3.5, 1.0, 1.0);
        let bot_mix = self.hsw_to_v(0.5 * w_co);
        let x_bot1 = metrics.o + mix(df.left_sb() + bot_mix, df.right_sb() - bot_mix, 0.235);
        let x_bot2 = df.width - x_bot1;
        let w_mid = top * 0.72;
        let x_right1 = df.middle();
        let x_left2 = df.width - x_right1;
        let cor_w_co = self.diag_cor_ds(w_mid, x_right1 - x_bot1, w_co);
        let w_co_cr = cor_w_co * w_co;
        let cor_f_outer = self.diag_cor_ds(top, x_bot1 - df.left_sb(), fine_outer);
        let fine_outer_cr = (metrics.c_thin * w_co_cr).min(cor_f_outer * fine_outer);
        let stroke_outer_cr = cor_f_outer * stroke_outer;
        let fine_inner_cr_top = metrics.c_thin * w_co_cr;
        let shift_t = self.hsw_to_v(0.5 * w_co_cr - 0.5 * fine_inner_cr_top);
        let shift_b = self.hsw_to_v(0.5 * w_co_cr - 0.5 * fine_outer_cr);
        let downward = shapes::downward(metrics.hv_contrast);

        let down1 = self
            .kit
            .dispiro(vec![
                Item::Af(self.kit.widths_lhs(Some(stroke_outer_cr))),
                self.kit.knot(
                    KnotType::Left,
                    df.left_sb() + metrics.o * 2.0,
                    top,
                    self.kit.heading(downward),
                ),
                self.kit.knot(
                    KnotType::G4,
                    x_bot1 - self.hsw_to_v(0.5 * w_co_cr),
                    0.0,
                    self.kit.widths_lhs_heading(Some(fine_outer_cr), downward),
                ),
            ])
            .geometry(kurbo::Affine::IDENTITY);
        let up2 = self
            .kit
            .dispiro(vec![
                Item::Af(self.kit.widths_rhs(Some(stroke_outer_cr))),
                self.kit.knot(
                    KnotType::Left,
                    df.right_sb() - metrics.o * 2.0,
                    top,
                    self.kit.heading(downward),
                ),
                self.kit.knot(
                    KnotType::G4,
                    x_bot2 + self.hsw_to_v(0.5 * w_co_cr),
                    0.0,
                    self.kit.widths_rhs_heading(Some(fine_outer_cr), downward),
                ),
            ])
            .geometry(kurbo::Affine::IDENTITY);
        let up1 = self
            .kit
            .dispiro(vec![
                self.kit.knot(
                    KnotType::Left,
                    x_right1 - shift_t,
                    w_mid,
                    self.kit
                        .widths_center_heading(Some(fine_inner_cr_top), downward),
                ),
                self.kit.knot(
                    KnotType::Right,
                    x_bot1 + shift_b,
                    0.0,
                    self.kit
                        .widths_center_heading(Some(fine_outer_cr), downward),
                ),
            ])
            .geometry(kurbo::Affine::IDENTITY);
        let down2 = self
            .kit
            .dispiro(vec![
                self.kit.knot(
                    KnotType::Left,
                    x_left2 + shift_t,
                    w_mid,
                    self.kit
                        .widths_center_heading(Some(fine_inner_cr_top), downward),
                ),
                self.kit.knot(
                    KnotType::Right,
                    x_bot2 - shift_b,
                    0.0,
                    self.kit
                        .widths_center_heading(Some(fine_outer_cr), downward),
                ),
            ])
            .geometry(kurbo::Affine::IDENTITY);

        Geometry::Combine(vec![down1, up2, up1, down2])
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        vec![("w.straightSerifless".to_string(), self.straight_serifless())]
    }
}
