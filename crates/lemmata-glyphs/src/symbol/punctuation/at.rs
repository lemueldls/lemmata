//! The at sign from `symbol/punctuation/at.ptl`.
//!
//! Ported: the normal-height threefold form, which the default plan selects.
//! The tall, solid-inner, fourfold and compact forms are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    control::{Item, UserControlKnot},
    metrics::Metrics,
    shapes::{self, ArchArgs, ArchParams, CenterAtArgs, Design, leftward},
    spiro_kit::Kit,
    util::mix,
};

/// The at signs this module builds.
pub struct At<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl At<'_> {
    const fn arch_params(&self) -> ArchParams {
        ArchParams {
            superness: self.design.superness,
            tan_slope: self.metrics.tan_slope,
            hv_contrast: self.metrics.hv_contrast,
            o: self.metrics.o,
            correction_o_mid_x: self.metrics.correction_o_mid_x,
        }
    }

    /// The arch's arguments, with the defaults upstream uses.
    fn arch_args(&self, y: f64, sw: f64, lhs: bool) -> ArchArgs {
        ArchArgs {
            y,
            p: None,
            sw,
            compact: false,
            o: self.metrics.o,
            angle_pre: None,
            angle_post: None,
            blend_pre: None,
            blend_post: None,
            sw_before: None,
            sw_after: None,
            lhs,
        }
    }

    /// `ThreeFoldShape` at the normal height, the non-solid form.
    fn threefold(&self) -> Geometry {
        let metrics = self.metrics;
        let height = mix(
            metrics.oper_top - metrics.oper_bot,
            metrics.paren_top - metrics.paren_bot,
            0.6,
        );
        let top = metrics.symbol_mid + height / 2.0;
        let bot = metrics.symbol_mid - height / 2.0;
        let otop = mix(bot, top, 0.75);
        let obot = mix(top, bot, 0.8);
        let sw = metrics.advice_stroke(3.5, 1.0, 1.0);
        let m1 = mix(metrics.sb + sw, metrics.right_sb - sw, 0.47) - sw / 2.0;
        let m2 = mix(m1, metrics.right_sb, 0.52) + metrics.tan_slope * sw;
        let factor = (metrics.right_sb - m1) / (metrics.right_sb - metrics.sb) * 1.1;
        let ada = metrics.arch_depth_a * factor;
        let adb = metrics.arch_depth_b * factor;
        let params = self.arch_params();
        let knot = |ty, x, y, af| self.kit.knot(ty, x, y, af);

        let inner_before = UserControlKnot {
            ty: KnotType::Right,
            x: m1,
            y: obot + adb,
            af: Vec::new(),
        };
        let inner_after = UserControlKnot {
            ty: KnotType::Left,
            x: metrics.right_sb,
            y: obot + ada,
            af: Vec::new(),
        };
        let mut inner = vec![
            Item::Af(self.kit.widths_lhs(Some(sw))),
            knot(
                KnotType::Left,
                metrics.right_sb,
                otop - metrics.o,
                self.kit.heading(leftward()),
            ),
            knot(KnotType::Right, m2, otop - metrics.o, Vec::new()),
            self.kit.archv(None, None),
            knot(KnotType::Left, m1, otop - ada, Vec::new()),
            knot(KnotType::Right, m1, obot + adb, Vec::new()),
        ];
        inner.extend(shapes::arch(
            self.kit,
            &self.arch_args(obot, sw, true),
            &inner_before,
            &inner_after,
            &params,
        ));
        inner.push(knot(
            KnotType::Left,
            metrics.right_sb,
            obot + ada,
            Vec::new(),
        ));
        inner.push(knot(
            KnotType::Right,
            metrics.right_sb,
            otop - metrics.o,
            Vec::new(),
        ));

        let outer_before = UserControlKnot {
            ty: KnotType::Right,
            x: metrics.right_sb,
            y: top - metrics.arch_depth_b,
            af: Vec::new(),
        };
        let outer_after = UserControlKnot {
            ty: KnotType::Left,
            x: metrics.sb,
            y: top - metrics.arch_depth_a,
            af: Vec::new(),
        };
        let mut outer = vec![
            Item::Af(self.kit.widths_lhs(Some(sw))),
            knot(KnotType::Left, metrics.right_sb, obot + ada, Vec::new()),
            knot(
                KnotType::Right,
                metrics.right_sb,
                top - metrics.arch_depth_b,
                Vec::new(),
            ),
        ];
        outer.extend(shapes::arch(
            self.kit,
            &self.arch_args(top, metrics.stroke, true),
            &outer_before,
            &outer_after,
            &params,
        ));
        outer.push(knot(
            KnotType::Left,
            metrics.sb,
            top - metrics.arch_depth_a,
            self.kit.widths_lhs(Some(sw)),
        ));
        outer.push(knot(
            KnotType::Right,
            metrics.sb,
            bot + metrics.arch_depth_b,
            Vec::new(),
        ));
        outer.push(self.kit.arcvh(None, None));
        outer.push(shapes::center_at(
            self.kit,
            &CenterAtArgs {
                x: metrics.middle,
                y: bot,
                sw: metrics.stroke,
                compact: false,
                o: metrics.o,
                sw_before: metrics.stroke,
                sw_after: metrics.stroke,
                lhs: true,
                rtl: false,
                at_bot: true,
                knot_ty: Some(KnotType::Left),
            },
            &params,
        ));
        outer.push(knot(
            KnotType::Right,
            metrics.right_sb - metrics.half_stroke + metrics.tan_slope * sw,
            bot + metrics.o,
            Vec::new(),
        ));

        lemmata_ptl::glyph::union(vec![
            self.kit.dispiro(inner).geometry(kurbo::Affine::IDENTITY),
            self.kit.dispiro(outer).geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        vec![("at.threefold".to_string(), self.threefold())]
    }
}
