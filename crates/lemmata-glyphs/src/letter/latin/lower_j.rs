//! The lower-case `j` bodies from `letter/latin/lower-j.ptl`.
//!
//! Ported: the 15 `dotlessj` configs and their `dotlessjBar` forms, plus the
//! two `dotlessjCurlyTail` forms. The `dotlessjBarHookTop` forms need
//! `TopHook.toRight.mBarOuter`, which is not ported; `mathbb/dotlessj` and the
//! compositions (`j`, `jBar`, `jCurlyTail`) need the tittle and are not here.
//!
//! The composed `j` is the dotless body plus `tittleAbove`, and the tittle's
//! placement is the open composition question, so only the dotless bodies are
//! registered.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops, clippy::similar_names)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    control::{Item, UserControlKnot},
    letter_model::DivFrame,
    metrics::Metrics,
    shapes::{self, Design, HSerifSide, HookArgs, HookParams, downward},
    spiro_kit::Kit,
    util::mix,
};

/// `advanceScaleI`, `advanceScaleII` and `advanceScaleF` are all 1 for the
/// reference plan, so every frame below is the glyph's own width.
const ADVANCE_SCALE: f64 = 1.0;

/// The strand body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    BentHook,
    NarrowBentHook,
    Straight,
    FlatHook,
    DiagonalTailed,
}

/// The top-left serif.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serif {
    None,
    Short,
    Long,
}

