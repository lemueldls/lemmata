//! The ampersand from `symbol/punctuation/ampersand.ptl`.
//!
//! Ported: the closed form, which the default sans plan selects. The open,
//! tailed and et forms are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use kurbo::Vec2;
use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    control::UserControlKnot,
    metrics::Metrics,
    shapes::{self, ArchArgs, ArchParams, Design},
    spiro_kit::{Direction, Family, Kit, Position},
    util::mix,
};

/// The ampersands this module builds.
pub struct Ampersand<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

/// The dimensions `ampersand.closed` reads.
struct Dims {
    sw: f64,
    tip_sw: f64,
    incised_sw: f64,
    slope: Vec2,
    x_terminal: f64,
    x_loop_left: f64,
    y_loop_left: f64,
    x_loop_right: f64,
    y_loop_right: f64,
    y_mock: f64,
    x_bottom: f64,
}

impl Ampersand<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    fn dims(&self) -> Dims {
        let metrics = self.metrics;
        let sw = metrics.advice_stroke2(2.5, 3.0, metrics.cap, 1.0);
        let tip_sw = metrics.advice_stroke(3.0, 1.0, 1.0);
        let incised_sw = metrics.advice_stroke(5.0, 1.0, 1.0);
        let px_loop_right = 0.75 + metrics.half_stroke / (metrics.right_sb - metrics.sb);
        let px_loop_left = 0.05;
        let slope_val = -0.5 * (sw - incised_sw) / sw;
        let range = px_loop_right - px_loop_left;
        let ada = metrics.arch_depth_a_of(metrics.arch_depth * range, Some(metrics.width));
        let adb = metrics.arch_depth_b_of(metrics.arch_depth * range, Some(metrics.width));
        let x_terminal = mix(metrics.sb, metrics.right_sb, 1.1) - self.hsw_to_v(sw);
        let x_loop_left = mix(metrics.sb, metrics.right_sb, px_loop_left);
        let x_loop_right = mix(metrics.sb, metrics.right_sb, px_loop_right);
        let x_bottom = mix(metrics.sb, metrics.right_sb, 0.44)
            + (0.125 + 0.5 * metrics.correction_o_mid_x + slope_val) * sw;

