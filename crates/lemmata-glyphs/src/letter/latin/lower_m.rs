//! The lower-case `m` from `letter/latin/lower-m.ptl`.
//!
//! `m` is a left stem and two shoulders; the bodies differ in how the shoulders
//! close and how tall the middle leg is. Two of the file's five bodies are
//! ported, the arches and the flat top, with both leg heights, both terminals
//! and all five serif settings. `meng` is ported with them.
//!
//! The three earless bodies, `mCrossedTail`, the Cyrillic forms and the turned
//! family are not ported. The anchors are not either.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops, clippy::similar_names)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::control::{Item, UserControlKnot};
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::serif::SerifFrame;
use lemmata_ptl::shapes::{self, Design};
use lemmata_ptl::spiro_kit::Kit;
use lemmata_ptl::util::{join_camel, mix};

/// The two bodies ported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Arches,
    FlatTop,
    EarlessSingleArch,
    EarlessCornerDoubleArch,
    EarlessRoundedDoubleArch,
}

/// The five serif settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serif {
    Serifless,
    Serifed,
    TopLeftSerifed,
    BottomRightSerifed,
    TopLeftAndBottomRightSerifed,
}

/// One row of the table.
#[derive(Clone, Copy, Debug)]
pub struct Variant {
    pub body: Body,
    pub short_leg: bool,
    pub tailed: bool,
    pub serif: Serif,
}

/// The bodies, in the table's order.
pub const BODIES: &[(&str, Body)] = &[
    ("", Body::Arches),
    ("flatTop", Body::FlatTop),
    ("earlessSingleArch", Body::EarlessSingleArch),
    ("earlessCornerDoubleArch", Body::EarlessCornerDoubleArch),
    ("earlessRoundedDoubleArch", Body::EarlessRoundedDoubleArch),
];

/// The leg heights, in the table's order.
pub const LEGS: &[(&str, bool)] = &[("", false), ("shortLeg", true)];

/// The terminals, in the table's order.
pub const TERMINALS: &[(&str, bool)] = &[("", false), ("tailed", true)];

/// The serifs, in the table's order.
pub const SERIFS: &[(&str, Serif)] = &[
    ("serifless", Serif::Serifless),
    ("serifed", Serif::Serifed),
    ("topLeftSerifed", Serif::TopLeftSerifed),
    ("bottomRightSerifed", Serif::BottomRightSerifed),
    (
        "topLeftAndBottomRightSerifed",
        Serif::TopLeftAndBottomRightSerifed,
    ),
];

/// Every variant the table names, as `(suffix, variant)` pairs.
#[must_use]
pub fn variants() -> Vec<(String, Variant)> {
    let mut out = Vec::new();
    for (body_key, body) in BODIES {
        for (leg_key, short_leg) in LEGS {
            for (terminal_key, tailed) in TERMINALS {
                for (serif_key, serif) in SERIFS {
                    let suffix = join_camel(
                        &join_camel(&join_camel(body_key, leg_key), terminal_key),
                        serif_key,
                    );
                    out.push((
                        suffix,
                        Variant {
                            body: *body,
                            short_leg: *short_leg,
                            tailed: *tailed,
                            serif: *serif,
                        },
                    ));
                }
            }
        }
    }

    out
}

/// One shoulder: the parameters its outline reads.
#[derive(Clone, Copy, Debug)]
struct Shoulder {
    left: f64,
    right: f64,
    fine: f64,
    top: f64,
    bottom: f64,
    ada: f64,
    adb: f64,
    stroke: f64,
    left_y0: Option<f64>,
}

