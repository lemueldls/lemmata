//! The X strand from `letter/latin/x.ptl`, shared with `upper-y.ptl`.
//!
//! Only the strand builders are ported here so far. They are what `Y` draws
//! with, and the rest of `x.ptl` (the `x`, the cap, the cursive and chancery
//! forms) is not ported yet.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(
    clippy::suboptimal_flops,
    clippy::too_many_arguments,
    clippy::similar_names
)]

use lemmata_geom::Geometry;
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::shapes::{diag_cor, downward, upward};
use lemmata_ptl::spiro_kit::Kit;
use lemmata_ptl::util::mix;

/// One half of an X: a strand from an outer point to the crossing.
#[derive(Clone, Copy, Debug)]
pub struct HalfXStrandArgs {
    /// `stb`: the straight body, which draws a single pair instead of a curve.
    pub straight: bool,
    /// `fSlab`: whether the strand has to clear a slab.
    pub slab: bool,
    pub x_outer: f64,
    pub y_outer: f64,
    pub x_center: f64,
    pub y_center: f64,
    pub turn: f64,
    pub p_straight: f64,
    pub tension: f64,
    /// `_sw`: the stroke width, defaulting to the global stroke.
    pub sw: Option<f64>,
    /// `_pConn`: how far the crossing pulls towards the outer point.
    pub p_conn: f64,
}

/// `HalfXStrand`.
#[must_use]
pub fn half_x_strand(kit: &Kit, metrics: &Metrics, args: &HalfXStrandArgs) -> Geometry {
    let sw = args.sw.unwrap_or(metrics.stroke);
    let side_jut = metrics.jut - metrics.half_stroke * metrics.hv_contrast;
    let sb_cor = if args.straight {
        metrics.stroke_width_blend(1.0, 6.0, None)
            * metrics.ox
            * ((args.y_outer - args.y_center).abs() / metrics.cap)
    } else {
        0.0
    };
    let direction = if args.x_center > args.x_outer {
        1.0
    } else {
        -1.0
    };
    let x_outer =
        args.x_outer + (0.5 * sw * metrics.hv_contrast + (-side_jut).max(sb_cor)) * direction;

    if args.straight {
        let hst = 0.5
            * sw
            * diag_cor(
                args.y_center - args.y_outer,
                args.x_center - x_outer,
                Some(0.0),
                Some(0.0),
            );
        let hse = 0.5 * sw * (metrics.advice_stroke(3.0, 1.0, 1.0) / metrics.stroke).min(0.97);
        let x_outer_adjusted =
            args.x_outer + (hst * metrics.hv_contrast + (-side_jut).max(sb_cor)) * direction;
        let heading = if args.y_outer < args.y_center {
            upward(metrics.hv_contrast)
        } else {
            downward(metrics.hv_contrast)
        };

        kit.dispiro(vec![
            kit.knot(
                lemmata_geom::KnotType::Corner,
                x_outer_adjusted,
                args.y_outer,
                kit.widths_heading(hst, hst, heading),
            ),
            kit.knot(
                lemmata_geom::KnotType::Corner,
                args.x_center,
                args.y_center,
                kit.widths_center(Some(2.0 * hse)),
            ),
        ])
        .geometry(kurbo::Affine::IDENTITY)
    } else {
        let height = (args.y_outer - args.y_center).abs();
        let slab_clearance = if args.slab {
            (sw / metrics.stroke) * metrics.advice_stroke2(2.0, 3.0, height, 1.0)
        } else {
            0.0
        };
        let turn_y_outer = mix(
            args.y_outer,
            args.y_center,
            if args.slab {
                args.turn.max(slab_clearance / height)
            } else {
                args.turn
            },
        );
        let cy_outer = mix(turn_y_outer, args.y_center, args.tension);
        let straight_x_outer = mix(x_outer, args.x_center, args.p_straight);
        let straight_y_outer = mix(cy_outer, args.y_center, args.p_straight);
        let x_center_adj = mix(args.x_center, x_outer, args.p_conn);
        let y_center_adj = mix(args.y_center, cy_outer, args.p_conn);
        let heading = if args.y_outer < args.y_center {
            upward(metrics.hv_contrast)
        } else {
            downward(metrics.hv_contrast)
        };

        kit.dispiro(vec![
            lemmata_ptl::control::Item::Af(kit.widths_center(Some(sw))),
            kit.knot(
                lemmata_geom::KnotType::Left,
                x_outer,
                args.y_outer,
                kit.heading(heading),
            ),
            kit.knot(
                lemmata_geom::KnotType::Right,
                x_outer,
                turn_y_outer,
                kit.heading(heading),
            ),
            kit.quad_controls(
                0.0,
                (cy_outer - turn_y_outer) / (straight_y_outer - turn_y_outer),
                24,
                Vec::new(),
            ),
            kit.knot(
                lemmata_geom::KnotType::Left,
                straight_x_outer,
                straight_y_outer,
                Vec::new(),
            ),
            kit.knot(
                lemmata_geom::KnotType::Right,
                x_center_adj,
                y_center_adj,
                Vec::new(),
            ),
        ])
        .geometry(kurbo::Affine::IDENTITY)
    }
}

/// `XStrand`: both halves, crossing in the middle.
#[must_use]
pub fn x_strand(
    kit: &Kit,
    metrics: &Metrics,
    straight: bool,
    slab: bool,
    x_left: f64,
    y_left: f64,
    x_right: f64,
    y_right: f64,
    turn: f64,
    p_straight: f64,
    tension: f64,
    sw: Option<f64>,
) -> Geometry {
    let x_mid = mix(x_left, x_right, 0.5);
    let y_mid = mix(y_left, y_right, 0.5);
    let left = half_x_strand(
        kit,
        metrics,
        &HalfXStrandArgs {
            straight,
            slab,
            x_outer: x_left,
            y_outer: y_left,
            x_center: x_mid,
            y_center: y_mid,
            turn,
            p_straight,
            tension,
            sw,
            p_conn: -0.001,
        },
    );
    let right = half_x_strand(
        kit,
        metrics,
        &HalfXStrandArgs {
            straight,
            slab,
            x_outer: x_right,
            y_outer: y_right,
            x_center: x_mid,
            y_center: y_mid,
            turn,
            p_straight,
            tension,
            sw,
            p_conn: -0.001,
        },
    );

    Geometry::Combine(vec![left, right])
}
