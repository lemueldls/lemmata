//! `nShoulder` from `letter/shared.ptl`.
//!
//! The shoulder a lower-case `h`, `n`, `m` and `b` draw: a flat side up from
//! `bottom`, an arch over the top and a flat across to `right`. `shape` sinks it
//! into a di-spiro and `mask` into a closed spiro outline, which is what the
//! boolean shapes cut against.

use lemmata_geom::KnotType;

use crate::control::{Item, UserControlKnot};
use crate::metrics::Metrics;
use crate::shapes::{self, ArchArgs, ArchParams, Design};
use crate::spiro_kit::{Dispiro, Kit, SpiroOutline};

/// The shoulder's dimensions.
#[derive(Clone, Copy, Debug)]
pub struct NShoulder {
    pub left: f64,
    pub right: f64,
    pub fine: f64,
    pub top: f64,
    pub bottom: f64,
    pub ada: f64,
    pub adb: f64,
    pub stroke: f64,
    /// The flat side's starting height.
    pub left_y0: f64,
}

impl NShoulder {
    const fn params(metrics: &Metrics, design: &Design) -> ArchParams {
        ArchParams {
            superness: design.superness,
            tan_slope: metrics.tan_slope,
            hv_contrast: metrics.hv_contrast,
            o: metrics.o,
            correction_o_mid_x: metrics.correction_o_mid_x,
        }
    }

    fn flat_y(&self, metrics: &Metrics) -> f64 {
        (self.top - self.ada).max(self.left_y0 + metrics.tiny)
    }

    /// The knots, which `shape` and `mask` sink differently.
    #[must_use]
    pub fn items(&self, kit: &Kit, metrics: &Metrics, design: &Design, mask: bool) -> Vec<Item> {
        let hsw = |value: f64| value * metrics.hv_contrast;
        let flat_y = self.flat_y(metrics);
        let left_x = self.left - hsw(self.fine);
        let before = UserControlKnot {
            ty: KnotType::Right,
            x: left_x,
            y: flat_y,
            af: Vec::new(),
        };
        let across = UserControlKnot {
            ty: KnotType::Left,
            x: self.right,
            y: (self.top - self.adb).max(self.bottom + metrics.tiny),
            af: kit.widths_rhs(Some(self.stroke)),
        };
        let end_ty = if mask {
            KnotType::Corner
        } else {
            KnotType::Right
        };
        let end_af =
            kit.widths_rhs_heading(Some(self.stroke), shapes::downward(metrics.hv_contrast));
        let mut items = vec![
            kit.knot(
                KnotType::Left,
                left_x,
                self.left_y0,
                kit.widths_rhs(Some(self.fine)),
            ),
            kit.knot(before.ty, before.x, before.y, before.af.clone()),
        ];
        items.extend(shapes::arch(
            kit,
            &ArchArgs {
                y: self.top,
                p: None,
                sw: self.stroke,
                compact: false,
                o: metrics.o,
                angle_pre: None,
                angle_post: None,
                blend_pre: None,
                blend_post: None,
                sw_before: Some(self.fine),
                sw_after: Some(self.stroke),
                lhs: false,
            },
            &before,
            &across,
            &Self::params(metrics, design),
        ));
        items.push(kit.knot(across.ty, across.x, across.y, across.af));
        items.push(kit.knot(end_ty, self.right, self.bottom, end_af));
        if mask {
            items.push(kit.knot(KnotType::Corner, self.left, self.bottom, Vec::new()));
        }

        items
    }

    /// `nShoulder.shape`.
    #[must_use]
    pub fn shape(&self, kit: &Kit, metrics: &Metrics, design: &Design) -> Dispiro {
        kit.dispiro(self.items(kit, metrics, design, false))
    }

    /// `nShoulder.mask`.
    #[must_use]
    pub fn mask(&self, kit: &Kit, metrics: &Metrics, design: &Design) -> SpiroOutline {
        kit.spiro_outline(self.items(kit, metrics, design, true))
    }
}
