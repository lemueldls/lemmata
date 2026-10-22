//! The lowercase y from `letter/latin/lower-y.ptl`.
//!
//! Ported: the non-cursive straight serifless form, which the default sans plan
//! selects. The curly body, the turn and loop bottoms, the serifs, the cursive
//! forms and the hooked right arm are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    metrics::Metrics,
    shapes::{self, Design},
    spiro_kit::Kit,
    util::{linreg, mix},
};

/// The lowercase y forms this module builds.
pub struct LowerY<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

/// The dimensions the straight serifless y reads.
struct Dims {
    top: f64,
    bottom: f64,
    l: f64,
    r: f64,
    sw: f64,
    ystrokel: f64,
    ystroker: f64,
    px2: f64,
    py2: f64,
    ds: f64,
    ds2: f64,
    diag_cor: f64,
    shrink_inner: f64,
}

impl LowerY<'_> {
    fn dims(&self) -> Dims {
        let metrics = self.metrics;
        let top = metrics.xh;
        let bottom = metrics.descender;
        let l = 0.0;
        let r = metrics.width;
        let m = mix(l, r, 0.5);
        let sw = metrics.stroke;
        let y_bottom_jut = metrics.jut * 1.25;
        let slabysize = 0.5 * y_bottom_jut * (metrics.width / (metrics.upm / 2.0)) + sw;
        let ds = (top - bottom) * 0.1;
        let ds2 = (top - bottom) * 0.01 + slabysize * 0.625;
        let yrstrokel = mix(l, r, 0.4) + self.hsw_to_v(sw * (2.0 / 3.0));
        let yrstroker = mix(r, r - metrics.sb, 0.8);
        let diag_cor = shapes::diag_cor(top - bottom, yrstroker - yrstrokel, None, None);
        let shrink_inner = metrics.stroke_width_blend(1.0, 0.85, None)
            * metrics.advice_stroke(3.25, 1.0, 1.0)
            / sw;
        let _ = m;

        Dims {
            top,
            bottom,
            l,
            r,
            sw,
            ystrokel: yrstrokel,
            ystroker: yrstroker,
            px2: 0.95,
            py2: 0.97,
            ds,
            ds2,
            diag_cor,
            shrink_inner,
        }
    }

    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    /// `yJoinProportion` for the straight body.
    fn join_proportion(&self, d: &Dims, x: f64) -> f64 {
        linreg(
            1.0 - d.px2,
            1.0 - d.py2,
            1.0,
            1.0,
            (x - d.ystrokel) / (d.ystroker - d.ystrokel),
        )
    }

    /// `Metrics.yJoinHeight`.
    fn join_height(&self, d: &Dims, m: f64) -> f64 {
        mix(d.bottom + d.ds2, d.top - d.ds, self.join_proportion(d, m))
    }

    /// `createSplitMask`.
    fn split_mask(&self, d: &Dims, direction: f64) -> Geometry {
        let m = mix(d.l, d.r, 0.5);
        let jp0 = self.join_proportion(d, mix(m, d.l, direction));
        let jp1 = self.join_proportion(d, mix(m, d.r, direction));
        let hs = 0.75 * d.sw;
        let y_left = mix(d.bottom + d.ds2, d.top - d.ds, jp1) + hs;
        let y_right = mix(d.bottom + d.ds2, d.top - d.ds, jp0) + hs;
        let y_min = d.bottom.min(y_left).min(y_right);

        spiro_geometry(self.kit.spiro_outline(vec![
            self.kit.knot(KnotType::Corner, d.r, y_min, Vec::new()),
            self.kit.knot(KnotType::Corner, d.r, y_right, Vec::new()),
            self.kit.knot(KnotType::Corner, d.l, y_left, Vec::new()),
            self.kit.knot(KnotType::Corner, d.l, y_min, Vec::new()),
        ]))
    }

    /// The right half's strokes, with `shrink` selecting the inner arm.
    fn right_dispiro(&self, d: &Dims, shrink: f64) -> Geometry {
        let join_x = mix(d.ystrokel, d.ystroker, 1.0 - d.px2);
        let join_y = mix(d.bottom + d.ds2, d.top - d.ds, 1.0 - d.py2);
        let base_x = mix(d.ystroker, join_x, (d.top - d.bottom) / (d.top - join_y));
        let arm_width = Some(d.diag_cor * d.sw);
        let downward = shapes::downward(self.metrics.hv_contrast);

        self.kit
            .dispiro(vec![
                self.kit.knot(
                    KnotType::Left,
                    d.ystroker,
                    d.top,
                    self.kit.widths_rhs_heading(arm_width, downward),
                ),
                self.kit.knot(
                    KnotType::Right,
                    join_x,
                    join_y,
                    self.kit.widths_rhs(Some(d.sw * shrink)),
                ),
                self.kit.knot(
                    KnotType::Right,
                    base_x,
                    d.bottom,
                    self.kit.widths_rhs_heading(arm_width, downward),
                ),
            ])
            .geometry(kurbo::Affine::IDENTITY)
    }

    /// The left half's stroke.
    fn left_dispiro(&self, d: &Dims) -> Geometry {
        let m = mix(d.l, d.r, 0.5);
        let downward = shapes::downward(self.metrics.hv_contrast);
        let arm_width = Some(d.diag_cor * d.sw);

        self.kit
            .dispiro(vec![
                self.kit.knot(
                    KnotType::Left,
                    d.l + d.r - d.ystroker,
                    d.top,
                    self.kit.widths_lhs_heading(arm_width, downward),
                ),
                self.kit.knot(
                    KnotType::Right,
                    m,
                    self.join_height(d, m),
                    self.kit.widths_lhs(Some(d.sw * d.shrink_inner)),
                ),
            ])
            .geometry(kurbo::Affine::IDENTITY)
    }

    /// `SmallYShape` with the straight body and no serifs.
    fn straight_serifless(&self) -> Geometry {
        let d = self.dims();
        let right = self.right_dispiro(&d, 1.0);
        let right_sh = self.right_dispiro(&d, d.shrink_inner);
        let left_mask = self.split_mask(&d, 1.0);
        let right_mask = self.split_mask(&d, -1.0);
        let left = self.left_dispiro(&d);

        let right_half = lemmata_ptl::glyph::union(vec![
            lemmata_ptl::glyph::intersection(vec![right, left_mask.clone()]),
            lemmata_ptl::glyph::difference(vec![right_sh, left_mask]),
        ]);
        let left_half = lemmata_ptl::glyph::difference(vec![
            left,
            right_mask,
            spiro_geometry(shapes::rect(
                self.kit,
                d.bottom + d.sw / 2.0,
                d.bottom - d.top,
                d.l,
                d.r,
            )),
        ]);

        Geometry::Combine(vec![right_half, left_half])
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        vec![("y.straightSerifless".to_string(), self.straight_serifless())]
    }
}

fn spiro_geometry(outline: lemmata_ptl::spiro_kit::SpiroOutline) -> Geometry {
    Geometry::Spiro {
        gizmo: kurbo::Affine::IDENTITY,
        closed: outline.closed,
        knots: outline.knots,
    }
}