/// The lower-case `j` families this module builds.
pub struct LowerJ<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerJ<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    fn right_sb(&self) -> f64 {
        self.metrics.width - self.metrics.sb
    }

    fn df(&self) -> DivFrame {
        DivFrame::new(self.metrics.clone(), Some(ADVANCE_SCALE), None, None)
    }

    /// `IBalance2`, the fallback when the plan does not carry one.
    fn i_balance2(&self) -> f64 {
        self.metrics.long_jut * 0.14
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

    /// `XMiddleT`, the x the body and its serif are centred on.
    fn x_middle(&self, body: Body, serif: Serif) -> f64 {
        let metrics = self.metrics;
        let middle = metrics.middle;
        let j_balance = metrics.j_balance;
        let i_balance = self.i_balance2();
        let adws = self.df().adws();

        match (body, serif) {
            (Body::Straight, Serif::Long) => middle + i_balance,
            (Body::Straight, _) => middle,
            (Body::DiagonalTailed, _) => middle + i_balance,
            (Body::FlatHook, Serif::Long) => middle + j_balance * adws,
            (Body::FlatHook, _) => middle + 0.25 * j_balance * adws * mix(1.0, adws, 2.0),
            _ => middle + j_balance,
        }
    }

    /// `JBody.BentHook` and `NarrowBentHook`.
    fn bent_hook(&self, df: &DivFrame, top: f64, x_middle: f64, narrow: bool) -> Geometry {
        let metrics = self.metrics;
        let half = metrics.half_stroke;
        let full = (x_middle - df.middle() - self.hsw_to_v(half) + metrics.ox_hook)
            .min(x_middle - self.hsw_to_v(1.5 * metrics.stroke) + metrics.ox_hook);
        let p_adaptive = if ADVANCE_SCALE < 1.0 {
            (1.0 - df.adws()) / (1.0 - ADVANCE_SCALE)
        } else {
            0.0
        };
        let hook_x = if narrow {
            mix(x_middle, full, mix(0.8, 1.0, p_adaptive))
        } else {
            full
        };
        let sw_terminal = if narrow {
            mix(
                metrics.stroke,
                df.advice_stroke(2.75, 1.0),
                mix(0.35, 0.1, p_adaptive),
            )
        } else {
            metrics.stroke
        };
        let start = UserControlKnot {
            ty: KnotType::Left,
            x: x_middle + self.hsw_to_v(half),
            y: top,
            af: self.kit.heading(downward(metrics.hv_contrast)),
        };
        let curve = UserControlKnot {
            ty: KnotType::Right,
            x: x_middle + self.hsw_to_v(half),
            y: metrics.descender + metrics.arch_depth_a,
            af: Vec::new(),
        };
        let end = UserControlKnot {
            ty: KnotType::G4,
            x: hook_x,
            y: metrics.descender + metrics.j_hook,
            af: Vec::new(),
        };
        let hook = shapes::hook_end(
            self.kit,
            &curve,
            &end,
            &HookArgs {
                y_ref: metrics.descender,
                sw: metrics.stroke,
                sw_terminal,
                is_tail: false,
                suppress_swash: false,
                overshoot: metrics.o,
            },
            &self.hook_params(),
        );
        let mut items = vec![
            Item::Af(self.kit.widths_rhs(Some(metrics.stroke))),
            self.kit.knot(start.ty, start.x, start.y, start.af),
            self.kit.knot(curve.ty, curve.x, curve.y, curve.af),
        ];
        items.extend(hook.items);
        items.push(self.kit.knot(
            hook.terminal.ty,
            hook.terminal.x,
            hook.terminal.y,
            hook.terminal.af,
        ));

        self.kit.dispiro(items).geometry(kurbo::Affine::IDENTITY)
    }

    /// `JBody.Straight`.
    fn straight(&self, top: f64, x_middle: f64) -> Geometry {
        shapes::vbar_m(
            self.kit,
            self.metrics,
            x_middle,
            self.metrics.descender,
            top,
            self.metrics.stroke,
        )
        .geometry(kurbo::Affine::IDENTITY)
    }

    /// `FlatHookDotlessJShape`, without its optional serif.
    fn flat_hook(
        &self,
        df: &DivFrame,
        df_hook: &DivFrame,
        top: f64,
        bar_center: f64,
        cross_left: f64,
    ) -> Geometry {
        shapes::flat_hook_dotless_j(
            self.kit,
            self.metrics,
            self.design,
            df,
            df_hook,
            top,
            bar_center,
            cross_left,
            false,
        )
    }

    /// `JBody.DiagonalTailed`.
    fn diagonal_tailed(&self, df: &DivFrame, top: f64, x_middle: f64) -> Geometry {
        let metrics = self.metrics;
        let depth = shapes::diag_tail_std_depth(metrics, df.adws(), metrics.stroke);
        let mut items = vec![
            Item::Af(self.kit.widths_center(None)),
            self.kit.knot(
                KnotType::Left,
                x_middle,
                top,
                self.kit.heading(downward(metrics.hv_contrast)),
            ),
        ];
        items.extend(shapes::diag_tail_l(
            self.kit,
            metrics,
            self.design,
            x_middle,
            metrics.descender,
            depth,
            metrics.stroke,
        ));

        self.kit.dispiro(items).geometry(kurbo::Affine::IDENTITY)
    }

    /// `JSerifs`.
    fn serif(&self, df: &DivFrame, serif: Serif, top: f64, x_middle: f64) -> Option<Geometry> {
        let metrics = self.metrics;
        let length = match serif {
            Serif::None => return None,
            Serif::Short => metrics.jut,
            Serif::Long => {
                metrics.jut.max(mix(
                    self.hsw_to_v(metrics.half_stroke),
                    metrics.long_jut,
                    df.adws(),
                ))
            }
        };

        Some(
            shapes::hserif(
                self.kit,
                metrics,
                HSerifSide::LeftTop,
                x_middle,
                top,
                length,
                metrics.stroke,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY),
        )
    }

    /// The body plus the serif the config selects.
    fn shape(&self, body: Body, serif: Serif, top: f64, adws: f64) -> Geometry {
        let df = DivFrame::new(self.metrics.clone(), Some(adws), None, None);
        let x_middle = self.x_middle(body, serif);
        let mut parts = match body {
            Body::BentHook => vec![self.bent_hook(&df, top, x_middle, false)],
            Body::NarrowBentHook => vec![self.bent_hook(&df, top, x_middle, true)],
            Body::Straight => vec![self.straight(top, x_middle)],
            Body::FlatHook => {
                let df_hook = DivFrame::new(self.metrics.clone(), Some(1.0), None, None);
                let cross_left = df.middle()
                    - (df_hook.middle()
                        - mix(0.0, df_hook.left_sb(), mix(1.0, df_hook.adws(), 2.0)));
                vec![self.flat_hook(&df, &df_hook, top, x_middle, cross_left)]
            }
            Body::DiagonalTailed => vec![self.diagonal_tailed(&df, top, x_middle)],
        };
        if let Some(serif) = self.serif(&df, serif, top, x_middle) {
            parts.push(serif);
        }

        Geometry::Combine(parts)
    }

    /// `LetterBarOverlay.m`: the bar that turns the body into the barred form.
    fn bar_overlay(&self, x: f64, y: f64) -> Geometry {
        let metrics = self.metrics;
        let hsw = |value: f64| value * metrics.hv_contrast;
        let space = (0.0, metrics.width);
        let ref_sw = metrics.stroke;
        let sw = metrics.overlay_stroke;
        let left_edge = x - 0.5 * hsw(ref_sw);
        let right_edge = x + 0.5 * hsw(ref_sw);
        let p = (left_edge - space.0) / ((space.1 - right_edge) + (left_edge - space.0));
        let x_c = mix(left_edge, right_edge, mix(0.5, 1.0 - p, 0.5));
        let jut = 0.75 * metrics.long_jut;
        let left =
            (left_edge - metrics.side_jut).min((x_c - jut).max(mix(space.0, left_edge, 0.3)));
        let right =
            (right_edge + metrics.side_jut).max((x_c + jut).min(mix(space.1, right_edge, 0.3)));

        shapes::h_overlay_bar(self.kit, metrics, left, right, y, Some(sw))
            .geometry(kurbo::Affine::IDENTITY)
    }

    /// `dotlessjCurlyTail`: the straight body with `CurlyTail`.
    fn curly_tail(&self, serif: Serif) -> Geometry {
        let metrics = self.metrics;
        let df = self.df();
        let x_middle = metrics.middle + metrics.j_balance;
        let fine = metrics.advice_stroke(3.5, 1.0, 1.0);
        let before = UserControlKnot {
            ty: KnotType::Right,
            x: x_middle + self.hsw_to_v(metrics.half_stroke),
            y: metrics.descender + metrics.long_v_jut,
            af: Vec::new(),
        };
        let start = UserControlKnot {
            ty: KnotType::Left,
            x: x_middle + self.hsw_to_v(metrics.half_stroke),
            y: metrics.xh,
            af: self.kit.heading(downward(metrics.hv_contrast)),
        };
        let mut items = vec![
            Item::Af(self.kit.widths_rhs(Some(metrics.stroke))),
            self.kit.knot(start.ty, start.x, start.y, start.af),
            self.kit
                .knot(before.ty, before.x, before.y, before.af.clone()),
        ];
        items.extend(shapes::curly_tail(
            self.kit,
            metrics,
            self.design,
            &before,
            &shapes::CurlyTailArgs {
                fine,
                bottom: metrics.descender,
                x_outer: (x_middle - self.hsw_to_v(metrics.half_stroke)) - metrics.long_jut,
                x2: self.right_sb() + self.hsw_to_v(0.5 * fine) + metrics.j_balance,
                y2: Some(metrics.descender + fine / 2.0),
                y_loop_top: None,
                sw_before: metrics.stroke,
                terminal_slope_adj: 0.5,
                flat: false,
            },
        ));
        let mut parts = vec![self.kit.dispiro(items).geometry(kurbo::Affine::IDENTITY)];
        if let Some(serif) = self.serif(&df, serif, metrics.xh, x_middle) {
            parts.push(serif);
        }

        Geometry::Combine(parts)
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for (suffix, body, serif, adws) in CONFIGS {
            let glyph = self.shape(*body, *serif, self.metrics.xh, *adws);
            let x_middle = self.x_middle(*body, *serif);
            out.push((format!("dotlessj.{suffix}"), glyph.clone()));
            out.push((
                format!("dotlessjBar.{suffix}"),
                Geometry::Combine(vec![
                    glyph,
                    self.bar_overlay(x_middle, self.metrics.xh / 2.0),
                ]),
            ));
        }
        out.push((
            "dotlessjCurlyTail.bentHookSerifless".to_string(),
            self.curly_tail(Serif::None),
        ));
        out.push((
            "dotlessjCurlyTail.bentHookSerifed".to_string(),
            self.curly_tail(Serif::Long),
        ));

        out
    }
}