        Dims {
            sw,
            tip_sw,
            incised_sw,
            slope: Vec2::new(slope_val, -1.0),
            x_terminal,
            x_loop_left,
            y_loop_left: metrics.cap - ada,
            x_loop_right,
            y_loop_right: metrics.cap - adb,
            y_mock: metrics.cap - ada - adb * 0.3,
            x_bottom,
        }
    }

    fn rect(&self, top: f64, bottom: f64, left: f64, right: f64) -> Geometry {
        spiro_geometry(shapes::rect(self.kit, top, bottom, left, right))
    }

    fn very_far(&self) -> f64 {
        self.metrics.upm * 16.0
    }

    fn mask_above(&self, y: f64) -> Geometry {
        let far = self.very_far();

        self.rect(far, y, -far, far)
    }

    fn mask_below(&self, y: f64) -> Geometry {
        let far = self.very_far();

        self.rect(y, -far, -far, far)
    }

    fn mask_right(&self, x: f64) -> Geometry {
        let far = self.very_far();

        self.rect(far, -far, x, far)
    }

    /// `ClosedAmpersandLeadDim`.
    fn lead_dim(&self, sw: f64, _sw_tip: f64, offset: f64) -> (f64, f64, f64, f64, f64) {
        let metrics = self.metrics;
        let x1 = mix(metrics.sb, metrics.right_sb, 0.80) - offset;
        let x2 = mix(metrics.sb, metrics.right_sb, 1.1) + self.hsw_to_v(0.1 * sw) - offset;
        let y1 = 0.0;
        let y2 = mix(0.0, metrics.cap, 0.375) + sw / 2.0;
        let k1 = 0.5;

        (x1, x2, y1, y2, k1)
    }

    /// `ClosedAmpersandLead`.
    fn lead(&self, sw: f64, sw_tip: f64) -> Geometry {
        let (x1, x2, y1, y2, k1) = self.lead_dim(sw, sw_tip, 0.0);
        let outline = self
            .kit
            .dispiro(vec![
                self.kit.knot(
                    KnotType::Left,
                    mix(x1, x2, 4.0),
                    mix(y1, y2, 4.0),
                    self.kit.widths_rhs(Some(sw_tip)),
                ),
                self.kit.knot(
                    KnotType::Right,
                    mix(x1, x2, k1),
                    mix(y1, y2, k1),
                    Vec::new(),
                ),
                self.kit.pair(
                    Family::G4,
                    Direction::Left,
                    Position::Mid,
                    self.dims().x_bottom,
                    self.metrics.o,
                    self.kit.widths_rhs_heading(Some(sw), self.dims().slope),
                ),
            ])
            .geometry(kurbo::Affine::IDENTITY);

        lemmata_ptl::glyph::intersection(vec![self.mask_below(y2), outline])
    }

    /// `ClosedAmpersandLeadMask2`.
    fn lead_mask2(&self, sw: f64, sw_tip: f64) -> Geometry {
        let offset = self.hsw_to_v(sw - sw_tip);
        let (x1, x2, y1, y2, k1) = self.lead_dim(sw, sw_tip, offset);
        let outline = self.kit.spiro_outline(vec![
            self.kit.knot(
                KnotType::Left,
                mix(x1, x2, 4.0),
                mix(y1, y2, 4.0),
                self.kit.widths_rhs(Some(sw_tip)),
            ),
            self.kit.knot(
                KnotType::Right,
                mix(x1, x2, k1),
                mix(y1, y2, k1),
                Vec::new(),
            ),
            self.kit.pair(
                Family::G4,
                Direction::Left,
                Position::Mid,
                self.dims().x_bottom,
                self.metrics.o,
                self.kit.widths_rhs(Some(sw)),
            ),
            self.kit
                .knot(KnotType::Corner, 0.0, self.metrics.o, Vec::new()),
            self.kit
                .knot(KnotType::Corner, 0.0, mix(y1, y2, 4.0), Vec::new()),
        ]);

        lemmata_ptl::glyph::intersection(vec![self.mask_below(y2), spiro_geometry(outline)])
    }

    /// `ClosedAmpersandLeadMask`.
    fn lead_mask(&self) -> Geometry {
        let dims = self.dims();
        let bar_y = mix(dims.y_mock, 0.0, 0.4);
        let mask = self.kit.spiro_outline(vec![
            self.kit.knot(
                KnotType::Corner,
                mix(dims.x_loop_left, dims.x_terminal, 0.4),
                bar_y,
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Corner,
                mix(dims.x_loop_left, dims.x_terminal, 2.0),
                -dims.y_mock,
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Corner,
                mix(dims.x_loop_left, dims.x_terminal, 4.0),
                -dims.y_mock,
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Corner,
                mix(dims.x_loop_left, dims.x_terminal, 4.0),
                bar_y,
                Vec::new(),
            ),
        ]);

        lemmata_ptl::glyph::union(vec![
            lemmata_ptl::glyph::intersection(vec![
                self.mask_above(bar_y),
                self.mask_right(dims.x_bottom),
            ]),
            spiro_geometry(mask),
        ])
    }

    /// `ampersand.closed`.
    fn closed(&self) -> Geometry {
        let metrics = self.metrics;
        let dims = self.dims();
        let sw = dims.sw;
        let lead = self.lead(sw, dims.tip_sw);
        let lead_mask = self.lead_mask();
        let incised = lemmata_ptl::glyph::difference(vec![
            lemmata_ptl::glyph::intersection(vec![
                lead.clone(),
                self.lead_mask2(sw, dims.incised_sw),
            ]),
            lead_mask.clone(),
        ]);

        let params = ArchParams {
            superness: self.design.superness,
            tan_slope: metrics.tan_slope,
            hv_contrast: metrics.hv_contrast,
            o: metrics.o,
            correction_o_mid_x: metrics.correction_o_mid_x,
        };
        let loop_right = UserControlKnot {
            ty: KnotType::G4,
            x: dims.x_loop_right,
            y: dims.y_loop_right,
            af: Vec::new(),
        };
        let loop_left = UserControlKnot {
            ty: KnotType::G4,
            x: dims.x_loop_left,
            y: dims.y_loop_left,
            af: Vec::new(),
        };
        let y_smooth = shapes::ysmooth_mid_l(
            0.66 * dims.y_loop_right + metrics.half_stroke,
            metrics.o,
            Some(metrics.arch_depth_a),
            Some(metrics.arch_depth_b),
            metrics.arch_depth_a,
            metrics.arch_depth_b,
        );
        let mut body = vec![
            self.kit.pair(
                Family::G4,
                Direction::Left,
                Position::Mid,
                dims.x_bottom,
                metrics.o,
                self.kit.widths_rhs_heading(Some(sw), dims.slope),
            ),
            self.kit.archv(None, None),
            self.kit
                .knot(KnotType::G4, metrics.sb + metrics.ox, y_smooth, Vec::new()),
            self.kit
                .also_thru(0.5, 0.5, self.kit.widths_center(Some(sw))),
            self.kit.knot(
                KnotType::G4,
                dims.x_loop_right,
                dims.y_loop_right,
                self.kit.widths_lhs(Some(sw)),
            ),
        ];
        body.extend(shapes::arch(
            self.kit,
            &ArchArgs {
                y: metrics.cap,
                p: None,
                sw,
                compact: false,
                o: metrics.o,
                angle_pre: None,
                angle_post: None,
                blend_pre: None,
                blend_post: None,
                sw_before: None,
                sw_after: None,
                lhs: true,
            },
            &loop_right,
            &loop_left,
            &params,
        ));
        body.push(
            self.kit
                .knot(KnotType::G4, dims.x_loop_left, dims.y_loop_left, Vec::new()),
        );
        body.push(self.kit.knot(
            KnotType::Left,
            mix(dims.x_loop_left, dims.x_terminal, 0.4),
            mix(dims.y_mock, 0.0, 0.4),
            Vec::new(),
        ));
        body.push(self.kit.knot(
            KnotType::Right,
            mix(dims.x_loop_left, dims.x_terminal, 2.0),
            -dims.y_mock,
            Vec::new(),
        ));

        let tail = lemmata_ptl::glyph::difference(vec![
            self.kit.dispiro(body).geometry(kurbo::Affine::IDENTITY),
            self.rect(
                0.0,
                -2.0 * metrics.cap,
                dims.x_terminal,
                2.0 * metrics.width,
            ),
        ]);

        lemmata_ptl::glyph::union(vec![
            lemmata_ptl::glyph::intersection(vec![lead, lead_mask]),
            incised,
            tail,
        ])
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        vec![("ampersand.closed".to_string(), self.closed())]
    }
}

fn spiro_geometry(outline: lemmata_ptl::spiro_kit::SpiroOutline) -> Geometry {
    Geometry::Spiro {
        gizmo: kurbo::Affine::IDENTITY,
        closed: outline.closed,
        knots: outline.knots,
    }
}
