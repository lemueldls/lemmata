//! The letters `V` and `v` from `letter/latin/v.ptl`.
//!
//! Ported: the two standard bodies (straight bar and curly) with the four
//! serif settings, and the two cursive forms. `VHookRight`, `vHookRight`,
//! `vLoop`, the palatal hooks, the Cyrillic forms and the blackboard forms are
//! not ported; the hooked ones need contour ejection by tag.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::control::Item;
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::serif::SerifFrame;
use lemmata_ptl::shapes::{self, Design, HSerifSide, downward};
use lemmata_ptl::spiro_kit::{Direction, Family, Kit, Position};
use lemmata_ptl::util::mix;

/// `DesignParameters.straightVShapeSbShrink`.
const STRAIGHT_V_SHAPE_SB_SHRINK: f64 = 0.8;

/// The standard serif setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serif {
    /// `VAutoSerifs`: no serifs unless the slab level asks for them.
    Auto,
    None,
    Motion,
    Full,
}

/// The `V` and `v` families this module builds.
pub struct LowerV<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerV<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    /// `VCornerHalfWidth`.
    fn corner_half_width(&self) -> f64 {
        self.hsw_to_v(1.15 * self.metrics.half_stroke)
    }

    /// `VShapeFine`.
    fn shape_fine(&self) -> f64 {
        let fine = self.metrics.stroke
            * if self.metrics.slab == 0.0 {
                self.metrics.vtipfine
            } else {
                self.metrics.vtipfine_slab
            };

        fine.max(self.corner_half_width() * 1.2)
    }

    /// `VShapeFineStraight`.
    fn shape_fine_straight(&self) -> f64 {
        1.15 * self.metrics.stroke * self.metrics.c_thin
    }

    /// `VShapeMiddleWidth`.
    fn shape_middle_width(&self) -> f64 {
        mix(self.metrics.stroke, self.shape_fine_straight(), 2.0 / 3.0)
            .min(self.metrics.advice_stroke(3.0, 1.0, 1.0))
    }

    /// `VStraightSbShrink`.
    fn straight_sb_shrink(&self) -> f64 {
        let blend = self.metrics.stroke_width_blend(1.0, 0.75, None);

        mix(
            1.0,
            STRAIGHT_V_SHAPE_SB_SHRINK * blend,
            if self.metrics.slab == 0.0 { 1.0 } else { 0.75 },
        )
    }

    /// `VStraightSbOffset`.
    fn straight_sb_offset(&self) -> f64 {
        self.metrics.ox * self.metrics.stroke_width_blend(1.0, 2.0, None)
    }

    /// `VShapeTopFlat`.
    fn top_flat(&self, top: f64) -> f64 {
        (top * 0.9).min(
            top - if self.metrics.slab == 0.0 {
                0.0
            } else {
                self.metrics.stroke
            },
        )
    }

    /// `VBottomCapShape`.
    fn bottom_cap(&self, df: &DivFrame, straight: bool, sw: f64) -> Geometry {
        let metrics = self.metrics;
        let corner = self.corner_half_width() * (sw / metrics.stroke);
        let height = sw.min(if straight {
            self.shape_fine_straight()
        } else {
            self.shape_fine()
        });

        spiro_geometry(self.kit.spiro_outline(vec![
            self.kit.knot(KnotType::Corner, df.middle() + corner, 0.0, Vec::new()),
            self.kit.knot(KnotType::Corner, df.middle() - corner, 0.0, Vec::new()),
            self.kit.knot(KnotType::Corner, df.middle(), height, Vec::new()),
        ]))
    }

    /// `VShape`.
    fn shape(&self, df: &DivFrame, straight: bool, top: f64, sw: f64) -> Geometry {
        let metrics = self.metrics;
        let corner = self.corner_half_width() * (sw / metrics.stroke);
        let sb_scale = if straight {
            self.straight_sb_shrink()
        } else {
            1.0
        };
        let start_left = mix(0.0, df.left_sb(), sb_scale);
        let start_right = mix(df.width, df.right_sb(), sb_scale);
        let end_left = df.middle() - corner;
        let end_right = df.middle() + corner;

        let mut parts = Vec::new();
        if straight {
            let dg_cor = shapes::diag_cor(2.0 * top, df.width, Some(0.0), Some(2.0 * sw));
            let start_sw = dg_cor * sw;
            let mid_sw = dg_cor * self.shape_middle_width() * (sw / metrics.stroke);
            let end_sw = sw.min(self.shape_fine_straight() * (sw / metrics.stroke));
            let heading = downward(metrics.hv_contrast);

            for (left_side, start, end) in [
                (true, start_left, end_left),
                (false, start_right, end_right),
            ] {
                let widths = |w: f64| {
                    if left_side {
                        self.kit.widths_lhs_heading(Some(w), heading)
                    } else {
                        self.kit.widths_rhs_heading(Some(w), heading)
                    }
                };
                let items = vec![
                    self.kit.knot(
                        KnotType::Left,
                        mix(start, end, 0.0),
                        mix(top, 0.0, 0.0),
                        widths(start_sw),
                    ),
                    self.kit.knot(
                        KnotType::Right,
                        mix(start, end, 2.0 / 3.0),
                        mix(top, 0.0, 2.0 / 3.0),
                        widths(mid_sw),
                    ),
                    self.kit.knot(
                        KnotType::G4,
                        mix(start, end, 1.0),
                        mix(top, 0.0, 1.0),
                        widths(end_sw),
                    ),
                ];
                parts.push(tagged(
                    self.kit.dispiro(items).geometry(kurbo::Affine::IDENTITY),
                ));
            }
        } else {
            let fine = sw.min(self.shape_fine() * (sw / metrics.stroke));
            let top_flat = self.top_flat(top);
            for (left_side, start, end) in [
                (true, start_left, end_left),
                (false, start_right, end_right),
            ] {
                let mut items = vec![Item::Af(if left_side {
                    self.kit.widths_lhs(Some(sw))
                } else {
                    self.kit.widths_rhs(Some(sw))
                })];
                items.push(self.kit.knot(
                    KnotType::Left,
                    start,
                    top,
                    self.kit.heading(downward(metrics.hv_contrast)),
                ));
                items.push(self.kit.knot(
                    KnotType::Right,
                    start,
                    top_flat,
                    self.kit.heading(downward(metrics.hv_contrast)),
                ));
                items.push(self.kit.quad_controls(0.0, 0.3, 9, Vec::new()));
                items.push(self.kit.knot(
                    KnotType::G4,
                    end,
                    0.0,
                    kit_widths(self.kit, left_side, fine),
                ));
                parts.push(tagged(
                    self.kit.dispiro(items).geometry(kurbo::Affine::IDENTITY),
                ));
            }
            parts.push(tagged(self.bottom_cap(df, straight, sw)));
        }

        Geometry::Combine(parts)
    }

    /// `VSerifs`, `VMotionSerifs` and `VAutoSerifs`.
    fn serif(
        &self,
        df: &DivFrame,
        serif: Serif,
        straight: bool,
        top: f64,
        sw: f64,
    ) -> Option<Geometry> {
        let serif = match serif {
            Serif::Auto if self.metrics.slab != 0.0 => Serif::Full,
            Serif::Auto => return None,
            other => other,
        };
        let sb_scale = if straight {
            self.straight_sb_shrink()
        } else {
            1.0
        };
        let sb_offset = if straight {
            self.straight_sb_offset()
        } else {
            0.0
        };
        let left = mix(0.0, df.left_sb(), sb_scale) - sb_offset;
        let right = mix(df.width, df.right_sb(), sb_scale) + sb_offset;
        let frame = SerifFrame::new(
            self.metrics,
            top,
            0.0,
            left,
            right,
            sw,
            sw,
            df.adws(),
            2.0,
            false,
        );

        Some(match serif {
            Serif::None | Serif::Auto => return None,
            Serif::Motion => frame.lt_outer(self.kit, self.metrics),
            Serif::Full => Geometry::Combine(vec![
                frame.lt_full(self.kit, self.metrics),
                frame.rt_full(self.kit, self.metrics),
            ]),
        })
    }

    /// `VCursiveShape`.
    fn cursive(&self, k: f64, top: f64, sw: f64, serif: bool) -> Geometry {
        let metrics = self.metrics;
        let x_bar = mix(metrics.sb, metrics.width - metrics.sb, k);
        let x_arc_right = metrics.width - metrics.sb - metrics.ox;
        let y_arc_right = mix(0.0, top, 0.6) - sw * 0.2;
        let x_arc_end = mix(metrics.sb, metrics.width - metrics.sb, 0.8) + sw * 0.375;
        let y_arc_end = top - metrics.o;
        let r_in = metrics.tight_bend_inner_radius;
        let x_arc_mid = x_bar + self.hsw_to_v(sw + r_in) + metrics.correction_o_mid_x * sw;
        let y_arc_mid = metrics.o;

        let items = vec![
            Item::Af(self.kit.widths_lhs(Some(sw))),
            self.kit.knot(
                KnotType::Left,
                x_bar,
                top,
                self.kit.heading(downward(metrics.hv_contrast)),
            ),
            self.kit.knot(
                KnotType::Right,
                x_bar,
                (sw + r_in) * (1.0 + metrics.tan_slope),
                Vec::new(),
            ),
            self.kit.arcvh(None, None),
            self.kit.pair(
                Family::G2,
                Direction::Right,
                Position::Mid,
                x_arc_mid,
                y_arc_mid,
                Vec::new(),
            ),
            self.kit.archv(None, Some(2.0)),
            self.kit
                .knot(KnotType::G4, x_arc_right, y_arc_right, Vec::new()),
            self.kit
                .knot(KnotType::G4, x_arc_end, y_arc_end, Vec::new()),
        ];
        let mut parts = vec![self.kit.dispiro(items).geometry(kurbo::Affine::IDENTITY)];
        if serif {
            parts.push(
                shapes::hserif(
                    self.kit,
                    metrics,
                    HSerifSide::LeftTop,
                    x_bar,
                    top,
                    metrics.side_jut + (x_bar - metrics.sb),
                    metrics.stroke,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY),
            );
        }

        Geometry::Combine(parts)
    }

    /// One `V.<suffix>` or `v.<suffix>`.
    #[must_use]
    pub fn glyph(&self, straight: bool, serif: Serif, capital: bool) -> Geometry {
        let df = DivFrame::new(self.metrics.clone(), Some(1.0), None, None);
        let top = if capital {
            self.metrics.cap
        } else {
            self.metrics.xh
        };
        let mut parts = vec![self.shape(&df, straight, top, self.metrics.stroke)];
        if let Some(serif) = self.serif(&df, serif, straight, top, self.metrics.stroke) {
            parts.push(serif);
        }

        Geometry::Combine(parts)
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for (body, straight) in [("straight", true), ("curly", false)] {
            for (suffix, serif) in SERIFS {
                let name = if suffix.is_empty() {
                    format!("V.{body}")
                } else {
                    format!("V.{body}{suffix}")
                };
                out.push((name, self.glyph(straight, *serif, true)));
                let name = if suffix.is_empty() {
                    format!("v.{body}")
                } else {
                    format!("v.{body}{suffix}")
                };
                out.push((name, self.glyph(straight, *serif, false)));
            }
        }
        out.push((
            "v.cursiveSerifless".to_string(),
            self.cursive(1.0 / 24.0, self.metrics.xh, self.metrics.stroke, false),
        ));
        out.push((
            "v.cursiveSerifed".to_string(),
            self.cursive(1.0 / 16.0, self.metrics.xh, self.metrics.stroke, true),
        ));

        out
    }
}

/// The serif suffixes, with the capitalised form `weave` writes.
pub const SERIFS: &[(&str, Serif)] = &[
    ("", Serif::Auto),
    ("Serifless", Serif::None),
    ("MotionSerifed", Serif::Motion),
    ("Serifed", Serif::Full),
];

fn kit_widths(kit: &Kit, left: bool, width: f64) -> lemmata_ptl::control::Af {
    if left {
        kit.widths_lhs(Some(width))
    } else {
        kit.widths_rhs(Some(width))
    }
}

fn tagged(geometry: Geometry) -> Geometry {
    Geometry::Tagged(Box::new(geometry))
}

fn spiro_geometry(outline: lemmata_ptl::spiro_kit::SpiroOutline) -> Geometry {
    Geometry::Spiro {
        gizmo: kurbo::Affine::IDENTITY,
        closed: outline.closed,
        knots: outline.knots,
    }
}
