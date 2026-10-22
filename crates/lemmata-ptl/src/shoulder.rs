//! `nShoulder` from `letter/shared.ptl`.
//!
//! The shoulder a lower-case `h`, `n`, `m` and `b` draw: a flat side up from
//! `bottom`, an arch over the top and a flat across to `right`. `shape` sinks it
//! into a di-spiro and `mask` into a closed spiro outline, which is what the
//! boolean shapes cut against.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::KnotType;

use crate::{
    control::{Item, UserControlKnot},
    metrics::Metrics,
    shapes::{self, ArchArgs, ArchParams, Design},
    spiro_kit::{Dispiro, Kit, SpiroOutline},
    util::mix,
};

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
        let flat_y = self.flat_y(metrics);
        let left_x = self.left - self.fine * metrics.hv_contrast;
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

/// `nShoulder.earlessCorner`: the shoulder cut at the top with a corner at
/// `rise`, or rounded away when the rise is zero.
#[must_use]
pub fn n_shoulder_earless_corner(
    kit: &Kit,
    metrics: &Metrics,
    design: &Design,
    shoulder: &NShoulder,
    rise: f64,
) -> Dispiro {
    let across = UserControlKnot {
        ty: KnotType::Left,
        x: shoulder.right,
        y: (shoulder.top - shoulder.adb).max(shoulder.bottom + metrics.tiny),
        af: Vec::new(),
    };
    let mut items = vec![Item::Af(kit.widths_rhs(Some(shoulder.stroke)))];
    if rise == 0.0 {
        items.push(kit.knot(
            KnotType::Left,
            shoulder.left + metrics.tiny,
            shoulder.top,
            kit.heading(shapes::rightward()),
        ));
        items.push(kit.knot(
            KnotType::Right,
            mix(shoulder.left, shoulder.right, 0.5) - metrics.correction_o_mid_x * shoulder.stroke,
            shoulder.top,
            Vec::new(),
        ));
        items.push(kit.archv(None, None));
    } else {
        let corner = UserControlKnot {
            ty: KnotType::G4,
            x: shoulder.left,
            y: shoulder.top - rise,
            af: Vec::new(),
        };
        items.push(kit.knot(corner.ty, corner.x, corner.y, Vec::new()));
        items.extend(shapes::arch(
            kit,
            &ArchArgs {
                y: shoulder.top,
                p: None,
                sw: shoulder.stroke,
                compact: false,
                o: metrics.o,
                angle_pre: None,
                angle_post: None,
                blend_pre: Some(false),
                blend_post: None,
                sw_before: Some(shoulder.stroke),
                sw_after: Some(shoulder.stroke),
                lhs: false,
            },
            &corner,
            &across,
            &NShoulder::params(metrics, design),
        ));
    }
    items.push(kit.knot(across.ty, across.x, across.y, Vec::new()));
    items.push(kit.knot(
        KnotType::Right,
        shoulder.right,
        shoulder.bottom,
        kit.heading(shapes::downward(metrics.hv_contrast)),
    ));

    kit.dispiro(items)
}

/// `nShoulder.earlessRounded`: the shoulder with its left side rounded into the
/// bar at `left_y0`.
#[must_use]
pub fn n_shoulder_earless_rounded(
    kit: &Kit,
    metrics: &Metrics,
    design: &Design,
    shoulder: &NShoulder,
) -> Dispiro {
    let flat_y = (shoulder.top - shoulder.ada).max(shoulder.left_y0 + metrics.tiny);
    let before = UserControlKnot {
        ty: KnotType::Right,
        x: shoulder.left,
        y: flat_y,
        af: Vec::new(),
    };
    let across = UserControlKnot {
        ty: KnotType::Left,
        x: shoulder.right,
        y: (shoulder.top - shoulder.adb).max(shoulder.bottom + metrics.tiny),
        af: Vec::new(),
    };
    let mut items = vec![
        kit.knot(
            KnotType::Left,
            shoulder.left,
            shoulder.left_y0,
            kit.widths_rhs_heading(Some(shoulder.stroke), shapes::upward(metrics.hv_contrast)),
        ),
        kit.knot(KnotType::Right, before.x, before.y, Vec::new()),
    ];
    items.extend(shapes::arch(
        kit,
        &ArchArgs {
            y: shoulder.top,
            p: None,
            sw: shoulder.stroke,
            compact: false,
            o: metrics.o,
            angle_pre: None,
            angle_post: None,
            blend_pre: None,
            blend_post: None,
            sw_before: Some(shoulder.stroke),
            sw_after: Some(shoulder.stroke),
            lhs: false,
        },
        &before,
        &across,
        &NShoulder::params(metrics, design),
    ));
    items.push(kit.knot(across.ty, across.x, across.y, Vec::new()));
    items.push(kit.knot(
        KnotType::Right,
        shoulder.right,
        shoulder.bottom,
        kit.heading(shapes::downward(metrics.hv_contrast)),
    ));

    kit.dispiro(items)
}
