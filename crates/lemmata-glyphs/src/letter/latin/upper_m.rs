//! The upper-case `M` from `letter/latin/upper-m.ptl`.
//!
//! Two side bars, two middle diagonals and an optional middle stem. The form
//! changes where the diagonals meet, the slab setting what the frames draw, and
//! `slantedSides` leans the outer bars.
//!
//! `Meng`, the descender forms, the Cyrillic bodies and the turned letters need
//! hooks or glyphs outside this file and are not ported. The anchors are not
//! either.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops, clippy::similar_names)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::serif::SerifFrame;
use lemmata_ptl::shapes::{self, Design};
use lemmata_ptl::spiro_kit::{Direction, Family, Kit, Position};
use lemmata_ptl::util::{join_camel, mix};

/// Where the middle diagonals meet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Flat,
    Hanging,
    TurnW,
    San,
    SanSmall,
}

/// The slab settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slab {
    None,
    Auto,
    Motion,
    SmallCyrillic,
    Full,
}

/// One row of the table.
#[derive(Clone, Copy, Debug)]
pub struct Variant {
    pub slanted: bool,
    pub form: Form,
    pub slab: Slab,
}

/// The table, as `(suffix, variant)` pairs.
#[must_use]
pub fn variants() -> Vec<(String, Variant)> {
    const MIDDLES: &[(&str, Form)] = &[("hanging", Form::Hanging), ("flatBottom", Form::Flat)];
    const SERIFS: &[(&str, Slab)] = &[
        ("serifless", Slab::None),
        ("motionSerifed", Slab::Motion),
        ("smallCyrl", Slab::SmallCyrillic),
        ("serifed", Slab::Full),
    ];
    const SIDES: &[(&str, bool)] = &[("", false), ("slantedSides", true)];

    let mut out = Vec::new();
    for (side_key, slanted) in SIDES {
        for (middle_key, form) in MIDDLES {
            for (serif_key, slab) in SERIFS {
                let suffix = join_camel(&join_camel(side_key, middle_key), serif_key);
                out.push((
                    suffix,
                    Variant {
                        slanted: *slanted,
                        form: *form,
                        slab: *slab,
                    },
                ));
            }
        }
    }
    out.push((
        "grekCapitalSan".to_string(),
        Variant {
            slanted: false,
            form: Form::San,
            slab: Slab::Auto,
        },
    ));
    out.push((
        "grekSmallSan".to_string(),
        Variant {
            slanted: false,
            form: Form::SanSmall,
            slab: Slab::None,
        },
    ));

    out
}