/// The config table, in `Object.entries` order.
pub const CONFIGS: &[(&str, Body, Serif, f64)] = &[
    ("bentHookSerifless", Body::BentHook, Serif::None, 1.0),
    ("bentHookShortSerifed", Body::BentHook, Serif::Short, 1.0),
    ("bentHookSerifed", Body::BentHook, Serif::Long, 1.0),
    (
        "narrowBentHookSerifless",
        Body::NarrowBentHook,
        Serif::None,
        1.0,
    ),
    (
        "narrowBentHookShortSerifed",
        Body::NarrowBentHook,
        Serif::Short,
        1.0,
    ),
    (
        "narrowBentHookSerifed",
        Body::NarrowBentHook,
        Serif::Long,
        1.0,
    ),
    (
        "diagonalTailedSerifless",
        Body::DiagonalTailed,
        Serif::None,
        1.0,
    ),
    (
        "diagonalTailedShortSerifed",
        Body::DiagonalTailed,
        Serif::Short,
        1.0,
    ),
    (
        "diagonalTailedSerifed",
        Body::DiagonalTailed,
        Serif::Long,
        1.0,
    ),
    ("straightSerifless", Body::Straight, Serif::None, 1.0),
    ("straightShortSerifed", Body::Straight, Serif::Short, 1.0),
    ("straightSerifed", Body::Straight, Serif::Long, 1.0),
    ("flatHookSerifless", Body::FlatHook, Serif::None, 1.0),
    ("flatHookShortSerifed", Body::FlatHook, Serif::Short, 1.0),
    ("flatHookSerifed", Body::FlatHook, Serif::Long, 1.0),
];
