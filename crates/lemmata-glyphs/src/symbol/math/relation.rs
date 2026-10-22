//! The relation signs from `symbol/math/relation.ptl`.
//!
//! Ported: `equal`, `less` and `greater`. The negated, subset and ligature
//! forms are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    metrics::Metrics,
    shapes::{self, Design, leftward, rightward},
    spiro_kit::Kit,
    util::mix,
};

/// The relation signs this module builds.
pub struct Relation<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl Relation<'_> {
    fn half_space(&self) -> f64 {
        let metrics = self.metrics;

        (metrics.oper_top - metrics.oper_bot) * 2.0 * self.design.equal_wideness
    }

    /// `dH`, the half height of `less` and `greater`.
    fn d_h(&self) -> f64 {
        let metrics = self.metrics;

        (5.0 / 13.0) * (metrics.oper_top - metrics.oper_bot)
    }

    /// `LessGreaterExpansion`.
    fn expansion(&self, top: f64, bot: f64, l: f64, r: f64) -> f64 {
        (1.0 + (0.5 * (top - bot) / (r - l)).powi(2)).sqrt()
    }

    /// `EqualShape`.
    fn equal(&self) -> Geometry {
        let metrics = self.metrics;
        let half = self.half_space();
        let mid = metrics.symbol_mid;
        let sw = Some(metrics.operator_stroke);

        Geometry::Combine(vec![
            shapes::hbar_m(
                self.kit,
                metrics,
                metrics.sb,
                metrics.right_sb,
                mid + half,
                sw,
            )
            .geometry(kurbo::Affine::IDENTITY),
            shapes::hbar_m(
                self.kit,
                metrics,
                metrics.sb,
                metrics.right_sb,
                mid - half,
                sw,
            )
            .geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// `LessShapeHalf`.
    fn less_half(&self, sign: u8, top: f64, bot: f64, l: f64, r: f64, s: f64) -> Geometry {
        let exp = self.expansion(top, bot, l, r);
        let exp_amend = exp.min(std::f64::consts::SQRT_2);
        let end_adj = 0.5 * s * (exp - exp_amend);
        let mid = mix(top, bot, 0.5);
        let p = 1.0;
        let y = mix(mid, if sign == 0 { bot } else { top }, p);

        self.kit
            .dispiro(vec![
                lemmata_ptl::control::Item::Af(self.kit.widths_center(Some(s))),
                self.kit.knot(KnotType::Left, mix(l, r, p), y, Vec::new()),
                self.kit.knot(
                    KnotType::Right,
                    l - end_adj,
                    mid,
                    self.kit
                        .widths_center_heading(Some(s * exp_amend), leftward()),
                ),
            ])
            .geometry(kurbo::Affine::IDENTITY)
    }

    /// `GreaterShapeHalf`.
    fn greater_half(&self, sign: u8, top: f64, bot: f64, l: f64, r: f64, s: f64) -> Geometry {
        let exp = self.expansion(top, bot, l, r);
        let exp_amend = exp.min(std::f64::consts::SQRT_2);
        let end_adj = 0.5 * s * (exp - exp_amend);
        let mid = mix(top, bot, 0.5);
        let p = 1.0;
        let y = mix(mid, if sign == 0 { bot } else { top }, p);

        self.kit
            .dispiro(vec![
                lemmata_ptl::control::Item::Af(self.kit.widths_center(Some(s))),
                self.kit.knot(KnotType::Left, mix(r, l, p), y, Vec::new()),
                self.kit.knot(
                    KnotType::Right,
                    r + end_adj,
                    mid,
                    self.kit
                        .widths_center_heading(Some(s * exp_amend), rightward()),
                ),
            ])
            .geometry(kurbo::Affine::IDENTITY)
    }

    fn chevron(&self, top: f64, bot: f64, less: bool) -> Geometry {
        let metrics = self.metrics;
        let l = metrics.sb;
        let r = metrics.right_sb;
        let s = metrics.operator_stroke;
        let half = |sign| {
            if less {
                self.less_half(sign, top, bot, l, r, s)
            } else {
                self.greater_half(sign, top, bot, l, r, s)
            }
        };

        Geometry::Combine(vec![half(0), half(1)])
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mid = self.metrics.symbol_mid;
        let d_h = self.d_h();

        vec![
            ("equal".to_string(), self.equal()),
            ("less".to_string(), self.chevron(mid + d_h, mid - d_h, true)),
            (
                "greater".to_string(),
                self.chevron(mid + d_h, mid - d_h, false),
            ),
        ]
    }
}
