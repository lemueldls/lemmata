//! The letter `f` from `letter/latin/lower-f.ptl`.
//!
//! Three bodies: the standard one, the narrow one, and the flat-hook one that
//! turns the top of the stem into a hook. The flat-hook body has four CLC
//! settings that widen the hook and the crossbar. Each body takes five counter
//! ends and three crossbar heights.
//!
//! `MarkSet.b`/`bp` and `LeaningAnchor.Above.Hook` place marks rather than
//! geometry, so they are not drawn here. `fPalatalHook` is built from the
//! `overlay` and `palatalHookPos` anchors the bodies set. `fHookBottom`,
//! `fLenis`, the accented compositions and `mathbb/f` are not ported yet.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    control::{Item, UserControlKnot},
    glyph,
    letter_model::DivFrame,
    metrics::Metrics,
    shapes::{self, Design, HookArgs, HookParams, downward, upward},
    spiro_kit::{Dispiro, Kit},
    util::mix,
};

/// The plan's `advanceScaleF`. The reference plan sets it to 1, and the port
/// builds the reference plan's frames.
const ADVANCE_SCALE_F: f64 = 1.0;

/// The body table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Standard,
    Narrow,
    FlatHook,
    FlatHookClc1,
    FlatHookClc2,
    FlatHookClc3,
    FlatHookClc4,
}

/// The counter ends, named by the hook they draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hook {
    Serifless,
    Serifed,
    Extended,
    Tailed,
    DiagonalTailed,
}

/// The crossbar heights.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crossbar {
    Default,
    XHeight,
    HalfAscenderHeight,
}

