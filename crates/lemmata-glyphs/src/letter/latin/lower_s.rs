//! The lowercase s from `letter/latin/s.ptl`.
//!
//! Ported: the serifless form, which the default sans plan selects. The serifed
//! and inward-slab forms, the curly tail and the reversed s are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::{
    control::{Item, UserControlKnot},
    metrics::Metrics,
    shapes::{self, Design, HookArgs, HookParams},
    spiro_kit::Kit,
    util::{linreg, mix},
};

/// The lowercase s forms this module builds.
pub struct LowerS<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerS<'_> {
    const fn hook_params(&self) -> HookParams {
        HookParams {
            superness: self.design.superness,
            tan_slope: self.metrics.tan_slope,
            contrast: self.metrics.contrast,
            correction_o_mid_x: self.metrics.correction_o_mid_x,
            ox_hook: self.metrics.ox_hook,
            hook: self.metrics.hook,
            slope_angle: 0.0,
            is_italic: false,
        }
    }

    /// `AdviceSArchDepth` with the s's downward sign.
    fn arch_depth(&self, height: f64, stroke: f64) -> f64 {
        let metrics = self.metrics;
        let stroke_factor = stroke
            * linreg(126.0, 1.0, 137.0, 1.025, stroke).clamp(1.0, 2.0)
            * (metrics.width / (metrics.upm / 2.0)).sqrt().clamp(0.0, 1.0);
        let width_factor = metrics.right_sb - metrics.sb;

        height * 0.22 + 0.12 * stroke_factor + 0.05 * width_factor
            - metrics.tan_slope * metrics.smooth_adjust
    }

    /// `SNeck`. Upstream delays the two outer interpolators so they resolve
    /// against the middle knot rather than the nodes either side of the neck.
    fn neck(&self, a: (f64, f64), b: (f64, f64), height: f64, ess: f64) -> Vec<Item> {
        let adjust = 0.1 * (self.metrics.right_sb - self.metrics.sb) / height;
        let mid = (mix(a.0, b.0, 0.5), mix(a.1, b.1, 0.5));

        vec![
            self.kit.knot(
                lemmata_geom::KnotType::G4,
                mix(a.0, mid.0, 0.5),
                mix(a.1, mid.1, 0.7 + adjust),
                self.kit.unimportant(),
            ),
            self.kit.knot(
                lemmata_geom::KnotType::G2,
                mid.0,
                mid.1,
                self.kit.widths_center(Some(ess)),
            ),
            self.kit.knot(
                lemmata_geom::KnotType::G4,
                mix(mid.0, b.0, 0.5),
                mix(mid.1, b.1, 0.3 - adjust),
                self.kit.unimportant(),
            ),
        ]
    }

    /// `SmallSStrokeImpl` with both ends serifless.
    fn serifless(&self) -> Geometry {
        let metrics = self.metrics;
        let top = metrics.xh;
        let bot = 0.0;
        let stroke = metrics.advice_stroke2(2.0, 3.0, top - bot, 1.0);
        let ess = stroke * metrics.ess_ratio_lower;
        let depth = self.arch_depth(top - bot, stroke);
        let so_top = 0.0;
        let so_bot = metrics.ox;

        let start_before = UserControlKnot {
            ty: lemmata_geom::KnotType::G4,
            x: metrics.right_sb + metrics.ox - so_top,
            y: top - metrics.s_hook,
            af: Vec::new(),
        };
        let start_after = UserControlKnot {
            ty: lemmata_geom::KnotType::G4,
            x: metrics.sb - metrics.ox + so_top,
            y: top - depth,
            af: Vec::new(),
        };
        let lemmata_ptl::shapes::HookOut {
            items: start_hook,
            terminal: start_terminal,
        } = shapes::hook_start(
            self.kit,
            &start_before,
            &start_after,
            &HookArgs {
                y_ref: top,
                sw: stroke,
                sw_terminal: stroke,
                is_tail: false,
                suppress_swash: false,
                overshoot: metrics.o,
            },
            &self.hook_params(),
        );

        let end_before = UserControlKnot {
            ty: lemmata_geom::KnotType::G4,
            x: metrics.right_sb + metrics.ox - so_bot,
            y: bot + depth,
            af: Vec::new(),
        };
        let end_after = UserControlKnot {
            ty: lemmata_geom::KnotType::G4,
            x: metrics.sb - metrics.ox + so_bot,
            y: bot + metrics.s_hook,
            af: Vec::new(),
        };
        let lemmata_ptl::shapes::HookOut {
            items: end_hook,
            terminal: end_terminal,
        } = shapes::hook_end(
            self.kit,
            &end_before,
            &end_after,
            &HookArgs {
                y_ref: bot,
                sw: stroke,
                sw_terminal: stroke,
                is_tail: false,
                suppress_swash: false,
                overshoot: metrics.o,
            },
            &self.hook_params(),
        );

        let mut items = vec![
            Item::Af(self.kit.widths_lhs(Some(stroke))),
            self.kit.knot(
                start_terminal.ty,
                start_terminal.x,
                start_terminal.y,
                start_terminal.af,
            ),
        ];
        items.extend(start_hook);
        items.push(
            self.kit
                .knot(start_after.ty, start_after.x, start_after.y, Vec::new()),
        );
        items.extend(self.neck(
            (metrics.sb - metrics.ox + so_top, top - depth),
            (metrics.right_sb + metrics.ox - so_bot, bot + depth),
            top - bot,
            ess,
        ));
        items.push(self.kit.knot(
            end_before.ty,
            end_before.x,
            end_before.y,
            self.kit.widths_rhs(Some(stroke)),
        ));
        items.extend(end_hook);
        items.push(self.kit.knot(
            end_terminal.ty,
            end_terminal.x,
            end_terminal.y,
            end_terminal.af,
        ));

        self.kit.dispiro(items).geometry(kurbo::Affine::IDENTITY)
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        vec![("s.serifless".to_string(), self.serifless())]
    }
}