/// The upper-case `M` under construction.
pub struct UpperM<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl UpperM<'_> {
    /// The frame the definition draws in: `DivFrame para.advanceScaleM 3`.
    fn frame(&self) -> DivFrame {
        DivFrame::new(self.metrics.clone(), Some(1.0), Some(3.0), None)
    }

    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    fn middle(&self) -> f64 {
        self.frame().width / 2.0
    }

    /// `MShape`.
    #[must_use]
    pub fn shape(&self, top: f64, variant: Variant) -> Geometry {
        let metrics = self.metrics;
        let frame = self.frame();
        let df = &frame;
        let form = variant.form;
        let slab = variant.slab;
        let up = shapes::upward(metrics.hv_contrast);
        let down = shapes::downward(metrics.hv_contrast);
        let sides_slope = if variant.slanted { 0.04 } else { 0.0 };
        let x_left_top = df.sb + top * sides_slope;
        let x_right_top = (df.width - df.sb) - top * sides_slope;
        let has_bottom_inner = match slab {
            Slab::Full => true,
            Slab::Auto => metrics.slab != 0.0,
            _ => false,
        };
        let sw_side_bot = df.advice_stroke(
            match form {
                Form::SanSmall | Form::San | Form::TurnW | Form::Hanging => 2.50,
                Form::Flat => 2.75,
            },
            1.0,
        );
        let sw_side_top = df.advice_stroke(
            match form {
                Form::SanSmall | Form::San | Form::TurnW => 3.00,
                Form::Flat | Form::Hanging => 4.00,
            },
            1.0,
        );
        let sw_middle = df.advice_stroke(4.50, 1.0);
        let k_mid_hang = df.adws();
        let middle_y = match form {
            Form::SanSmall | Form::San => mix(0.0, top, 0.55),
            Form::TurnW => mix(0.0, top, 0.45),
            Form::Hanging => mix(0.0, top, 0.30 / (k_mid_hang * k_mid_hang)),
            Form::Flat => {
                if has_bottom_inner {
                    sw_side_bot
                } else {
                    0.0
                }
            }
        };
        let k_mid_shift = if form == Form::Flat { 0.75 } else { 0.50 };
        let k_mid_bot_stroke_shrink =
            mix(df.adws(), 1.0, if form == Form::Flat { 1.50 } else { 1.00 });
        let sw_middle_shrink = k_mid_bot_stroke_shrink * sw_middle;

        let left_y0 = if form == Form::SanSmall {
            metrics.descender
        } else {
            0.0
        };
        let mut parts = vec![
            Geometry::Tagged(Box::new(
                self.kit
                    .dispiro(vec![
                        self.kit.knot(
                            KnotType::Left,
                            df.sb,
                            left_y0,
                            self.kit.widths_heading(0.0, sw_side_bot, up),
                        ),
                        self.kit.knot(
                            KnotType::Right,
                            mix(df.sb, x_left_top, 0.2),
                            top * 0.2,
                            self.kit.heading(up),
                        ),
                        self.kit.pair(
                            Family::Straight,
                            Direction::Up,
                            Position::End,
                            x_left_top,
                            top,
                            self.kit.widths_heading(0.0, sw_side_top, up),
                        ),
                    ])
                    .geometry(kurbo::Affine::IDENTITY),
            )),
            Geometry::Tagged(Box::new(
                self.kit
                    .dispiro(vec![
                        self.kit.knot(
                            KnotType::Left,
                            df.width - df.sb,
                            0.0,
                            self.kit.widths_heading(sw_side_bot, 0.0, up),
                        ),
                        self.kit.knot(
                            KnotType::Right,
                            mix(df.width - df.sb, x_right_top, 0.2),
                            top * 0.2,
                            self.kit.heading(up),
                        ),
                        self.kit.pair(
                            Family::Straight,
                            Direction::Up,
                            Position::End,
                            x_right_top,
                            top,
                            self.kit.widths_heading(sw_side_top, 0.0, up),
                        ),
                    ])
                    .geometry(kurbo::Affine::IDENTITY),
            )),
            self.kit
                .dispiro(vec![
                    self.kit.knot(
                        KnotType::Left,
                        x_left_top + k_mid_shift * self.hsw_to_v(sw_side_top),
                        top,
                        self.kit.widths_lhs_heading(Some(sw_middle), down),
                    ),
                    self.kit.knot(
                        KnotType::Right,
                        self.middle() - self.hsw_to_v(0.5 * sw_middle_shrink),
                        middle_y,
                        self.kit.widths_lhs_heading(Some(sw_middle_shrink), down),
                    ),
                ])
                .geometry(kurbo::Affine::IDENTITY),
            self.kit
                .dispiro(vec![
                    self.kit.knot(
                        KnotType::Left,
                        x_right_top - k_mid_shift * self.hsw_to_v(sw_side_top),
                        top,
                        self.kit.widths_rhs_heading(Some(sw_middle), down),
                    ),
                    self.kit.knot(
                        KnotType::Right,
                        self.middle() + self.hsw_to_v(0.5 * sw_middle_shrink),
                        middle_y,
                        self.kit.widths_rhs_heading(Some(sw_middle_shrink), down),
                    ),
                ])
                .geometry(kurbo::Affine::IDENTITY),
        ];

        let force_symmetric = form != Form::Flat;
        let frame_t = SerifFrame::new(
            metrics,
            top,
            0.0,
            x_left_top,
            x_right_top,
            sw_side_bot,
            sw_side_bot,
            df.adws(),
            3.0,
            force_symmetric,
        );
        let frame_b = SerifFrame::new(
            metrics,
            top,
            0.0,
            df.sb,
            df.width - df.sb,
            sw_side_bot,
            sw_side_bot,
            df.adws(),
            3.0,
            force_symmetric,
        );
        match slab {
            Slab::None => {}
            Slab::Full => parts.extend([
                frame_t.lt_outer(self.kit, metrics),
                frame_t.rt_outer(self.kit, metrics),
                frame_b.lb_full(self.kit, metrics),
                frame_b.rb_full(self.kit, metrics),
            ]),
            Slab::Auto => {
                if metrics.slab != 0.0 {
                    parts.extend([
                        frame_t.lt_outer(self.kit, metrics),
                        frame_t.rt_outer(self.kit, metrics),
                        frame_b.lb_full(self.kit, metrics),
                        frame_b.rb_full(self.kit, metrics),
                    ]);
                }
            }
            Slab::Motion => parts.push(frame_t.lt_outer(self.kit, metrics)),
            Slab::SmallCyrillic => {
                parts.push(frame_t.lt_outer(self.kit, metrics));
                parts.push(frame_t.rt_outer(self.kit, metrics));
                parts.push(frame_b.lb_full(self.kit, metrics));
                parts.push(frame_b.rb_full(self.kit, metrics));
            }
        }

        Geometry::Combine(parts)
    }

    /// `M.<suffix>`.
    #[must_use]
    pub fn capital(&self, variant: Variant) -> Geometry {
        self.shape(self.metrics.cap, variant)
    }

    /// `smcpM.<suffix>`.
    #[must_use]
    pub fn small(&self, variant: Variant) -> Geometry {
        self.shape(self.metrics.xh, variant)
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for (suffix, variant) in variants() {
            out.push((format!("M.{suffix}"), self.capital(variant)));
            out.push((format!("smcpM.{suffix}"), self.small(variant)));
        }

        out
    }
}
