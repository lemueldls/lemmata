//! The question mark and exclamation from `symbol/punctuation/emotion.ptl`.
//!
//! Ported: the question's smooth hook in its round and square dot forms, and
//! the exclamation in both dot forms. The straight (`corner`) and hooked
//! (`cornerFlatHooked`) question hooks, the reversed and down questions, the
//! interrobang and the turned exclamation forms are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    metrics::Metrics,
    shapes::{self, Design, HookArgs, HookParams, downward},
    spiro_kit::{Direction, Family, Kit, Position},
};

/// The question marks this module builds.
pub struct Question<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl Question<'_> {
    fn emotion_bottom(&self) -> f64 {
        let metrics = self.metrics;

        (metrics.cap * 0.3).max(metrics.dot_size * 1.5)
    }

    /// `EssQuestion`.
    fn ess_question(&self) -> f64 {
        self.metrics.stroke * self.metrics.ess_ratio_question
    }

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

    /// `SmoothShape`.
    fn smooth(&self, reverse: bool) -> Geometry {
        let metrics = self.metrics;
        let top = metrics.cap;
        let bottom = self.emotion_bottom();
        let left = metrics.sb;
        let right = metrics.right_sb;
        let middle = metrics.middle;
        let stroke = metrics.stroke;
        let cor = (1.0 + metrics.tan_slope * metrics.tan_slope).sqrt();
        let end_x = if reverse {
            middle + metrics.half_stroke * metrics.hv_contrast * cor
        } else {
            middle - metrics.half_stroke * metrics.hv_contrast * cor
        };
        let end_af = if reverse {
            self.kit
                .widths_rhs_heading(Some(stroke * cor), downward(metrics.hv_contrast))
        } else {
            self.kit
                .widths_lhs_heading(Some(stroke * cor), downward(metrics.hv_contrast))
        };
        let widths = if reverse {
            self.kit.widths_lhs(Some(stroke))
        } else {
            self.kit.widths_rhs(Some(stroke))
        };

        let (side, other) = if reverse {
            (right, left)
        } else {
            (left, right)
        };
        let terminal = lemmata_ptl::control::UserControlKnot {
            ty: KnotType::G4,
            x: side,
            y: top - metrics.hook,
            af: Vec::new(),
        };
        let straight = lemmata_ptl::control::UserControlKnot {
            ty: KnotType::G4,
            x: other,
            y: top
                - if reverse {
                    metrics.arch_depth_a * 0.9
                } else {
                    metrics.arch_depth_b * 0.9
                },
            af: Vec::new(),
        };
        let lemmata_ptl::shapes::HookOut {
            items: hook_items,
            terminal,
        } = shapes::hook_start(
            self.kit,
            &terminal,
            &straight,
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

        let mut items = vec![
            lemmata_ptl::control::Item::Af(widths),
            self.kit
                .knot(terminal.ty, terminal.x, terminal.y, terminal.af),
        ];
        items.extend(hook_items);
        items.push(
            self.kit
                .knot(straight.ty, straight.x, straight.y, straight.af),
        );
        items.push(self.kit.also_thru_them(
            vec![(0.5, 0.45, 0.0)],
            self.kit.widths_center(Some(self.ess_question())),
            KnotType::G2,
        ));
        let direction = if reverse {
            Direction::Right
        } else {
            Direction::Down
        };
        items.push(self.kit.pair(
            Family::Straight,
            direction,
            Position::End,
            end_x,
            bottom,
            end_af,
        ));

        self.kit.dispiro(items).geometry(kurbo::Affine::IDENTITY)
    }

    /// One question mark, body and dot.
    fn question(&self, square: bool) -> Geometry {
        Geometry::Combine(vec![
            self.smooth(false),
            dot(self.kit, self.metrics, square),
        ])
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        vec![
            ("question.round.smooth".to_string(), self.question(false)),
            ("question.square.smooth".to_string(), self.question(true)),
        ]
    }
}

/// `DrawAt`: the period dot the punctuation shares.
fn dot(kit: &Kit, metrics: &Metrics, square: bool) -> Geometry {
    let middle = metrics.middle;
    let k = if square {
        metrics.square_dot_scalar
    } else {
        1.0
    };
    let centre = metrics.dot_radius * k;
    let radius = centre - if square { 0.0 } else { metrics.o };

    if square {
        spiro_geometry(shapes::rect(
            kit,
            centre + radius,
            centre - radius,
            middle - radius,
            middle + radius,
        ))
    } else {
        spiro_geometry(shapes::dot_at(kit, metrics, middle, centre, radius))
    }
}

/// The exclamation marks this module builds.
pub struct Exclam<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl Exclam<'_> {
    fn emotion_bottom(&self) -> f64 {
        let metrics = self.metrics;

        (metrics.cap * 0.3).max(metrics.dot_size * 1.5)
    }

    /// One exclamation, stem and dot.
    fn exclam(&self, square: bool) -> Geometry {
        let metrics = self.metrics;
        let bottom = (self.emotion_bottom() * 1.18).clamp(0.0, metrics.cap * 0.4);

        Geometry::Combine(vec![
            shapes::vbar_m(
                self.kit,
                metrics,
                metrics.middle,
                bottom,
                metrics.cap,
                metrics.stroke,
            )
            .geometry(kurbo::Affine::IDENTITY),
            dot(self.kit, metrics, square),
        ])
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        vec![
            ("exclam.round".to_string(), self.exclam(false)),
            ("exclam.square".to_string(), self.exclam(true)),
        ]
    }
}

fn spiro_geometry(outline: lemmata_ptl::spiro_kit::SpiroOutline) -> Geometry {
    Geometry::Spiro {
        gizmo: kurbo::Affine::IDENTITY,
        closed: outline.closed,
        knots: outline.knots,
    }
}
