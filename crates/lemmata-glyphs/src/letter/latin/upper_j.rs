//! The capital `J` from `letter/latin/upper-j.ptl`.
//!
//! Five bodies (bent hook, flat hook, descending bent hook, and the two
//! descending flat hooks) over the top serifs they select. The `JCurlyTail`
//! forms and `mathbb/J` are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    control::{Item, UserControlKnot},
    letter_model::DivFrame,
    metrics::Metrics,
    shapes::{self, Design, HSerifSide, HookArgs, HookParams, downward},
    spiro_kit::Kit,
    util::mix,
};

/// The body the config selects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    BentHook,
    FlatHook,
    DescendingBentHook,
    DescendingFlatHookSerifless,
    DescendingFlatHookSerifed,
}

/// The top serif the config adds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serif {
    None,
    Left,
    Both,
    Symmetric,
}

/// The capital `J` families this module builds.
pub struct UpperJ<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl UpperJ<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    fn df(&self) -> DivFrame {
        DivFrame::new(self.metrics.clone(), Some(1.0), None, None)
    }

    fn arch_depth_a(&self) -> f64 {
        self.metrics.arch_depth_a_of(self.metrics.arch_depth, None)
    }

    fn arch_depth_b(&self) -> f64 {
        self.metrics.arch_depth_b_of(self.metrics.arch_depth, None)
    }

    /// `IBalance`: the symmetric serif's share of the space.
    fn i_balance(&self, df: &DivFrame) -> f64 {
        df.adws() * df.adws() * (self.metrics.long_jut * 0.04)
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

    /// `JBentHookBase`.
    fn bent_hook(&self, df: &DivFrame, top: f64) -> Geometry {
        let metrics = self.metrics;
        let x_bar_right = df.right_sb() - metrics.j_balance2;
        let sw = metrics.stroke.min(0.4 * (x_bar_right - df.left_sb()));
        let hook_x = 0.75 * df.left_sb();
        let curve = UserControlKnot {
            ty: KnotType::Right,
            x: x_bar_right,
            y: self.arch_depth_b(),
            af: Vec::new(),
        };
        let end = UserControlKnot {
            ty: KnotType::G4,
            x: hook_x,
            y: metrics.hook,
            af: Vec::new(),
        };
        let hook = shapes::hook_end(
            self.kit,
            &curve,
            &end,
            &HookArgs {
                y_ref: 0.0,
                sw,
                sw_terminal: sw,
                is_tail: false,
                suppress_swash: false,
                overshoot: metrics.o,
            },
            &self.hook_params(),
        );
        let mut items = vec![
            Item::Af(self.kit.widths_rhs(Some(sw))),
            self.kit.knot(
                KnotType::Left,
                x_bar_right,
                top,
                self.kit.heading(downward(metrics.hv_contrast)),
            ),
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

    /// `JFlatHookBase`.
    fn flat_hook(&self, df: &DivFrame, df_hook: &DivFrame, top: f64) -> Geometry {
        let metrics = self.metrics;
        let x_bar_right = df.right_sb() - metrics.j_balance2;
        let sw = metrics.stroke.min(0.4 * (x_bar_right - df.left_sb()));
        let hd = shapes::flat_hook_depth(metrics, df_hook.adws());
        let reach = (0.625 * (df.right_sb() - df.left_sb())).min(1.375 * hd.x);

        self.kit
            .dispiro(vec![
                self.kit.knot(
                    KnotType::Left,
                    x_bar_right,
                    top,
                    self.kit
                        .widths_rhs_heading(Some(sw), downward(metrics.hv_contrast)),
                ),
                self.kit.knot(
                    KnotType::Right,
                    x_bar_right,
                    metrics.half_stroke + 1.375 * hd.y,
                    Vec::new(),
                ),
                self.kit.arcvh(None, Some(self.design.tight_hook_superness)),
                self.kit.knot(
                    KnotType::Left,
                    x_bar_right - reach,
                    0.0,
                    self.kit.widths_rhs(Some(metrics.stroke)),
                ),
                self.kit
                    .knot(KnotType::Right, df.left_sb(), 0.0, Vec::new()),
            ])
            .geometry(kurbo::Affine::IDENTITY)
    }

    /// `JDescendingBentHookBase`.
    fn descending_bent_hook(&self, df: &DivFrame, top: f64) -> Geometry {
        let metrics = self.metrics;
        let center = df.middle() + metrics.j_balance + self.hsw_to_v(metrics.half_stroke);
        let hook_x = center - df.width * 0.5 - self.hsw_to_v(metrics.stroke) + metrics.ox_hook;
        let curve = UserControlKnot {
            ty: KnotType::Right,
            x: center,
            y: metrics.descender + self.arch_depth_a(),
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
                sw_terminal: metrics.stroke,
                is_tail: false,
                suppress_swash: false,
                overshoot: metrics.o,
            },
            &self.hook_params(),
        );
        let mut items = vec![
            Item::Af(self.kit.widths_rhs(None)),
            self.kit.knot(
                KnotType::Left,
                center,
                top,
                self.kit.heading(downward(metrics.hv_contrast)),
            ),
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

    /// `JDescendingFlatHookSeriflessBase` and `...SerifedBase`.
    fn descending_flat_hook(
        &self,
        df: &DivFrame,
        df_hook: &DivFrame,
        top: f64,
        serif: bool,
    ) -> Geometry {
        let cross_left = if serif {
            mix(0.0, df.left_sb(), mix(1.0, df.adws(), 2.0))
        } else {
            df.middle()
                - (df_hook.middle() - mix(0.0, df_hook.left_sb(), mix(1.0, df_hook.adws(), 2.0)))
        };
        let bar_center = if serif {
            df.middle() + self.metrics.j_balance * df.adws()
        } else {
            df.middle() + 0.25 * self.metrics.j_balance * df.adws() * mix(1.0, df.adws(), 2.0)
        };

        shapes::flat_hook_dotless_j(
            self.kit,
            self.metrics,
            self.design,
            df,
            df_hook,
            top,
            bar_center,
            cross_left,
            serif,
        )
    }

    /// `jTopSerifAttach` x for a body.
    fn attach_x(&self, body: Body, df: &DivFrame) -> f64 {
        let metrics = self.metrics;
        match body {
            Body::BentHook | Body::FlatHook => {
                df.right_sb() - metrics.j_balance2 - self.hsw_to_v(metrics.half_stroke)
            }
            Body::DescendingBentHook => df.middle() + metrics.j_balance,
            Body::DescendingFlatHookSerifless | Body::DescendingFlatHookSerifed => {
                df.middle() + metrics.j_balance * df.adws()
            }
        }
    }

    /// The top serif the config selects.
    fn serif(&self, df: &DivFrame, serif: Serif, x: f64, top: f64) -> Option<Geometry> {
        let metrics = self.metrics;
        let left = || {
            shapes::hserif(
                self.kit,
                metrics,
                HSerifSide::LeftTop,
                x,
                top,
                metrics.long_jut,
                metrics.stroke,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY)
        };
        let right = || {
            shapes::hserif(
                self.kit,
                metrics,
                HSerifSide::RightTop,
                x,
                top,
                metrics.jut,
                metrics.stroke,
                None,
            )
            .geometry(kurbo::Affine::IDENTITY)
        };

        Some(match serif {
            Serif::None => return None,
            Serif::Left => left(),
            Serif::Both => Geometry::Combine(vec![left(), right()]),
            Serif::Symmetric => {
                shapes::hserif_mt(
                    self.kit,
                    metrics,
                    x - self.i_balance(df),
                    top,
                    metrics.mid_jut_center,
                    None,
                )
                .geometry(kurbo::Affine::IDENTITY)
            }
        })
    }

    /// One `J.<suffix>`.
    #[must_use]
    pub fn glyph(&self, body: Body, serif: Serif) -> Geometry {
        let df = self.df();
        let df_hook = self.df();
        let top = self.metrics.cap;
        let mut parts = vec![match body {
            Body::BentHook => self.bent_hook(&df, top),
            Body::FlatHook => self.flat_hook(&df, &df_hook, top),
            Body::DescendingBentHook => self.descending_bent_hook(&df, top),
            Body::DescendingFlatHookSerifless => {
                self.descending_flat_hook(&df, &df_hook, top, false)
            }
            Body::DescendingFlatHookSerifed => self.descending_flat_hook(&df, &df_hook, top, true),
        }];
        if let Some(serif) = self.serif(&df, serif, self.attach_x(body, &df), top) {
            parts.push(serif);
        }

        Geometry::Combine(parts)
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        CONFIGS
            .iter()
            .map(|(suffix, body, serif)| (format!("J.{suffix}"), self.glyph(*body, *serif)))
            .collect()
    }
}

/// The config table, in `Object.entries` order.
pub const CONFIGS: &[(&str, Body, Serif)] = &[
    ("bentHookSerifless", Body::BentHook, Serif::None),
    ("bentHookSerifed", Body::BentHook, Serif::Left),
    ("bentHookSerifedBothSides", Body::BentHook, Serif::Both),
    ("bentHookSerifedSymmetric", Body::BentHook, Serif::Symmetric),
    ("flatHookSerifless", Body::FlatHook, Serif::None),
    ("flatHookSerifed", Body::FlatHook, Serif::Left),
    ("flatHookSerifedBothSides", Body::FlatHook, Serif::Both),
    ("flatHookSerifedSymmetric", Body::FlatHook, Serif::Symmetric),
    (
        "descendingBentHookSerifless",
        Body::DescendingBentHook,
        Serif::None,
    ),
    (
        "descendingBentHookSerifed",
        Body::DescendingBentHook,
        Serif::Left,
    ),
    (
        "descendingBentHookSerifedBothSides",
        Body::DescendingBentHook,
        Serif::Both,
    ),
    (
        "descendingBentHookSerifedSymmetric",
        Body::DescendingBentHook,
        Serif::Symmetric,
    ),
    (
        "descendingFlatHookSerifless",
        Body::DescendingFlatHookSerifless,
        Serif::None,
    ),
    (
        "descendingFlatHookSerifed",
        Body::DescendingFlatHookSerifed,
        Serif::None,
    ),
];
