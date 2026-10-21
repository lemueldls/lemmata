//! The letters `U` and `u` from `letter/latin/u.ptl`.
//!
//! Ported: the five standard bodies (toothed, tailed, flat bottom, toothless
//! corner and toothless rounded) over the serif settings their tables select,
//! for the capital and the lower case. The hooked, short-leg, tailed-stem,
//! Greek and Cyrillic forms are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::control::{Item, UserControlKnot};
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::serif::SerifFrame;
use lemmata_ptl::shapes::{self, ArchArgs, ArchParams, Design, downward, rightward, upward};
use lemmata_ptl::spiro_kit::Kit;
use lemmata_ptl::util::mix;

/// The bowl body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Toothed,
    Tailed,
    FlatBottom,
    ToothlessCorner,
    ToothlessRounded,
}

/// The serif setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serif {
    Serifless,
    BottomRight,
    BilateralMotion,
    Motion,
    Full,
}

/// The `U` and `u` families this module builds.
pub struct UpperU<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl UpperU<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    fn df(&self) -> DivFrame {
        DivFrame::new(self.metrics.clone(), Some(1.0), None, None)
    }

    const fn arch_params(&self) -> ArchParams {
        ArchParams {
            superness: self.design.superness,
            tan_slope: self.metrics.tan_slope,
            hv_contrast: self.metrics.hv_contrast,
            o: self.metrics.o,
            correction_o_mid_x: self.metrics.correction_o_mid_x,
        }
    }

    /// `uBowl.shape`.
    #[allow(clippy::too_many_arguments)]
    fn bowl_shape(
        &self,
        df: &DivFrame,
        top: f64,
        bottom: f64,
        right: f64,
        stroke: f64,
        fine: f64,
        ada: f64,
        adb: f64,
        right_y0: f64,
    ) -> Vec<Item> {
        let metrics = self.metrics;
        let hsw = |value: f64| value * metrics.hv_contrast;
        let before = UserControlKnot {
            ty: KnotType::Right,
            x: right + hsw(fine),
            y: (bottom + ada).min(right_y0 - metrics.tiny),
            af: Vec::new(),
        };
        let after = UserControlKnot {
            ty: KnotType::Left,
            x: df.left_sb(),
            y: (bottom + adb).min(top - metrics.tiny),
            af: self.kit.widths_rhs(Some(stroke)),
        };
        let mut items = vec![
            Item::Af(self.kit.widths_rhs(Some(fine))),
            self.kit
                .knot(KnotType::Left, right + hsw(fine), right_y0, Vec::new()),
            self.kit
                .knot(before.ty, before.x, before.y, before.af.clone()),
        ];
        items.extend(shapes::arch(
            self.kit,
            &ArchArgs {
                y: bottom,
                p: None,
                sw: stroke,
                compact: false,
                o: metrics.o,
                angle_pre: None,
                angle_post: None,
                blend_pre: None,
                blend_post: None,
                sw_before: Some(fine),
                sw_after: Some(stroke),
                lhs: false,
            },
            &before,
            &after,
            &self.arch_params(),
        ));
        items.push(self.kit.knot(after.ty, after.x, after.y, after.af));
        items.push(
            self.kit.knot(
                KnotType::Right,
                df.left_sb(),
                top,
                self.kit
                    .widths_rhs_heading(Some(stroke), upward(metrics.hv_contrast)),
            ),
        );

        items
    }

    /// `uBowl.toothlessCorner`.
    #[allow(clippy::too_many_arguments, clippy::if_not_else)]
    fn toothless_corner(
        &self,
        df: &DivFrame,
        top: f64,
        bottom: f64,
        right: f64,
        stroke: f64,
        ada: f64,
        adb: f64,
        rise: f64,
    ) -> Vec<Item> {
        let metrics = self.metrics;
        let before = UserControlKnot {
            ty: KnotType::Right,
            x: df.left_sb(),
            y: (bottom + adb).min(top - metrics.tiny),
            af: Vec::new(),
        };
        let mut items = vec![
            self.kit.knot(
                KnotType::Left,
                df.left_sb(),
                top,
                self.kit
                    .widths_lhs_heading(Some(stroke), downward(metrics.hv_contrast)),
            ),
            self.kit
                .knot(before.ty, before.x, before.y, before.af.clone()),
        ];
        if rise != 0.0 {
            let after = UserControlKnot {
                ty: KnotType::G4,
                x: right,
                y: bottom + rise,
                af: self.kit.widths_lhs(Some(stroke)),
            };
            items.extend(shapes::arch(
                self.kit,
                &ArchArgs {
                    y: bottom,
                    p: None,
                    sw: stroke,
                    compact: false,
                    o: metrics.o,
                    angle_pre: None,
                    angle_post: None,
                    blend_pre: None,
                    blend_post: Some(false),
                    sw_before: None,
                    sw_after: Some(stroke),
                    lhs: true,
                },
                &before,
                &after,
                &self.arch_params(),
            ));
            items.push(self.kit.knot(after.ty, after.x, after.y, after.af));
        } else {
            items.push(self.kit.arcvh(None, None));
            items.push(self.kit.knot(
                KnotType::Left,
                mix(df.left_sb(), right, 0.5) + metrics.correction_o_mid_x * stroke,
                bottom,
                Vec::new(),
            ));
            items.push(self.kit.knot(
                KnotType::Right,
                right - metrics.tiny,
                bottom,
                self.kit.heading(rightward()),
            ));
        }
        let _ = ada;

        items
    }

    /// `uBowl.toothlessRounded`.
    #[allow(clippy::too_many_arguments)]
    fn toothless_rounded(
        &self,
        df: &DivFrame,
        top: f64,
        bottom: f64,
        right: f64,
        stroke: f64,
        ada: f64,
        adb: f64,
        right_y0: f64,
    ) -> Vec<Item> {
        let metrics = self.metrics;
        let before = UserControlKnot {
            ty: KnotType::Right,
            x: df.left_sb(),
            y: (bottom + adb).min(top - metrics.tiny),
            af: Vec::new(),
        };
        let after = UserControlKnot {
            ty: KnotType::Left,
            x: right,
            y: (bottom + ada).min(right_y0 - metrics.tiny),
            af: Vec::new(),
        };
        let mut items = vec![
            self.kit.knot(
                KnotType::Left,
                df.left_sb(),
                top,
                self.kit
                    .widths_lhs_heading(Some(stroke), downward(metrics.hv_contrast)),
            ),
            self.kit
                .knot(before.ty, before.x, before.y, before.af.clone()),
        ];
        items.extend(shapes::arch(
            self.kit,
            &ArchArgs {
                y: bottom,
                p: None,
                sw: stroke,
                compact: false,
                o: metrics.o,
                angle_pre: None,
                angle_post: None,
                blend_pre: None,
                blend_post: None,
                sw_before: None,
                sw_after: Some(stroke),
                lhs: true,
            },
            &before,
            &after,
            &self.arch_params(),
        ));
        items.push(self.kit.knot(after.ty, after.x, after.y, after.af));
        items.push(self.kit.knot(
            KnotType::Right,
            right,
            right_y0,
            self.kit.heading(upward(metrics.hv_contrast)),
        ));

        items
    }

    /// One body, without the serif.
    fn body(&self, df: &DivFrame, body: Body, top: f64, capital: bool) -> Geometry {
        let metrics = self.metrics;
        let stroke = metrics.stroke;
        let fine = metrics.shoulder_fine;
        let (ada, adb) = if capital {
            (
                metrics.arch_depth_a_of(metrics.arch_depth, None),
                metrics.arch_depth_b_of(metrics.arch_depth, None),
            )
        } else {
            (
                metrics.arch_depth_a_of(metrics.small_arch_depth, None),
                metrics.arch_depth_b_of(metrics.small_arch_depth, None),
            )
        };
        let right = df.right_sb();
        let mut parts = Vec::new();
        match body {
            Body::Toothed => {
                parts.push(
                    self.kit
                        .dispiro(self.bowl_shape(
                            df,
                            top,
                            0.0,
                            right - self.hsw_to_v(stroke),
                            stroke,
                            fine,
                            ada,
                            adb,
                            (ada + metrics.tiny).min(top),
                        ))
                        .geometry(kurbo::Affine::IDENTITY),
                );
                parts.push(
                    shapes::vbar_r(self.kit, metrics, right, 0.0, top, stroke)
                        .geometry(kurbo::Affine::IDENTITY),
                );
            }
            Body::Tailed => {
                parts.push(
                    self.kit
                        .dispiro(self.bowl_shape(
                            df,
                            top,
                            0.0,
                            right - self.hsw_to_v(stroke),
                            stroke,
                            fine,
                            ada,
                            adb,
                            (ada + metrics.tiny).min(top),
                        ))
                        .geometry(kurbo::Affine::IDENTITY),
                );
                parts.push(
                    shapes::rightward_tailed_bar(self.kit, metrics, df, right, 0.0, top, stroke)
                        .geometry(kurbo::Affine::IDENTITY),
                );
            }
            Body::FlatBottom => {
                parts.push(
                    self.kit
                        .dispiro(self.toothless_corner(df, top, 0.0, right, stroke, ada, adb, 0.0))
                        .geometry(kurbo::Affine::IDENTITY),
                );
                parts.push(
                    shapes::vbar_r(self.kit, metrics, right, 0.0, top, stroke)
                        .geometry(kurbo::Affine::IDENTITY),
                );
            }
            Body::ToothlessCorner => {
                parts.push(
                    self.kit
                        .dispiro(self.toothless_corner(
                            df,
                            top,
                            0.0,
                            right,
                            stroke,
                            ada,
                            adb,
                            metrics.d_toothless_rise,
                        ))
                        .geometry(kurbo::Affine::IDENTITY),
                );
                parts.push(
                    shapes::vbar_r(
                        self.kit,
                        metrics,
                        right,
                        metrics.d_toothless_rise,
                        top,
                        stroke,
                    )
                    .geometry(kurbo::Affine::IDENTITY),
                );
            }
            Body::ToothlessRounded => {
                parts.push(
                    self.kit
                        .dispiro(self.toothless_rounded(df, top, 0.0, right, stroke, ada, adb, top))
                        .geometry(kurbo::Affine::IDENTITY),
                );
            }
        }

        Geometry::Combine(parts)
    }

    /// `USerifs`.
    fn serif(
        &self,
        df: &DivFrame,
        body: Body,
        serif: Serif,
        top: f64,
        capital: bool,
    ) -> Option<Geometry> {
        if serif == Serif::Serifless {
            return None;
        }
        let metrics = self.metrics;
        let sf = SerifFrame::from_df(metrics, df, top, 0.0, Some(metrics.stroke), None, false);
        let small = !capital;
        let bottom_right = || sf.rb_outer(self.kit, metrics);
        let full = || {
            if small {
                Geometry::Combine(vec![
                    sf.lt_outer(self.kit, metrics),
                    sf.rt_inner(self.kit, metrics),
                ])
            } else {
                Geometry::Combine(vec![
                    sf.lt_full(self.kit, metrics),
                    sf.rt_full(self.kit, metrics),
                ])
            }
        };
        let toothed = || Geometry::Combine(vec![full(), bottom_right()]);
        let motion_toothed =
            || Geometry::Combine(vec![sf.lt_outer(self.kit, metrics), bottom_right()]);
        let motion_toothless = || sf.lt_outer(self.kit, metrics);
        let bilateral = || {
            Geometry::Combine(vec![
                sf.lt_outer(self.kit, metrics),
                sf.rt_outer(self.kit, metrics),
            ])
        };

        Some(match serif {
            Serif::Serifless => return None,
            Serif::BottomRight => bottom_right(),
            Serif::BilateralMotion => bilateral(),
            Serif::Motion => {
                if matches!(body, Body::Toothed | Body::FlatBottom) {
                    motion_toothed()
                } else {
                    motion_toothless()
                }
            }
            Serif::Full => {
                if matches!(body, Body::Toothed | Body::FlatBottom) {
                    toothed()
                } else {
                    full()
                }
            }
        })
    }

    /// One `U.<suffix>` or `u.<suffix>`.
    #[must_use]
    pub fn glyph(&self, body: Body, serif: Serif, capital: bool) -> Geometry {
        let df = self.df();
        let top = if capital {
            self.metrics.cap
        } else {
            self.metrics.xh
        };
        let mut parts = vec![self.body(&df, body, top, capital)];
        if let Some(serif) = self.serif(&df, body, serif, top, capital) {
            parts.push(serif);
        }

        Geometry::Combine(parts)
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for (body_key, body) in BODIES {
            for (suffix, serif) in SERIFS {
                out.push((
                    format!("U.{body_key}{suffix}"),
                    self.glyph(*body, *serif, true),
                ));
                out.push((
                    format!("u.{body_key}{suffix}"),
                    self.glyph(*body, *serif, false),
                ));
            }
        }

        out
    }
}

/// The body suffixes, in the table's order.
pub const BODIES: &[(&str, Body)] = &[
    ("toothed", Body::Toothed),
    ("tailed", Body::Tailed),
    ("flatBottom", Body::FlatBottom),
    ("toothlessCorner", Body::ToothlessCorner),
    ("toothlessRounded", Body::ToothlessRounded),
];

/// The serif suffixes.
pub const SERIFS: &[(&str, Serif)] = &[
    ("Serifless", Serif::Serifless),
    ("BottomRightSerifed", Serif::BottomRight),
    ("BilateralMotionSerifed", Serif::BilateralMotion),
    ("MotionSerifed", Serif::Motion),
    ("Serifed", Serif::Full),
];
