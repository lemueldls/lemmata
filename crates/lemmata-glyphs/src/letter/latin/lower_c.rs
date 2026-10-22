//! The lowercase c from `letter/latin/c.ptl`.
//!
//! Ported: the serifless form, which the default sans plan selects. The serifed
//! and inward-slab forms, the reversed c and the accented compositions are not
//! ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::{
    control::{Item, UserControlKnot},
    metrics::Metrics,
    shapes::{self, Design, FlatSide, HookArgs, HookParams},
    spiro_kit::Kit,
};

/// The lowercase c forms this module builds.
pub struct LowerC<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerC<'_> {
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

    /// `CShapeT` with both ends serifless.
    fn serifless(&self) -> Geometry {
        let metrics = self.metrics;
        let offset = 0.0;
        let top = metrics.xh;
        let bot = 0.0;
        let sw = metrics.stroke;
        let x_right = metrics.right_sb - offset;
        let x_left = metrics.sb + offset;

        let start_before = UserControlKnot {
            ty: lemmata_geom::KnotType::G4,
            x: x_right,
            y: top - metrics.hook,
            af: Vec::new(),
        };
        let side = FlatSide {
            x: x_left,
            d: bot,
            u: top,
            ada: metrics.small_arch_depth_a,
            adb: metrics.small_arch_depth_b,
            ox: metrics.ox,
            af: Vec::new(),
        };
        let side_items = shapes::flatside_ld(self.kit, &side);
        let start_after = shapes::knot_of(&side_items[0]);
        let lemmata_ptl::shapes::HookOut {
            items: start_hook,
            terminal: start_terminal,
        } = shapes::hook_start(
            self.kit,
            &start_before,
            &start_after,
            &HookArgs {
                y_ref: top - offset,
                sw,
                sw_terminal: sw,
                is_tail: false,
                suppress_swash: false,
                overshoot: metrics.o,
            },
            &self.hook_params(),
        );

        let end_before = shapes::knot_of(side_items.last().expect("the side has knots"));
        let end_after = UserControlKnot {
            ty: lemmata_geom::KnotType::G4,
            x: x_right,
            y: bot + metrics.hook,
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
                y_ref: bot + offset,
                sw,
                sw_terminal: sw,
                is_tail: false,
                suppress_swash: false,
                overshoot: metrics.o,
            },
            &self.hook_params(),
        );

        let mut items = vec![
            Item::Af(self.kit.widths_lhs(Some(sw))),
            self.kit.knot(
                start_terminal.ty,
                start_terminal.x,
                start_terminal.y,
                start_terminal.af,
            ),
        ];
        items.extend(start_hook);
        items.extend(side_items);
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
        vec![("c.serifless".to_string(), self.serifless())]
    }
}