/// The `f` families this module builds.
pub struct LowerF<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerF<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    fn right_sb(&self) -> f64 {
        self.metrics.width - self.metrics.sb
    }

    fn df_n(&self) -> DivFrame {
        DivFrame::new(self.metrics.clone(), Some(1.0), None, None)
    }

    fn df_f(&self) -> DivFrame {
        DivFrame::new(
            self.metrics.clone(),
            Some(ADVANCE_SCALE_F),
            Some(1.0),
            Some(0.75),
        )
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

    /// `ySmallFBarPos`.
    fn small_f_bar_pos(&self) -> f64 {
        mix(0.0, self.metrics.xh, self.design.f_bar_pos_to_xh)
            + self.metrics.stroke * self.design.fbar_stroke_adj
    }

    /// `StdSmallFBarLeftPos0`.
    fn bar_left_0(&self) -> f64 {
        mix(self.metrics.sb, self.right_sb(), 0.35) - self.hsw_to_v(0.25 * self.metrics.stroke)
    }

    /// `StdSmallFBarLeftPos`.
    fn bar_left(&self, bar_at_center: bool) -> f64 {
        let (at, ink) = if bar_at_center {
            (0.45, 0.45)
        } else {
            (0.35, 0.25)
        };

        mix(self.metrics.sb, self.right_sb(), at) - self.hsw_to_v(ink * self.metrics.stroke)
    }

    /// The crossbar mix the hook selects.
    const fn side_mix(body: Body, hook: Hook) -> f64 {
        match hook {
            Hook::Tailed | Hook::DiagonalTailed => 1.0,
            _ => {
                if matches!(body, Body::Standard) {
                    0.95
                } else {
                    mix(1.0, ADVANCE_SCALE_F, 1.5)
                }
            }
        }
    }

    /// The crossbar height the table selects.
    fn bar_height(&self, crossbar: Crossbar) -> f64 {
        match crossbar {
            Crossbar::Default => self.small_f_bar_pos(),
            Crossbar::XHeight => self.metrics.xh,
            Crossbar::HalfAscenderHeight => {
                mix(0.0, self.metrics.ascender, 0.5) + self.metrics.half_stroke
            }
        }
    }

    /// The stem's bar edges for a body and counter end.
    fn bar_layout(&self, body: Body, hook: Hook, m: f64) -> (DivFrame, f64, f64) {
        let metrics = self.metrics;
        let df = match body {
            Body::Standard => self.df_n(),
            _ => self.df_f(),
        };
        let at_center = matches!(hook, Hook::Tailed | Hook::DiagonalTailed);
        let left = match body {
            Body::Standard => self.bar_left(at_center),
            Body::Narrow => {
                let (at, ink) = if at_center {
                    (0.45, 0.45)
                } else {
                    (0.35, 0.25)
                };
                mix(df.left_sb(), df.right_sb(), at) - self.hsw_to_v(ink * metrics.stroke)
            }
            _ => {
                if at_center {
                    df.middle() - self.hsw_to_v(0.5 * metrics.stroke)
                } else {
                    let cross_left = mix(0.0, df.left_sb(), m);
                    let cross_right = mix(df.width, df.right_sb(), m);

                    mix(cross_left, cross_right, 0.42) - self.hsw_to_v(0.375 * metrics.stroke)
                }
            }
        };

        (df, left, left + self.hsw_to_v(metrics.stroke))
    }

    /// `palatalHookPos`, when the counter end sets it.
    fn palatal_hook_pos(
        &self,
        body: Body,
        hook: Hook,
        df: &DivFrame,
        bar_left: f64,
    ) -> Option<(f64, f64)> {
        let metrics = self.metrics;
        match hook {
            Hook::Serifless => {
                Some((
                    bar_left
                        + self.hsw_to_v(
                            metrics.stroke + metrics.v_jut_stroke.max(metrics.width / 12.0),
                        ),
                    0.0,
                ))
            }
            Hook::Serifed => {
                let x = match body {
                    Body::FlatHook
                    | Body::FlatHookClc1
                    | Body::FlatHookClc2
                    | Body::FlatHookClc3
                    | Body::FlatHookClc4 => {
                        let jut = metrics.mid_jut_center.max(mix(
                            self.hsw_to_v(metrics.half_stroke),
                            metrics.long_jut,
                            df.adws(),
                        ));

                        df.middle() + jut + 0.5 * metrics.stroke * metrics.tan_slope
                    }
                    _ => {
                        mix(df.left_sb(), df.right_sb(), 0.875)
                            + 0.5 * metrics.stroke * metrics.tan_slope
                    }
                };

                Some((x, 0.0))
            }
            _ => None,
        }
    }

    /// `StdFShapeT`: the stem, its hook, and the crossbar's left edge.
    #[allow(clippy::too_many_arguments)]
    fn std_f_shape_t(&self, df: &DivFrame, offset: f64, bar_left: f64, sw: f64) -> Dispiro {
        let metrics = self.metrics;
        let x = (bar_left + offset + self.hsw_to_v(2.125 * sw))
            .max(df.right_sb() + metrics.r_balance2 - (metrics.ox - metrics.o) - offset);
        let terminal = UserControlKnot {
            ty: KnotType::G4,
            x,
            y: metrics.ascender - metrics.hook * 0.75,
            af: Vec::new(),
        };
        let straight = UserControlKnot {
            ty: KnotType::Left,
            x: bar_left + offset,
            y: metrics.ascender - df.arch_depth_a * 0.8,
            af: Vec::new(),
        };
        let bottom = UserControlKnot {
            ty: if offset == 0.0 {
                KnotType::Right
            } else {
                KnotType::Corner
            },
            x: bar_left + offset,
            y: 0.0,
            af: self.kit.heading(downward(metrics.hv_contrast)),
        };

        let mut items = vec![Item::Af(self.kit.widths_lhs(Some(sw)))];
        let hook = shapes::hook_start(
            self.kit,
            &terminal,
            &straight,
            &HookArgs {
                y_ref: metrics.ascender - offset,
                sw,
                sw_terminal: sw,
                is_tail: false,
                suppress_swash: false,
                overshoot: metrics.o,
            },
            &self.hook_params(),
        );
        items.push(self.kit.knot(
            hook.terminal.ty,
            hook.terminal.x,
            hook.terminal.y,
            hook.terminal.af,
        ));
        items.extend(hook.items);
        items.push(
            self.kit
                .knot(straight.ty, straight.x, straight.y, straight.af),
        );
        items.push(self.kit.knot(bottom.ty, bottom.x, bottom.y, bottom.af));
        if offset != 0.0 {
            items.push(self.kit.knot(
                KnotType::Corner,
                df.right_sb() + metrics.r_balance2 - (metrics.ox - metrics.o) - offset,
                0.0,
                Vec::new(),
            ));
        }

        self.kit.dispiro(items)
    }

    /// `NarrowStdFShapeT`.
    fn narrow_std_f_shape_t(&self, df: &DivFrame, offset: f64, bar_left: f64, sw: f64) -> Dispiro {
        let metrics = self.metrics;
        let p_adaptive = if ADVANCE_SCALE_F < 1.0 {
            (1.0 - df.adws()) / (1.0 - ADVANCE_SCALE_F)
        } else {
            0.0
        };
        let hook_scale = mix(0.9, 1.0, p_adaptive);
        let hook_right = mix(
            bar_left,
            df.right_sb() + metrics.r_balance2 - (metrics.ox - metrics.o),
            hook_scale,
        );
        let sw_terminal = mix(sw, df.advice_stroke(2.75, 1.0), mix(0.35, 0.1, p_adaptive));
        let terminal = UserControlKnot {
            ty: KnotType::G4,
            x: (bar_left + offset + self.hsw_to_v(2.125 * sw)).max(hook_right - offset),
            y: metrics.ascender - metrics.hook * 0.75,
            af: Vec::new(),
        };
        let straight = UserControlKnot {
            ty: KnotType::Left,
            x: bar_left + offset,
            y: metrics.ascender - df.arch_depth_a * 0.8,
            af: Vec::new(),
        };
        let bottom = UserControlKnot {
            ty: if offset == 0.0 {
                KnotType::Right
            } else {
                KnotType::Corner
            },
            x: bar_left + offset,
            y: 0.0,
            af: self.kit.heading(downward(metrics.hv_contrast)),
        };

        let mut items = vec![Item::Af(self.kit.widths_lhs(Some(sw)))];
        let hook = shapes::hook_start(
            self.kit,
            &terminal,
            &straight,
            &HookArgs {
                y_ref: metrics.ascender - offset,
                sw,
                sw_terminal,
                is_tail: false,
                suppress_swash: false,
                overshoot: metrics.o,
            },
            &self.hook_params(),
        );
        items.push(self.kit.knot(
            hook.terminal.ty,
            hook.terminal.x,
            hook.terminal.y,
            hook.terminal.af,
        ));
        items.extend(hook.items);
        items.push(
            self.kit
                .knot(straight.ty, straight.x, straight.y, straight.af),
        );
        items.push(self.kit.knot(bottom.ty, bottom.x, bottom.y, bottom.af));
        if offset != 0.0 {
            items.push(
                self.kit
                    .knot(KnotType::Corner, hook_right - offset, 0.0, Vec::new()),
            );
        }

        self.kit.dispiro(items)
    }

    /// `SmallFBottomSerif`.
    fn small_f_bottom_serif(&self, df: &DivFrame) -> Dispiro {
        let metrics = self.metrics;
        let l = mix(df.left_sb(), df.right_sb(), 0.020) + 0.5 * metrics.stroke * metrics.tan_slope;
        let r = mix(df.left_sb(), df.right_sb(), 0.875) + 0.5 * metrics.stroke * metrics.tan_slope;

        self.kit.dispiro(vec![
            Item::Af(self.kit.widths_lhs(None)),
            self.kit.knot(KnotType::Left, l, 0.0, Vec::new()),
            self.kit.knot(KnotType::Right, r, 0.0, Vec::new()),
        ])
    }

    /// `NarrowBottomSerif`.
    fn narrow_bottom_serif(&self, df: &DivFrame) -> Dispiro {
        let metrics = self.metrics;
        let jut = metrics.mid_jut_center.max(mix(
            self.hsw_to_v(metrics.half_stroke),
            metrics.long_jut,
            df.adws(),
        ));

        shapes::hserif_mb(self.kit, metrics, df.middle(), 0.0, jut, None)
    }

    /// `SmallFDownExtension`.
    fn small_f_down_extension(&self, bar_left: f64) -> Geometry {
        shapes::vbar_l(
            self.kit,
            self.metrics,
            bar_left,
            mix(0.0, self.metrics.descender, 0.9),
            self.metrics.tiny,
            self.metrics.stroke,
        )
        .geometry(kurbo::Affine::IDENTITY)
    }

    /// `SmallFDownHook`.
    fn small_f_down_hook(&self, df: &DivFrame, bar_right: f64, counter_left: f64) -> Dispiro {
        let metrics = self.metrics;
        let hd = shapes::flat_hook_depth(metrics, df.adws());
        let heading = self.kit.heading(upward(metrics.hv_contrast));

        self.kit.dispiro(vec![
            Item::Af(self.kit.widths_lhs(None)),
            self.kit
                .knot(KnotType::Left, counter_left, metrics.descender, Vec::new()),
            self.kit.knot(
                KnotType::Right,
                (bar_right - hd.x).max(counter_left + metrics.tiny),
                metrics.descender,
                Vec::new(),
            ),
            self.kit.archv(None, Some(self.design.tight_hook_superness)),
            self.kit.knot(
                KnotType::Left,
                bar_right,
                0.0f64.min(metrics.descender + hd.y),
                heading.clone(),
            ),
            self.kit
                .knot(KnotType::Right, bar_right, metrics.tiny, heading),
        ])
    }

    /// `SmallFDiagonalTail`.
    fn small_f_diagonal_tail(&self, df: &DivFrame, bar_left: f64) -> Dispiro {
        let metrics = self.metrics;
        let x_bar_middle = bar_left + self.hsw_to_v(0.5 * metrics.stroke);
        let depth = shapes::diag_tail_std_depth(metrics, df.adws(), metrics.stroke);
        let mut items = vec![
            Item::Af(self.kit.widths_center(None)),
            self.kit.knot(
                KnotType::Left,
                x_bar_middle,
                metrics.tiny,
                self.kit.heading(downward(metrics.hv_contrast)),
            ),
        ];
        items.extend(shapes::diag_tail_l(
            self.kit,
            metrics,
            self.design,
            x_bar_middle,
            metrics.descender,
            depth - metrics.half_stroke,
            metrics.stroke,
        ));

        self.kit.dispiro(items)
    }

    /// The counter hook the variant selects, if it draws one.
    fn counter_hook(
        &self,
        df: &DivFrame,
        hook: Hook,
        bar_left: f64,
        bar_right: f64,
        counter_left: f64,
        narrow_serif: bool,
    ) -> Vec<Geometry> {
        match hook {
            Hook::Serifless => Vec::new(),
            Hook::Serifed => {
                vec![
                    if narrow_serif {
                        self.narrow_bottom_serif(df)
                    } else {
                        self.small_f_bottom_serif(df)
                    }
                    .geometry(kurbo::Affine::IDENTITY),
                ]
            }
            Hook::Extended => vec![self.small_f_down_extension(bar_left)],
            Hook::Tailed => {
                vec![
                    self.small_f_down_hook(df, bar_right, counter_left)
                        .geometry(kurbo::Affine::IDENTITY),
                ]
            }
            Hook::DiagonalTailed => {
                vec![
                    self.small_f_diagonal_tail(df, bar_left)
                        .geometry(kurbo::Affine::IDENTITY),
                ]
            }
        }
    }

    /// `StandardFShape`.
    fn standard(
        &self,
        df: &DivFrame,
        m: f64,
        bh: f64,
        hook: Hook,
        bar_left: f64,
        bar_right: f64,
    ) -> Geometry {
        let metrics = self.metrics;
        let cross_right = mix(df.width, df.right_sb(), m);
        let bar_right_x = mix(df.left_sb(), df.right_sb(), m);
        let bar_left_0 = self.bar_left_0();

        let stem = Geometry::Transformed {
            tf: kurbo::Affine::translate((bar_left - bar_left_0, 0.0)),
            geom: Box::new(
                self.std_f_shape_t(df, 0.0, bar_left_0, metrics.stroke)
                    .geometry(kurbo::Affine::IDENTITY),
            ),
        };
        let mut parts = vec![
            stem,
            shapes::h_cross_bar_top(self.kit, metrics.sb, bar_right_x, bh, None)
                .geometry(kurbo::Affine::IDENTITY),
        ];
        let counter_left = (bar_right - (cross_right - bar_left)).max(metrics.sb);
        parts.extend(self.counter_hook(df, hook, bar_left, bar_right, counter_left, false));

        Geometry::Combine(parts)
    }

    /// `NarrowStdFShape`.
    fn narrow_std(
        &self,
        df: &DivFrame,
        m: f64,
        bh: f64,
        hook: Hook,
        bar_left: f64,
        bar_right: f64,
    ) -> Geometry {
        let metrics = self.metrics;
        let cross_right = mix(df.width, df.right_sb(), m);
        let bar_left_0 =
            mix(df.left_sb(), df.right_sb(), 0.35) - self.hsw_to_v(0.25 * metrics.stroke);

        let stem = Geometry::Transformed {
            tf: kurbo::Affine::translate((bar_left - bar_left_0, 0.0)),
            geom: Box::new(
                self.narrow_std_f_shape_t(df, 0.0, bar_left_0, metrics.stroke)
                    .geometry(kurbo::Affine::IDENTITY),
            ),
        };
        let p_adaptive = if ADVANCE_SCALE_F < 1.0 {
            (1.0 - df.adws()) / (1.0 - ADVANCE_SCALE_F)
        } else {
            0.0
        };
        let std_cross_right = mix(df.left_sb(), df.right_sb(), m);
        let narrow_cross_right = mix(bar_left, std_cross_right, mix(0.85, 1.4, p_adaptive));
        let narrow_cross_left = mix(df.left_sb(), bar_left, mix(0.5, 0.375, p_adaptive));
        let mut parts = vec![
            stem,
            shapes::h_cross_bar_top(self.kit, narrow_cross_left, narrow_cross_right, bh, None)
                .geometry(kurbo::Affine::IDENTITY),
        ];
        parts.extend(self.counter_hook(
            df,
            hook,
            bar_left,
            bar_right,
            (bar_right - 0.75 * (cross_right - bar_left)).max(df.left_sb()),
            false,
        ));

        Geometry::Combine(parts)
    }

    /// `NarrowFShape`, with the CLC widening the style selects.
    #[allow(clippy::too_many_arguments)]
    fn narrow(
        &self,
        df: &DivFrame,
        m: f64,
        bh: f64,
        hook: Hook,
        clc: Option<u8>,
        bar_left: f64,
        bar_right: f64,
    ) -> Geometry {
        let metrics = self.metrics;
        let top_hook_ext = match clc {
            Some(1) => metrics.width,
            Some(2..=4) => metrics.jut,
            _ => 0.0,
        };
        let top_hook_hd_ext = if clc == Some(1) { 1.0 } else { 0.0 };
        let cross_bar_ext = match clc {
            Some(1 | 2 | 4) => metrics.jut,
            _ => 0.0,
        };
        let cross_left = mix(0.0, df.left_sb(), m);
        let cross_right = mix(df.width, df.right_sb(), m);
        let hd = shapes::flat_hook_depth(metrics, df.adws());

        let stem = self.kit.dispiro(vec![
            Item::Af(self.kit.widths_lhs(None)),
            self.kit.knot(
                KnotType::Left,
                cross_right + top_hook_ext,
                metrics.ascender,
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Right,
                (bar_left + hd.x * (1.0 + top_hook_hd_ext)).min(cross_right - metrics.tiny),
                metrics.ascender,
                Vec::new(),
            ),
            self.kit.archv(None, Some(self.design.tight_hook_superness)),
            self.kit.knot(
                KnotType::Left,
                bar_left,
                metrics.xh.max(metrics.ascender - hd.y),
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Right,
                bar_left,
                0.0,
                self.kit.heading(downward(metrics.hv_contrast)),
            ),
        ]);
        let mut parts = vec![
            stem.geometry(kurbo::Affine::IDENTITY),
            shapes::h_cross_bar_top(self.kit, cross_left, cross_right + cross_bar_ext, bh, None)
                .geometry(kurbo::Affine::IDENTITY),
        ];
        parts.extend(self.counter_hook(
            df,
            hook,
            bar_left,
            bar_right,
            bar_right - (cross_right - bar_left),
            true,
        ));

        Geometry::Combine(parts)
    }

    /// One `f.<suffix>`.
    #[must_use]
    pub fn glyph(&self, body: Body, hook: Hook, crossbar: Crossbar) -> Geometry {
        let m = Self::side_mix(body, hook);
        let bh = self.bar_height(crossbar);
        let (df, bar_left, bar_right) = self.bar_layout(body, hook, m);

        match body {
            Body::Standard => self.standard(&df, m, bh, hook, bar_left, bar_right),
            Body::Narrow => self.narrow_std(&df, m, bh, hook, bar_left, bar_right),
            Body::FlatHook => self.narrow(&df, m, bh, hook, None, bar_left, bar_right),
            Body::FlatHookClc1 => self.narrow(&df, m, bh, hook, Some(1), bar_left, bar_right),
            Body::FlatHookClc2 => self.narrow(&df, m, bh, hook, Some(2), bar_left, bar_right),
            Body::FlatHookClc3 => self.narrow(&df, m, bh, hook, Some(3), bar_left, bar_right),
            Body::FlatHookClc4 => self.narrow(&df, m, bh, hook, Some(4), bar_left, bar_right),
        }
    }

    /// `fPalatalHook.<suffix>`: the base with the palatal hook under the stem.
    ///
    /// The hook hangs from the `overlay` anchor. A counter end that sets
    /// `palatalHookPos` also moves the hook out to it, and where it does not,
    /// the part below the baseline and left of the stem is cut away.
    #[must_use]
    pub fn palatal_hook(&self, body: Body, hook: Hook, crossbar: Crossbar) -> Geometry {
        let metrics = self.metrics;
        let m = Self::side_mix(body, hook);
        let (df, bar_left, _) = self.bar_layout(body, hook, m);
        let base = self.glyph(body, hook, crossbar);
        let attach_x = bar_left + self.hsw_to_v(0.5 * metrics.stroke);
        let pos = self.palatal_hook_pos(body, hook, &df, bar_left);
        let ref_sw = metrics.advice_stroke(if pos.is_some() { 2.0 } else { 3.0 }, 1.0, 1.0);
        let sw = shapes::arc_start_serif_width(ref_sw, metrics.v_jut_stroke);
        let x = pos.map_or_else(
            || attach_x + (self.right_sb() - metrics.sb) / 2.0,
            |(x, _)| x,
        ) - self.hsw_to_v(0.5 * sw);
        let y = pos.map_or(0.0, |(_, y)| y);
        let hook = glyph::union(shapes::palatal_hook(
            self.kit,
            metrics,
            x,
            y,
            Some(attach_x),
            y,
            0.0,
            sw,
        ));
        let hook = if pos.is_some() {
            hook
        } else {
            let mask = glyph::intersection(vec![
                spiro_geometry(shapes::rect(
                    self.kit,
                    0.0,
                    -metrics.very_far,
                    -metrics.very_far,
                    metrics.very_far,
                )),
                spiro_geometry(shapes::rect(
                    self.kit,
                    metrics.very_far,
                    -metrics.very_far,
                    -metrics.very_far,
                    attach_x + self.hsw_to_v(metrics.half_stroke),
                )),
            ]);
            glyph::difference(vec![hook, mask])
        };

        Geometry::Combine(vec![base, hook])
    }

    /// The variants the plan can select, by name.
    ///
    /// The four CLC bodies are the left parts of the `f` ligatures rather than
    /// selectable variants, so they are not here. Upstream's cleaned store only
    /// keeps them as ligature parts too.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        self.build_glyphs(false)
    }

    /// The ligature components, by name.
    ///
    /// The four CLC bodies widen the hook and the crossbar for a ligature's
    /// left half. The ligature machinery asks for them when it composes a
    /// sequence.
    #[must_use]
    pub fn ligature_glyphs(&self) -> Vec<(String, Geometry)> {
        self.build_glyphs(true)
    }

    fn build_glyphs(&self, ligatures: bool) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for (body_key, body) in BODIES {
            let is_ligature = matches!(
                body,
                Body::FlatHookClc1 | Body::FlatHookClc2 | Body::FlatHookClc3 | Body::FlatHookClc4
            );
            if is_ligature != ligatures {
                continue;
            }
            for (hook_key, hook) in HOOKS {
                for (bar_key, bar) in BARS {
                    let suffix = weave(&[body_key, hook_key, bar_key]);
                    out.push((format!("f.{suffix}"), self.glyph(*body, *hook, *bar)));
                    out.push((
                        format!("fPalatalHook.{suffix}"),
                        self.palatal_hook(*body, *hook, *bar),
                    ));
                }
            }
        }

        out
    }
}