/// The lower-case `m` under construction.
pub struct LowerM<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerM<'_> {
    /// The frame the definition draws in: `dfM`, `DivFrame para.advanceScaleMM 3`.
    fn frame(&self) -> DivFrame {
        DivFrame::new(self.metrics.clone(), Some(1.0), Some(3.0), None)
    }

    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    fn middle(&self) -> f64 {
        self.frame().width / 2.0
    }

    fn right_sb(&self) -> f64 {
        let df = self.frame();

        df.width - df.sb
    }

    fn adjust_x_top(&self, x: f64, sw: f64) -> f64 {
        x - self.metrics.correction_o_mid_x * sw
    }

    /// `SmallMSmoothA`.
    fn smooth_a(&self) -> f64 {
        let df = self.frame();

        df.slice(Some(3.0), Some(2.0), None)
            .arch_depth_a_of(self.metrics.small_arch_depth, Some(df.mvs))
    }

    /// `SmallMSmoothB`.
    fn smooth_b(&self) -> f64 {
        let df = self.frame();

        df.slice(Some(3.0), Some(2.0), None)
            .arch_depth_b_of(self.metrics.small_arch_depth, Some(df.mvs))
    }

    /// `SmallMShortLegHeight`.
    fn short_leg_height(&self, top: f64) -> f64 {
        let df = self.frame();

        mix(0.0, top - df.mvs, 0.45)
    }

    /// `SmallMSmoothHeight`.
    fn smooth_height(&self, top: f64) -> f64 {
        let df = self.frame();

        top - self.smooth_b()
            - self.metrics.tiny
            - self.hsw_to_v(self.metrics.tan_slope.abs() * df.mvs)
    }

    /// `SmallMShoulderSpiro`.
    fn shoulder_items(&self, shoulder: &Shoulder) -> Vec<Item> {
        let metrics = self.metrics;
        let stroke = shoulder.stroke;
        let fix = self.hsw_to_v(metrics.tan_slope * stroke);
        let right_inner = shoulder.right - self.hsw_to_v(stroke);
        let left_inner = shoulder.left + self.hsw_to_v(shoulder.fine);
        let left_y0 = |fallback: f64| shoulder.left_y0.unwrap_or(fallback);

        vec![
            self.kit
                .knot(KnotType::Corner, right_inner, shoulder.bottom, Vec::new()),
            self.kit.knot(
                KnotType::Right,
                right_inner,
                (shoulder.top - shoulder.adb + fix).max(shoulder.bottom + metrics.tiny),
                Vec::new(),
            ),
            self.kit.arcvh(Some(8), None),
            self.kit.pair(
                lemmata_ptl::spiro_kit::Family::G2,
                lemmata_ptl::spiro_kit::Direction::Right,
                lemmata_ptl::spiro_kit::Position::Mid,
                mix(right_inner, left_inner, 0.5),
                (shoulder.top - metrics.o) - stroke,
                Vec::new(),
            ),
            self.kit.archv(Some(8), None),
            self.kit.knot(
                KnotType::Left,
                left_inner,
                (left_y0(shoulder.bottom) + metrics.tiny).max(shoulder.top - shoulder.ada - fix),
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Corner,
                left_inner,
                left_y0(
                    shoulder
                        .bottom
                        .max((shoulder.top - shoulder.ada - fix) - metrics.tiny),
                ),
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Corner,
                shoulder.left,
                left_y0(
                    shoulder
                        .bottom
                        .max((shoulder.top - shoulder.ada) - metrics.tiny),
                ),
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Right,
                shoulder.left,
                (left_y0(shoulder.bottom) + metrics.tiny).max(shoulder.top - shoulder.ada),
                Vec::new(),
            ),
            self.kit.arcvh(Some(8), None),
            self.kit.pair(
                lemmata_ptl::spiro_kit::Family::G2,
                lemmata_ptl::spiro_kit::Direction::Right,
                lemmata_ptl::spiro_kit::Position::Mid,
                mix(shoulder.left, shoulder.right, 0.5),
                shoulder.top - metrics.o,
                Vec::new(),
            ),
            self.kit.archv(Some(8), None),
            self.kit.knot(
                KnotType::Left,
                shoulder.right,
                (shoulder.top - shoulder.adb).max(shoulder.bottom + metrics.tiny),
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Corner,
                shoulder.right,
                shoulder.bottom,
                Vec::new(),
            ),
            self.kit.close(),
        ]
    }

    /// `SmallMArches`: the left stem and the two shoulders.
    #[must_use]
    pub fn arches(&self, top: f64, lbot: f64, mbot: f64, rbot: f64, sw: f64) -> Vec<Geometry> {
        let metrics = self.metrics;
        let df = self.frame();
        let fine = sw * (metrics.shoulder_fine / metrics.stroke);
        let stem = shapes::vbar_l(self.kit, metrics, df.sb, lbot, top, sw)
            .geometry(kurbo::Affine::IDENTITY);
        let left_shoulder = self.kit.spiro_outline(self.shoulder_items(&Shoulder {
            left: df.sb + self.hsw_to_v(sw - fine),
            right: self.middle() + self.hsw_to_v(0.5 * sw),
            fine,
            top,
            bottom: mbot,
            ada: self.smooth_a(),
            adb: self.smooth_b(),
            stroke: sw,
            left_y0: None,
        }));
        let right_shoulder = self.kit.spiro_outline(self.shoulder_items(&Shoulder {
            left: self.middle() + self.hsw_to_v(0.5 * sw - fine),
            right: self.right_sb(),
            fine,
            top,
            bottom: rbot,
            ada: self.smooth_a(),
            adb: self.smooth_b(),
            stroke: sw,
            left_y0: Some(mbot),
        }));

        vec![
            stem,
            Geometry::Tagged(Box::new(Geometry::Spiro {
                gizmo: kurbo::Affine::IDENTITY,
                closed: left_shoulder.closed,
                knots: left_shoulder.knots,
            })),
            Geometry::Spiro {
                gizmo: kurbo::Affine::IDENTITY,
                closed: right_shoulder.closed,
                knots: right_shoulder.knots,
            },
        ]
    }

    /// `FlatTopSmallMShape`.
    #[must_use]
    pub fn flat_top(&self, top: f64, lbot: f64, mbot: f64, rbot: f64, sw: f64) -> Vec<Geometry> {
        let metrics = self.metrics;
        let df = self.frame();
        let adb = self.smooth_b();
        let arch = self.kit.dispiro(vec![
            Item::Af(self.kit.widths_rhs(Some(sw))),
            self.kit.knot(
                KnotType::Left,
                df.sb + metrics.tiny,
                top,
                self.kit.heading(shapes::rightward()),
            ),
            self.kit.knot(
                KnotType::Right,
                self.adjust_x_top(
                    mix(
                        self.middle() - self.hsw_to_v(0.5 * sw),
                        self.right_sb(),
                        0.5,
                    ),
                    sw,
                ),
                top,
                Vec::new(),
            ),
            self.kit.archv(None, None),
            self.kit.knot(
                KnotType::Left,
                self.right_sb(),
                (top - adb).max(rbot + metrics.tiny),
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Right,
                self.right_sb(),
                rbot,
                self.kit.heading(shapes::downward(metrics.hv_contrast)),
            ),
        ]);

        vec![
            Geometry::Tagged(Box::new(
                shapes::vbar_l(self.kit, metrics, df.sb, lbot, top, sw)
                    .geometry(kurbo::Affine::IDENTITY),
            )),
            Geometry::Tagged(Box::new(
                shapes::vbar_m(self.kit, metrics, self.middle(), mbot, top - 0.25 * sw, sw)
                    .geometry(kurbo::Affine::IDENTITY),
            )),
            arch.geometry(kurbo::Affine::IDENTITY),
        ]
    }

    const fn params(&self) -> shapes::ArchParams {
        let metrics = self.metrics;

        shapes::ArchParams {
            superness: self.design.superness,
            tan_slope: metrics.tan_slope,
            hv_contrast: metrics.hv_contrast,
            o: metrics.o,
            correction_o_mid_x: metrics.correction_o_mid_x,
        }
    }

    fn arch_after(
        &self,
        y: f64,
        before: &UserControlKnot,
        after: &UserControlKnot,
        sw: f64,
    ) -> Vec<Item> {
        shapes::arch(
            self.kit,
            &shapes::ArchArgs {
                y,
                p: None,
                sw,
                compact: false,
                o: self.metrics.o,
                angle_pre: None,
                angle_post: None,
                blend_pre: Some(false),
                blend_post: None,
                sw_before: Some(sw),
                sw_after: Some(sw),
                lhs: false,
            },
            before,
            after,
            &self.params(),
        )
    }

    /// `EarlessSingleArchSmallMShape`.
    #[must_use]
    pub fn earless_single_arch(
        &self,
        top: f64,
        lbot: f64,
        mbot: f64,
        rbot: f64,
        sw: f64,
    ) -> Vec<Geometry> {
        let metrics = self.metrics;
        let df = self.frame();
        let rise = metrics.d_toothless_rise;
        let adb = self.smooth_b();
        let before = UserControlKnot {
            ty: KnotType::G4,
            x: df.sb,
            y: top - rise,
            af: Vec::new(),
        };
        let after = UserControlKnot {
            ty: KnotType::Left,
            x: self.right_sb(),
            y: (top - adb).max(rbot + metrics.tiny),
            af: self.kit.heading(shapes::downward(metrics.hv_contrast)),
        };
        let mut items = vec![
            Item::Af(self.kit.widths_rhs(Some(sw))),
            self.kit.knot(before.ty, before.x, before.y, Vec::new()),
        ];
        items.extend(self.arch_after(top, &before, &after, sw));
        items.push(self.kit.knot(after.ty, after.x, after.y, after.af.clone()));
        items.push(self.kit.knot(
            KnotType::Right,
            self.right_sb(),
            rbot,
            self.kit.heading(shapes::downward(metrics.hv_contrast)),
        ));

        vec![
            Geometry::Tagged(Box::new(
                shapes::vbar_l(self.kit, metrics, df.sb, lbot, top - rise, sw)
                    .geometry(kurbo::Affine::IDENTITY),
            )),
            Geometry::Tagged(Box::new(
                shapes::vbar_m(self.kit, metrics, self.middle(), mbot, top - 0.25 * sw, sw)
                    .geometry(kurbo::Affine::IDENTITY),
            )),
            self.kit.dispiro(items).geometry(kurbo::Affine::IDENTITY),
        ]
    }

    /// `leftKnots` of `EarlessCornerDoubleArchSmallMShape`.
    fn corner_left(&self, top: f64, mbot: f64, offset: f64, outline: bool, sw: f64) -> Vec<Item> {
        let metrics = self.metrics;
        let df = self.frame();
        let rise = metrics.d_toothless_rise;
        let adb = self.smooth_b();
        let x = self.middle() + self.hsw_to_v(0.5 * sw) - offset;
        let start_ty = if outline {
            KnotType::Corner
        } else {
            KnotType::G2
        };
        let end_ty = if outline {
            KnotType::Corner
        } else {
            KnotType::Right
        };
        let before = UserControlKnot {
            ty: start_ty,
            x: df.sb - offset,
            y: top - rise,
            af: Vec::new(),
        };
        let after = UserControlKnot {
            ty: KnotType::Left,
            x,
            y: (top - adb).max(mbot + metrics.tiny),
            af: self.kit.heading(shapes::downward(metrics.hv_contrast)),
        };
        let mut items = vec![
            Item::Af(self.kit.widths_rhs(Some(sw))),
            self.kit.knot(before.ty, before.x, before.y, Vec::new()),
        ];
        items.extend(self.arch_after(top, &before, &after, sw));
        items.push(self.kit.knot(after.ty, after.x, after.y, after.af.clone()));
        items.push(self.kit.knot(
            end_ty,
            x,
            mbot,
            self.kit.heading(shapes::downward(metrics.hv_contrast)),
        ));
        if outline {
            items.push(
                self.kit
                    .knot(KnotType::Corner, df.sb - offset, 0.0, Vec::new()),
            );
        }

        items
    }

    /// `rightKnots` of `EarlessCornerDoubleArchSmallMShape`.
    fn corner_right(&self, top: f64, rbot: f64, sw: f64) -> Vec<Item> {
        let metrics = self.metrics;
        let rise = metrics.d_toothless_rise;
        let adb = self.smooth_b();
        let x_mid_left = self.middle() - self.hsw_to_v(0.5 * sw);
        let before = UserControlKnot {
            ty: KnotType::G2,
            x: x_mid_left,
            y: top - rise,
            af: Vec::new(),
        };
        let after = UserControlKnot {
            ty: KnotType::Left,
            x: self.right_sb(),
            y: (top - adb).max(rbot + metrics.tiny),
            af: self.kit.heading(shapes::downward(metrics.hv_contrast)),
        };
        let mut items = vec![
            Item::Af(self.kit.widths_rhs(Some(sw))),
            self.kit.knot(
                KnotType::G2,
                mix(self.right_sb(), x_mid_left, 1.5),
                top - 2.0 * rise,
                Vec::new(),
            ),
            self.kit.knot(before.ty, before.x, before.y, Vec::new()),
        ];
        items.extend(self.arch_after(top, &before, &after, sw));
        items.push(self.kit.knot(after.ty, after.x, after.y, after.af.clone()));
        items.push(self.kit.knot(
            KnotType::Right,
            self.right_sb(),
            rbot,
            self.kit.heading(shapes::downward(metrics.hv_contrast)),
        ));

        items
    }

    /// `EarlessCornerDoubleArchSmallMShape`.
    #[must_use]
    pub fn earless_corner_double_arch(
        &self,
        top: f64,
        lbot: f64,
        mbot: f64,
        rbot: f64,
        sw: f64,
    ) -> Vec<Geometry> {
        let metrics = self.metrics;
        let df = self.frame();
        let rise = metrics.d_toothless_rise;
        let left = self
            .kit
            .dispiro(self.corner_left(top, mbot, 0.0, false, sw));
        let left_outline = self
            .kit
            .spiro_outline(self.corner_left(top, mbot, 0.1, true, sw));
        let right = self.kit.dispiro(self.corner_right(top, rbot, sw));
        let difference = lemmata_ptl::glyph::difference(vec![
            right.geometry(kurbo::Affine::IDENTITY),
            Geometry::Spiro {
                gizmo: kurbo::Affine::IDENTITY,
                closed: left_outline.closed,
                knots: left_outline.knots,
            },
        ]);

        vec![
            Geometry::Tagged(Box::new(
                shapes::vbar_l(self.kit, metrics, df.sb, lbot, top - rise, sw)
                    .geometry(kurbo::Affine::IDENTITY),
            )),
            left.geometry(kurbo::Affine::IDENTITY),
            difference,
        ]
    }

    /// `RevSmallMShoulderSpiro`.
    fn rev_shoulder_items(&self, shoulder: &Shoulder, right_y0: f64) -> Vec<Item> {
        let metrics = self.metrics;
        let stroke = shoulder.stroke;
        let fix = self.hsw_to_v(metrics.tan_slope * stroke);
        let left_inner = shoulder.left + self.hsw_to_v(stroke);
        let right_inner = shoulder.right - self.hsw_to_v(shoulder.fine);

        vec![
            self.kit
                .knot(KnotType::Corner, left_inner, shoulder.bottom, Vec::new()),
            self.kit.knot(
                KnotType::Right,
                left_inner,
                (shoulder.top - shoulder.ada - fix).max(shoulder.bottom + metrics.tiny),
                Vec::new(),
            ),
            self.kit.arcvh(Some(8), None),
            self.kit.pair(
                lemmata_ptl::spiro_kit::Family::G2,
                lemmata_ptl::spiro_kit::Direction::Right,
                lemmata_ptl::spiro_kit::Position::Mid,
                mix(left_inner, right_inner, 0.5),
                (shoulder.top - metrics.o) - stroke,
                Vec::new(),
            ),
            self.kit.archv(Some(8), None),
            self.kit.knot(
                KnotType::Left,
                right_inner,
                (right_y0.max(shoulder.bottom) + metrics.tiny)
                    .max(shoulder.top - shoulder.adb + fix),
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Corner,
                right_inner,
                right_y0.max(
                    shoulder
                        .bottom
                        .max((shoulder.top - shoulder.adb + fix) - metrics.tiny),
                ),
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Corner,
                shoulder.right,
                right_y0.max(
                    shoulder
                        .bottom
                        .max((shoulder.top - shoulder.adb) - metrics.tiny),
                ),
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Right,
                shoulder.right,
                (right_y0.max(shoulder.bottom) + metrics.tiny).max(shoulder.top - shoulder.adb),
                Vec::new(),
            ),
            self.kit.arcvh(Some(8), None),
            self.kit.pair(
                lemmata_ptl::spiro_kit::Family::G2,
                lemmata_ptl::spiro_kit::Direction::Right,
                lemmata_ptl::spiro_kit::Position::Mid,
                mix(shoulder.right, shoulder.left, 0.5),
                shoulder.top - metrics.o,
                Vec::new(),
            ),
            self.kit.archv(Some(8), None),
            self.kit.knot(
                KnotType::Left,
                shoulder.left,
                (shoulder.top - shoulder.ada).max(shoulder.bottom + metrics.tiny),
                Vec::new(),
            ),
            self.kit
                .knot(KnotType::Corner, shoulder.left, shoulder.bottom, Vec::new()),
            self.kit.close(),
        ]
    }

    /// `EarlessRoundedDoubleArchSmallMShape`.
    #[must_use]
    pub fn earless_rounded_double_arch(
        &self,
        top: f64,
        lbot: f64,
        mbot: f64,
        rbot: f64,
        sw: f64,
    ) -> Vec<Geometry> {
        let metrics = self.metrics;
        let df = self.frame();
        let fine = sw * metrics.c_thin;
        let rev = self.kit.spiro_outline(self.rev_shoulder_items(
            &Shoulder {
                left: df.sb,
                right: self.middle() - self.hsw_to_v(0.5 * sw - fine),
                fine,
                top,
                bottom: lbot,
                ada: self.smooth_a(),
                adb: self.smooth_b(),
                stroke: sw,
                left_y0: None,
            },
            mbot,
        ));
        let arch = self.kit.spiro_outline(self.shoulder_items(&Shoulder {
            left: self.middle() + self.hsw_to_v(0.5 * sw - fine),
            right: self.right_sb(),
            fine,
            top,
            bottom: rbot,
            ada: self.smooth_a(),
            adb: self.smooth_b(),
            stroke: sw,
            left_y0: Some(mbot),
        }));

        vec![lemmata_ptl::glyph::union(vec![
            Geometry::Spiro {
                gizmo: kurbo::Affine::IDENTITY,
                closed: rev.closed,
                knots: rev.knots,
            },
            Geometry::Spiro {
                gizmo: kurbo::Affine::IDENTITY,
                closed: arch.closed,
                knots: arch.knots,
            },
        ])]
    }

    /// The body the variant selects.
    fn body(&self, variant: Variant, top: f64, mbot: f64, rbot: f64, sw: f64) -> Vec<Geometry> {
        let lbot = 0.0;

        match variant.body {
            Body::Arches => self.arches(top, lbot, mbot, rbot, sw),
            Body::FlatTop => self.flat_top(top, lbot, mbot, rbot, sw),
            Body::EarlessSingleArch => self.earless_single_arch(top, lbot, mbot, rbot, sw),
            Body::EarlessCornerDoubleArch => {
                self.earless_corner_double_arch(top, lbot, mbot, rbot, sw)
            }
            Body::EarlessRoundedDoubleArch => {
                self.earless_rounded_double_arch(top, lbot, mbot, rbot, sw)
            }
        }
    }

    /// `MEnoughSpaceForFullSerifs`.
    fn enough_space_for_full_serifs(&self) -> bool {
        let df = self.frame();
        let ink = self.hsw_to_v(df.mvs);
        let gap = (self.right_sb() - df.sb) / 2.0 - 1.5 * ink;

        0.5 * ink + 0.375 * gap > self.metrics.ref_jut
    }

    fn serif_frame(&self, top: f64, bot: f64) -> SerifFrame {
        SerifFrame::from_df(self.metrics, &self.frame(), top, bot, None, None, false)
    }

    fn top_left(&self, top: f64) -> Geometry {
        self.serif_frame(top, 0.0).lt_outer(self.kit, self.metrics)
    }

    fn bottom_left(&self, top: f64, lbot: f64, full: bool) -> Geometry {
        if full {
            self.serif_frame(top, lbot).lb_full(self.kit, self.metrics)
        } else {
            self.serif_frame(top, lbot).lb_outer(self.kit, self.metrics)
        }
    }

    fn bottom_right(&self, top: f64, rbot: f64, full: bool) -> Geometry {
        if full {
            self.serif_frame(top, rbot).rb_full(self.kit, self.metrics)
        } else {
            self.serif_frame(top, rbot).rb_outer(self.kit, self.metrics)
        }
    }

    /// The serif setting's shapes.
    fn serifs(
        &self,
        variant: Variant,
        top: f64,
        mbot: f64,
        rbot: f64,
        include_rb: bool,
    ) -> Vec<Geometry> {
        if variant.serif == Serif::Serifless {
            return Vec::new();
        }
        let full = self.enough_space_for_full_serifs();
        let left_full = full || variant.short_leg;
        let earless = matches!(
            variant.body,
            Body::EarlessSingleArch
                | Body::EarlessCornerDoubleArch
                | Body::EarlessRoundedDoubleArch
        );
        let mut parts = Vec::new();
        // `FullSerifs` leaves the top-left serif off an earless body; the
        // settings that name it explicitly still draw it.
        let wants_top_left = match variant.serif {
            Serif::Serifed => !earless,
            Serif::TopLeftSerifed | Serif::TopLeftAndBottomRightSerifed => true,
            Serif::Serifless | Serif::BottomRightSerifed => false,
        };
        if wants_top_left {
            parts.push(self.top_left(top));
        }
        if variant.serif == Serif::Serifed {
            parts.push(self.bottom_left(top, 0.0, left_full));
            if full {
                parts.push(self.serif_frame(top, mbot).mb_full(self.kit, self.metrics));
            }
            if !variant.tailed && include_rb {
                parts.push(self.bottom_right(top, rbot, left_full));
            }
        }
        if matches!(
            variant.serif,
            Serif::BottomRightSerifed | Serif::TopLeftAndBottomRightSerifed
        ) {
            parts.push(self.serif_frame(top, rbot).rb_outer(self.kit, self.metrics));
        }

        parts
    }

    /// `m.<suffix>`, with the right-bottom serif optional so `meng` can leave it
    /// out without the tag, which the geometry does not carry.
    fn glyph_inner(&self, variant: Variant, include_rb: bool) -> Geometry {
        let metrics = self.metrics;
        let df = self.frame();
        let top = metrics.xh;
        let mbot = if variant.short_leg {
            self.short_leg_height(top)
        } else {
            0.0
        };
        let smooth_height = self.smooth_height(top);
        let rbot = if variant.tailed {
            smooth_height + metrics.o
        } else {
            0.0
        };
        let mut parts = self.body(variant, top, mbot, rbot, df.mvs);
        if variant.tailed {
            parts.push(
                shapes::rightward_tailed_bar(
                    self.kit,
                    metrics,
                    &df,
                    self.right_sb(),
                    0.0,
                    smooth_height,
                    df.mvs,
                )
                .geometry(kurbo::Affine::IDENTITY),
            );
        }
        parts.extend(self.serifs(variant, top, mbot, rbot, include_rb));

        Geometry::Combine(parts)
    }

    /// `m.<suffix>`.
    #[must_use]
    pub fn glyph(&self, variant: Variant) -> Geometry {
        self.glyph_inner(variant, true)
    }

    /// `meng.<suffix>`, which the untailed half gets.
    #[must_use]
    pub fn eng(&self, variant: Variant) -> Geometry {
        let metrics = self.metrics;
        let base = self.glyph_inner(variant, false);
        let mut parts = vec![base];
        parts.push(
            shapes::eng_hook(
                self.kit,
                metrics,
                self.right_sb(),
                0.0,
                metrics.descender,
                self.frame().mvs,
            )
            .geometry(kurbo::Affine::IDENTITY),
        );

        Geometry::Combine(parts)
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for (suffix, variant) in variants() {
            out.push((format!("m.{suffix}"), self.glyph(variant)));
            if !variant.tailed {
                out.push((format!("meng.{suffix}"), self.eng(variant)));
            }
        }

        out
    }
}
