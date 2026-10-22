//! The lowercase r from `letter/latin/lower-r.ptl`.
//!
//! Ported: the standard serifless form, which the default sans plan selects.
//! The flat-top, earless, hookless, corner-hooked, compact and narrow forms and
//! their serifs are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use kurbo::Vec2;
use lemmata_geom::Geometry;
use lemmata_ptl::{
    control::Item,
    letter_model::DivFrame,
    metrics::Metrics,
    shapes::{self, Design},
    spiro_kit::{Direction, Family, Kit, Position},
    util::{linreg, mix},
};

/// The lowercase r forms this module builds.
pub struct LowerR<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerR<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    /// `SmallRShape.Standard` for the standard serifless mode.
    fn standard(&self) -> Geometry {
        let metrics = self.metrics;
        let design = self.design;
        let df = DivFrame::new(metrics.clone(), Some(1.0), None, None);
        let stroke = metrics.stroke;
        let fine = metrics.shoulder_fine;
        let top = metrics.xh;
        let bottom = 0.0;
        let bar_left = df.left_sb() + metrics.r_balance;
        let hook_x = df.right_sb() + metrics.r_balance2 - (metrics.ox - metrics.o);
        let arch_middle_x = mix(
            bar_left + self.hsw_to_v(stroke - fine),
            hook_x,
            0.54 + 2.0 * metrics.tan_slope * stroke / metrics.width,
        );
        let p_mix_in = 0.65 + 4.0 * metrics.tan_slope * stroke / metrics.width;
        let middle_in = mix(
            bar_left + self.hsw_to_v(stroke),
            hook_x - self.hsw_to_v(1.05 * stroke),
            p_mix_in,
        ) - metrics.correction_o_mid_s;
        let ward = Vec2::new(
            (metrics.tan_slope - (arch_middle_x - middle_in) / stroke).min(0.0),
            -1.0,
        );
        let hook_y = metrics.r_hook + stroke * 0.5;
        let cor_x = metrics.correction_o_mid_s * linreg(72.0, 0.75, 108.0, 1.0, stroke);
        let arch_depth = 0.47 * (top - bottom);
        let ada = shapes::arch_depth_a_clamped(
            top,
            bottom,
            Some(metrics.arch_depth_a_of(arch_depth, None)),
            Some(metrics.arch_depth_b_of(arch_depth, None)),
            metrics.arch_depth_a,
            metrics.arch_depth_b,
        );

        let items = vec![
            Item::Af(self.kit.widths_lhs(None)),
            self.kit.pair(
                Family::G4,
                Direction::Up,
                Position::Start,
                hook_x,
                top - hook_y,
                self.kit.heading(shapes::upward(metrics.hv_contrast)),
            ),
            self.kit.arcvh(Some(32), Some(design.r_hook_superness)),
            self.kit.pair(
                Family::G4,
                Direction::Left,
                Position::Mid,
                arch_middle_x - cor_x,
                top - metrics.o,
                self.kit.widths_lhs_heading(Some(stroke), ward),
            ),
            self.kit.archv(Some(32), None),
            self.kit.pair(
                Family::Straight,
                Direction::Down,
                Position::End,
                bar_left + self.hsw_to_v(stroke - fine),
                (bottom + metrics.tiny).max(top - ada),
                self.kit
                    .widths_lhs_heading(Some(fine), shapes::downward(metrics.hv_contrast)),
            ),
        ];
        let arm = self.kit.dispiro(items);
        let stem = shapes::vbar_l(self.kit, metrics, bar_left, bottom, top, stroke)
            .geometry(kurbo::Affine::IDENTITY);

        Geometry::Combine(vec![arm.geometry(kurbo::Affine::IDENTITY), stem])
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        vec![("r.serifless".to_string(), self.standard())]
    }
}