/// A spiro outline as a geometry node.
fn spiro_geometry(outline: lemmata_ptl::spiro_kit::SpiroOutline) -> Geometry {
    Geometry::Spiro {
        gizmo: kurbo::Affine::IDENTITY,
        closed: outline.closed,
        knots: outline.knots,
    }
}

/// `SuffixCfg.weave`: join the non-empty keys, capitalizing every one but the
/// first so `narrow` and `serifless` read as `narrowSerifless`.
fn weave(keys: &[&str]) -> String {
    let mut suffix = String::new();
    let mut first = true;
    for key in keys.iter().filter(|key| !key.is_empty()) {
        if first {
            suffix.push_str(key);
            first = false;
        } else {
            let mut chars = key.chars();
            if let Some(first_char) = chars.next() {
                suffix.extend(first_char.to_uppercase());
                suffix.push_str(chars.as_str());
            }
        }
    }

    suffix
}

/// The body suffixes, in the order the table names them.
pub const BODIES: &[(&str, Body)] = &[
    ("", Body::Standard),
    ("narrow", Body::Narrow),
    ("flatHook", Body::FlatHook),
    ("flatHookCLC1", Body::FlatHookClc1),
    ("flatHookCLC2", Body::FlatHookClc2),
    ("flatHookCLC3", Body::FlatHookClc3),
    ("flatHookCLC4", Body::FlatHookClc4),
];

/// The counter-end suffixes.
pub const HOOKS: &[(&str, Hook)] = &[
    ("serifless", Hook::Serifless),
    ("serifed", Hook::Serifed),
    ("extended", Hook::Extended),
    ("tailed", Hook::Tailed),
    ("diagonalTailed", Hook::DiagonalTailed),
];

/// The crossbar suffixes.
pub const BARS: &[(&str, Crossbar)] = &[
    ("", Crossbar::Default),
    ("CrossbarAtXHeight", Crossbar::XHeight),
    ("CrossbarAtHalfAscenderHeight", Crossbar::HalfAscenderHeight),
];
