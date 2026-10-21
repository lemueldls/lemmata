//! Delimiters from `symbol/punctuation/brackets.ptl`.
//!
//! Parentheses, brackets, braces, angle brackets, floors and ceilings. The
//! MATH size variants and assembly pieces are derived from these outlines by
//! `lemmata-build`, so a ported base is what lets them follow the plan's weight
//! and slope rather than one captured shape.
//!
//! The drawn assembly pieces at U+239B to U+23AD are not built here. They need
//! the OS/2 typo ascender and descender, which the PTL metrics do not carry,
//! and the table derives its pieces from the base for now.

#![allow(clippy::suboptimal_flops)]

use kurbo::Affine;
use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::control::Item;
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::shapes::{self, Design};
use lemmata_ptl::spiro_kit::{Direction, Family, Kit, Position};
use lemmata_ptl::util::mix;

/// The delimiters a definition builds.
pub struct Brackets<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

/// The `Paren.Dim` values after the design overrides are applied.
struct Dim {
    outside: f64,
    inside: f64,
    bottom: f64,
    top: f64,
    mid: f64,
}

/// One of the three paren shapes, as the design values it overrides.
#[derive(Clone, Copy)]
struct ParenCfg {
    outside: f64,
    inside: f64,
    outside_sw_adj: f64,
    inside_sw_adj: f64,
    curliness: f64,
    overshoot: f64,
}

impl Brackets<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    const fn normal(&self) -> ParenCfg {
        ParenCfg {
            outside: self.design.paren_outside,
            inside: self.design.paren_inside,
            outside_sw_adj: self.design.paren_outside_sw_adj,
            inside_sw_adj: self.design.paren_inside_sw_adj,
            curliness: self.design.paren_curliness,
            overshoot: self.design.paren_overshoot,
        }
    }

    fn large_contour(&self) -> ParenCfg {
        ParenCfg {
            outside: 0.25 * self.design.paren_outside,
            curliness: mix(1.0, self.design.paren_curliness, 0.5),
            ..self.normal()
        }
    }

    fn flat_arc(&self) -> ParenCfg {
        ParenCfg {
            outside: 0.5 * self.design.paren_outside,
            curliness: mix(1.0, self.design.paren_curliness, 0.5),
            ..self.normal()
        }
    }

    fn dim(&self, df: &DivFrame, sw: f64, delta: f64, cfg: &ParenCfg) -> Dim {
        let outside = mix(df.left_sb(), df.right_sb(), cfg.outside)
            - delta
            - self.hsw_to_v(cfg.outside_sw_adj) * sw
            + self.metrics.o;
        let inside =
            mix(df.left_sb(), df.right_sb(), cfg.inside) - self.hsw_to_v(cfg.inside_sw_adj) * sw;
        let bottom = mix(
            self.metrics.symbol_mid,
            self.metrics.paren_bot,
            1.0 + cfg.overshoot,
        ) - delta;
        let top = mix(
            self.metrics.symbol_mid,
            self.metrics.paren_top,
            1.0 + cfg.overshoot,
        ) + delta;

        Dim {
            outside,
            inside,
            bottom,
            top,
            mid: self.metrics.symbol_mid,
        }
    }

    /// `Paren.Shape`: one open di-spiro with a quadratic bend at each side.
    fn shape(&self, df: &DivFrame, sw: f64, cfg: &ParenCfg) -> Geometry {
        let dim = self.dim(df, sw, 0.0, cfg);
        self.kit
            .dispiro(vec![
                Item::Af(self.kit.widths_lhs(Some(sw))),
                self.kit.knot(KnotType::G4, dim.inside, dim.top, Vec::new()),
                self.kit
                    .quad_controls(1.0, 1.0 - cfg.curliness, 3, Vec::new()),
                self.kit
                    .knot(KnotType::G4, dim.outside, dim.mid, Vec::new()),
                self.kit.quad_controls(0.0, cfg.curliness, 3, Vec::new()),
                self.kit
                    .knot(KnotType::G4, dim.inside, dim.bottom, Vec::new()),
            ])
            .geometry(Affine::IDENTITY)
    }

    /// `Paren.FlatArcShape`: a straight stem with a shallow arch in the middle.
    fn shape_flat_arc(&self, df: &DivFrame, cfg: &ParenCfg) -> Geometry {
        let dim = self.dim(df, df.mvs, 0.0, cfg);
        self.kit
            .dispiro(vec![
                Item::Af(self.kit.widths_lhs(Some(df.mvs))),
                self.kit.pair(
                    Family::G4,
                    Direction::Left,
                    Position::Start,
                    dim.inside,
                    dim.top,
                    Vec::new(),
                ),
                self.kit.archv(None, None),
                self.kit.knot(
                    KnotType::Left,
                    dim.outside,
                    mix(dim.bottom, dim.top, 0.65),
                    Vec::new(),
                ),
                self.kit.knot(
                    KnotType::Right,
                    dim.outside,
                    mix(dim.bottom, dim.top, 0.35),
                    Vec::new(),
                ),
                self.kit.arcvh(None, None),
                self.kit.pair(
                    Family::G4,
                    Direction::Right,
                    Position::Start,
                    dim.inside,
                    dim.bottom,
                    Vec::new(),
                ),
            ])
            .geometry(Affine::IDENTITY)
    }

    /// `turned`: the 180 degree rotation that makes a right delimiter from a
    /// left one about the cell's centre and the symbol's middle.
    fn turned(&self, df: &DivFrame, geometry: &Geometry) -> Geometry {
        let (x, y) = (df.middle(), self.metrics.symbol_mid);
        let tf = Affine::translate((x, y))
            * Affine::scale_non_uniform(-1.0, -1.0)
            * Affine::translate((-x, -y));

        Geometry::Transformed {
            tf,
            geom: Box::new(geometry.clone()),
        }
    }

    /// A rectangle as a closed spiro outline, for a half mask.
    fn mask(&self, above: bool) -> Geometry {
        let far = self.metrics.very_far;
        let y = self.metrics.symbol_mid;
        let outline = if above {
            shapes::rect(self.kit, far, y, -far, far)
        } else {
            shapes::rect(self.kit, y, -far, -far, far)
        };

        Geometry::Spiro {
            gizmo: Affine::IDENTITY,
            closed: outline.closed,
            knots: outline.knots,
        }
    }

    fn half(&self, shape: &Geometry, above: bool) -> Geometry {
        lemmata_ptl::glyph::intersection(vec![shape.clone(), self.mask(above)])
    }

    /// The bracket's two verticals and its two bars.
    fn bracket(&self, top: f64, bottom: f64, bar_left: Option<f64>, ext: f64) -> Geometry {
        let metrics = self.metrics;
        let l = mix(metrics.sb, metrics.right_sb, self.design.bracket_outside)
            - self.hsw_to_v(self.design.bracket_outside_sw_adj) * metrics.stroke;
        let r = ext + mix(metrics.sb, metrics.right_sb, self.design.bracket_inside);
        let l = bar_left.unwrap_or(l);

        lemmata_ptl::glyph::union(vec![
            shapes::hbar_b(self.kit, metrics, l, r, bottom, None).geometry(Affine::IDENTITY),
            shapes::hbar_t(self.kit, metrics, l, r, top, None).geometry(Affine::IDENTITY),
            shapes::vbar_l(self.kit, metrics, l, bottom, top, metrics.stroke)
                .geometry(Affine::IDENTITY),
        ])
    }

    /// `Brace.Dim`: the centre line of the point and the radius of the bend.
    fn brace_dim(&self) -> (f64, f64) {
        let metrics = self.metrics;
        let centre = mix(
            metrics.sb,
            metrics.right_sb,
            mix(self.design.brace_inside, self.design.brace_outside, 0.5),
        );
        let radius = (mix(metrics.sb, metrics.right_sb, self.design.brace_inside) - centre)
            .min((metrics.paren_top - metrics.symbol_mid - metrics.stroke * 1.5) / 2.0);

        (centre, radius)
    }

    /// `Brace.UpperHalfShape`: from the top bar down to the point.
    fn brace_upper(&self, top: f64, bottom: f64, sw: f64, ext: f64) -> Geometry {
        let metrics = self.metrics;
        let (centre, radius) = self.brace_dim();
        let inside = mix(metrics.sb, metrics.right_sb, self.design.brace_inside);
        let tip = mix(metrics.sb, metrics.right_sb, self.design.brace_outside);
        let left = shapes::leftward();
        let down = shapes::downward(metrics.hv_contrast);

        self.kit
            .dispiro(vec![
                Item::Af(self.kit.widths_center_heading(Some(sw), left)),
                self.kit
                    .knot(KnotType::Left, inside + ext, top, self.kit.heading(left)),
                self.kit.knot(
                    KnotType::Right,
                    inside - self.kit.tiny,
                    top,
                    self.kit.heading(left),
                ),
                self.kit.archv(None, None),
                self.kit
                    .knot(KnotType::Left, centre, top - radius, self.kit.heading(down)),
                self.kit.knot(
                    KnotType::Right,
                    centre,
                    bottom + radius,
                    self.kit.heading(down),
                ),
                self.kit.arcvh(None, None),
                self.kit.pair(
                    Family::Straight,
                    Direction::Left,
                    Position::End,
                    tip,
                    bottom,
                    self.kit.heading(left),
                ),
            ])
            .geometry(Affine::IDENTITY)
    }

    /// `Brace.LowerHalfShape`, the same shape turned over the middle.
    fn brace_lower(&self, top: f64, bottom: f64, sw: f64, ext: f64) -> Geometry {
        let metrics = self.metrics;
        let (centre, radius) = self.brace_dim();
        let inside = mix(metrics.sb, metrics.right_sb, self.design.brace_inside);
        let tip = mix(metrics.sb, metrics.right_sb, self.design.brace_outside);
        let left = shapes::leftward();
        let up = shapes::upward(metrics.hv_contrast);

        self.kit
            .dispiro(vec![
                Item::Af(self.kit.widths_center_heading(Some(sw), left)),
                self.kit
                    .knot(KnotType::Left, inside + ext, bottom, self.kit.heading(left)),
                self.kit.knot(
                    KnotType::Right,
                    inside - self.kit.tiny,
                    bottom,
                    self.kit.heading(left),
                ),
                self.kit.archv(None, None),
                self.kit.knot(
                    KnotType::Left,
                    centre,
                    bottom + radius,
                    self.kit.heading(up),
                ),
                self.kit
                    .knot(KnotType::Right, centre, top - radius, self.kit.heading(up)),
                self.kit.arcvh(None, None),
                self.kit.pair(
                    Family::Straight,
                    Direction::Left,
                    Position::End,
                    tip,
                    top,
                    self.kit.heading(left),
                ),
            ])
            .geometry(Affine::IDENTITY)
    }

    /// `Brace.CurlyShape`: two mirrored halves with a flat point at the middle.
    fn brace_curly(&self, sw: f64, p_flat_in: f64, p_flat_out: f64, ext: f64) -> Geometry {
        let metrics = self.metrics;
        let hs = sw / 2.0;
        let inside = mix(metrics.sb, metrics.right_sb, self.design.brace_inside);
        let outside = mix(metrics.sb, metrics.right_sb, self.design.brace_outside);
        let m1 = mix(
            inside,
            outside,
            self.design.brace_curly_m1 * (1.0 + 0.5 * p_flat_in),
        );
        let m2 = mix(
            inside,
            outside,
            self.design.brace_curly_m2 * (1.0 + 0.5 * p_flat_out),
        );
        let limit = (metrics.paren_top - metrics.symbol_mid - sw) / 3.0 + hs;
        let radius1 = sw / 16.0 + mix((inside - m1).min(limit), sw, 0.75 * p_flat_in);
        let radius2 = sw / 16.0 + mix((m2 - outside).min(limit), sw, 0.75 * p_flat_out) - hs;
        let ess = mix(sw, metrics.ess_upper * sw / metrics.stroke, 0.25);
        let top = mix(
            metrics.symbol_mid,
            metrics.paren_top,
            1.0 + self.design.brace_overshoot,
        );
        let bottom = mix(
            metrics.symbol_mid,
            metrics.paren_bot,
            1.0 + self.design.brace_overshoot,
        );
        let flat_in = (0.1_f64).max(p_flat_in * (inside - outside));
        let flat_out = (0.1_f64).max(p_flat_out * (inside - outside));
        let fine = sw * metrics.c_thin;
        let left = shapes::leftward();
        let down = shapes::downward(metrics.hv_contrast);
        let up = shapes::upward(metrics.hv_contrast);
        let arch = Some(2.2 + p_flat_in);

        let upper = self.kit.dispiro(vec![
            Item::Af(self.kit.widths_center_heading(Some(sw), left)),
            self.kit.knot(
                KnotType::Left,
                inside + ext,
                top - hs,
                self.kit.heading(left),
            ),
            self.kit.knot(
                KnotType::Right,
                inside - flat_in,
                top - hs,
                self.kit.heading(left),
            ),
            self.kit.archv(None, arch),
            self.kit.pair(
                Family::G4,
                Direction::Down,
                Position::Mid,
                m1,
                top - radius1,
                self.kit.heading(down),
            ),
            self.kit
                .also_thru(0.5, 0.5, self.kit.widths_center(Some(ess))),
            self.kit.pair(
                Family::G4,
                Direction::Down,
                Position::Mid,
                m2,
                metrics.symbol_mid + radius2,
                self.kit.widths_center_heading(Some(sw), down),
            ),
            self.kit.arcvh(None, None),
            self.kit.knot(
                KnotType::Left,
                outside + flat_out,
                metrics.symbol_mid + (sw - fine) / 2.0,
                self.kit.widths_center_heading(Some(fine), left),
            ),
            self.kit.knot(
                KnotType::Right,
                outside,
                metrics.symbol_mid + (sw - fine) / 2.0,
                self.kit.heading(left),
            ),
        ]);

        let lower = self.kit.dispiro(vec![
            Item::Af(self.kit.widths_center_heading(Some(sw), left)),
            self.kit.knot(
                KnotType::Left,
                inside + ext,
                bottom + hs,
                self.kit.heading(left),
            ),
            self.kit.knot(
                KnotType::Right,
                inside - flat_in,
                bottom + hs,
                self.kit.heading(left),
            ),
            self.kit.archv(None, arch),
            self.kit.pair(
                Family::G4,
                Direction::Up,
                Position::Mid,
                m1,
                bottom + radius1,
                self.kit.heading(up),
            ),
            self.kit
                .also_thru(0.5, 0.5, self.kit.widths_center(Some(ess))),
            self.kit.pair(
                Family::G4,
                Direction::Up,
                Position::Mid,
                m2,
                metrics.symbol_mid - radius2,
                self.kit.widths_center_heading(Some(sw), up),
            ),
            self.kit.arcvh(None, None),
            self.kit.knot(
                KnotType::Left,
                outside + flat_out,
                metrics.symbol_mid - (sw - fine) / 2.0,
                self.kit.widths_center_heading(Some(fine), left),
            ),
            self.kit.knot(
                KnotType::Right,
                outside,
                metrics.symbol_mid - (sw - fine) / 2.0,
                self.kit.heading(left),
            ),
        ]);

        Geometry::Combine(vec![
            upper.geometry(Affine::IDENTITY),
            lower.geometry(Affine::IDENTITY),
        ])
    }

    /// The angle bracket's two arms, meeting at the outside point.
    fn angle_left(&self, outside: f64, inside: f64, sw: f64) -> Geometry {
        let metrics = self.metrics;
        let fatten = f64::hypot(
            1.0,
            (inside - outside - metrics.o) / (metrics.paren_top - metrics.symbol_mid),
        );
        let down = shapes::downward(metrics.hv_contrast);
        let up = shapes::upward(metrics.hv_contrast);

        let upper = self.kit.dispiro(vec![
            Item::Af(self.kit.widths_lhs(Some(sw))),
            self.kit
                .knot(KnotType::Left, inside, metrics.paren_top, Vec::new()),
            self.kit.knot(
                KnotType::Right,
                outside + metrics.o,
                metrics.symbol_mid,
                self.kit.widths_heading(sw * fatten, 0.0, down),
            ),
        ]);
        let lower = self.kit.dispiro(vec![
            Item::Af(self.kit.widths_rhs(Some(sw))),
            self.kit
                .knot(KnotType::Left, inside, metrics.paren_bot, Vec::new()),
            self.kit.knot(
                KnotType::Right,
                outside + metrics.o,
                metrics.symbol_mid,
                self.kit.widths_heading(0.0, sw * fatten, up),
            ),
        ]);

        lemmata_ptl::glyph::union(vec![
            upper.geometry(Affine::IDENTITY),
            lower.geometry(Affine::IDENTITY),
        ])
    }

    /// The glyphs this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let df = self.df;
        let metrics = self.metrics;
        let normal = self.normal();
        let large = self.large_contour();
        let flat = self.flat_arc();
        let mut out = Vec::new();

        let normal_left = self.shape(df, df.mvs, &normal);
        let large_left = self.shape(df, df.mvs, &large);
        let flat_left = self.shape_flat_arc(df, &flat);
        let normal_right = self.turned(df, &normal_left);
        let large_right = self.turned(df, &large_left);
        let flat_right = self.turned(df, &flat_left);

        out.push(("parenLeft.normal".to_string(), normal_left.clone()));
        out.push(("parenRight.normal".to_string(), normal_right.clone()));
        out.push(("parenLeft.largeContour".to_string(), large_left));
        out.push(("parenRight.largeContour".to_string(), large_right));
        out.push(("parenLeft.flatArc".to_string(), flat_left.clone()));
        out.push(("parenRight.flatArc".to_string(), flat_right.clone()));

        out.push(("parenLeft".to_string(), normal_left.clone()));
        out.push(("parenRight".to_string(), normal_right.clone()));
        out.push(("flattenedParenLeft".to_string(), flat_left));
        out.push(("flattenedParenRight".to_string(), flat_right));

        out.push(("parenLeftUHalf".to_string(), self.half(&normal_left, true)));
        out.push(("parenLeftLHalf".to_string(), self.half(&normal_left, false)));
        out.push((
            "parenRightUHalf".to_string(),
            self.half(&normal_right, true),
        ));
        out.push((
            "parenRightLHalf".to_string(),
            self.half(&normal_right, false),
        ));

        // The double parens sit in a three-stroke frame sliced to two.
        let wide = DivFrame::new(metrics.clone(), Some(1.0), Some(3.0), None);
        let sub = wide.slice(Some(3.0), Some(2.0), None);
        let first = self.shape(&sub, sub.mvs, &normal);
        let shift = Affine::translate((wide.width - sub.width, 0.0));
        let double = Geometry::Combine(vec![
            first.clone(),
            Geometry::Transformed {
                tf: shift,
                geom: Box::new(first),
            },
        ]);
        out.push(("doubleParenLeft.normal".to_string(), double.clone()));
        out.push((
            "doubleParenRight.normal".to_string(),
            self.turned(df, &double),
        ));

        // Brackets.
        let bracket_left = self.bracket(metrics.paren_top, metrics.paren_bot, None, 0.0);
        let bracket_right = self.turned(df, &bracket_left);
        out.push(("bracketLeft".to_string(), bracket_left.clone()));
        out.push(("bracketRight".to_string(), bracket_right.clone()));
        out.push((
            "bracketLeftUHalf".to_string(),
            self.half(&bracket_left, true),
        ));
        out.push((
            "bracketLeftLHalf".to_string(),
            self.half(&bracket_left, false),
        ));
        out.push((
            "bracketRightUHalf".to_string(),
            self.half(&bracket_right, true),
        ));
        out.push((
            "bracketRightLHalf".to_string(),
            self.half(&bracket_right, false),
        ));

        let dbl_left = self.double_bracket_left();
        out.push(("dblBracketLeft".to_string(), dbl_left.clone()));
        out.push(("dblBracketRight".to_string(), self.turned(df, &dbl_left)));

        let bar_left = self.bracket_bar_left();
        out.push(("bracketBarLeft".to_string(), bar_left.clone()));
        out.push(("bracketBarRight".to_string(), self.turned(df, &bar_left)));

        let bar_bar_left = self.bar_bar_left();
        out.push(("barBarLeft".to_string(), bar_bar_left.clone()));
        out.push(("barBarRight".to_string(), self.turned(df, &bar_bar_left)));

        // Braces. The selected `braceLeft` is the straight pair, which is the
        // first rank in `params/variants.toml`.
        let top = mix(
            metrics.symbol_mid,
            metrics.paren_top,
            1.0 + self.design.brace_overshoot,
        ) - metrics.half_stroke;
        let bottom = mix(
            metrics.symbol_mid,
            metrics.paren_bot,
            1.0 + self.design.brace_overshoot,
        ) + metrics.half_stroke;

        let brace_left = Geometry::Combine(vec![
            self.brace_upper(top, metrics.symbol_mid, metrics.stroke, 0.0),
            self.brace_lower(metrics.symbol_mid, bottom, metrics.stroke, 0.0),
        ]);
        let brace_right = self.turned(df, &brace_left);
        out.push(("braceLeft.straight".to_string(), brace_left.clone()));
        out.push(("braceRight.straight".to_string(), brace_right.clone()));
        out.push(("braceLeft".to_string(), brace_left));
        out.push(("braceRight".to_string(), brace_right));

        let curly_left = self.brace_curly(metrics.stroke, 0.0, 0.0, 0.0);
        out.push(("braceLeft.curly".to_string(), curly_left.clone()));
        out.push(("braceRight.curly".to_string(), self.turned(df, &curly_left)));

        let flat_curly = self.brace_curly(metrics.stroke, 0.25, 0.05, 0.0);
        out.push((
            "braceLeft.curlyFlatBoundary".to_string(),
            flat_curly.clone(),
        ));
        out.push((
            "braceRight.curlyFlatBoundary".to_string(),
            self.turned(df, &flat_curly),
        ));

        // Angle brackets.
        let angle = self.angle_left(
            mix(metrics.sb, metrics.right_sb, self.design.paren_outside),
            mix(metrics.sb, metrics.right_sb, self.design.paren_inside),
            metrics.operator_stroke,
        );
        let angle_right = self.turned(df, &angle);
        out.push(("angleLeft".to_string(), angle.clone()));
        out.push(("angleRight".to_string(), angle_right.clone()));
        out.push(("mathAngleLeft".to_string(), angle));
        out.push(("mathAngleRight".to_string(), angle_right));

        // Floors and ceilings are one bar and one stem, not a full bracket.
        let ceiling_left = lemmata_ptl::glyph::union(vec![
            shapes::hbar_t(
                self.kit,
                metrics,
                mix(metrics.sb, metrics.right_sb, self.design.bracket_outside),
                mix(metrics.sb, metrics.right_sb, self.design.bracket_inside),
                metrics.paren_top,
                None,
            )
            .geometry(Affine::IDENTITY),
            shapes::vbar_l(
                self.kit,
                metrics,
                mix(metrics.sb, metrics.right_sb, self.design.bracket_outside),
                metrics.paren_bot,
                metrics.paren_top,
                metrics.stroke,
            )
            .geometry(Affine::IDENTITY),
        ]);
        let floor_left = lemmata_ptl::glyph::union(vec![
            shapes::hbar_b(
                self.kit,
                metrics,
                mix(metrics.sb, metrics.right_sb, self.design.bracket_outside),
                mix(metrics.sb, metrics.right_sb, self.design.bracket_inside),
                metrics.paren_bot,
                None,
            )
            .geometry(Affine::IDENTITY),
            shapes::vbar_l(
                self.kit,
                metrics,
                mix(metrics.sb, metrics.right_sb, self.design.bracket_outside),
                metrics.paren_bot,
                metrics.paren_top,
                metrics.stroke,
            )
            .geometry(Affine::IDENTITY),
        ]);
        out.push(("ceilingLeft".to_string(), ceiling_left.clone()));
        out.push(("ceilingRight".to_string(), self.turned(df, &ceiling_left)));
        out.push(("floorLeft".to_string(), floor_left.clone()));
        out.push(("floorRight".to_string(), self.turned(df, &floor_left)));

        out
    }

    /// `dblBracketLeft`: two stems inside one pair of bars.
    fn double_bracket_left(&self) -> Geometry {
        let metrics = self.metrics;
        let l = mix(metrics.sb, metrics.right_sb, self.design.bracket_outside)
            - self.hsw_to_v(self.design.bracket_outside_sw_adj) * metrics.stroke;
        let r = mix(metrics.sb, metrics.right_sb, self.design.bracket_inside);
        let sw = ((r - l) / 5.0).min(metrics.advice_stroke(4.0, 1.0, 1.0));

        lemmata_ptl::glyph::union(vec![
            shapes::hbar_b(self.kit, metrics, l, r, metrics.paren_bot, None)
                .geometry(Affine::IDENTITY),
            shapes::hbar_t(self.kit, metrics, l, r, metrics.paren_top, None)
                .geometry(Affine::IDENTITY),
            shapes::vbar_l(
                self.kit,
                metrics,
                l,
                metrics.paren_bot,
                metrics.paren_top,
                sw,
            )
            .geometry(Affine::IDENTITY),
            shapes::vbar_m(
                self.kit,
                metrics,
                mix(l, r, 0.5),
                metrics.paren_bot,
                metrics.paren_top,
                sw,
            )
            .geometry(Affine::IDENTITY),
        ])
    }

    /// `bracketBarLeft`: a bracket with a middle bar.
    fn bracket_bar_left(&self) -> Geometry {
        let metrics = self.metrics;
        let l = mix(metrics.sb, metrics.right_sb, self.design.bracket_outside)
            - self.hsw_to_v(self.design.bracket_outside_sw_adj) * metrics.stroke;
        let r = mix(metrics.sb, metrics.right_sb, self.design.bracket_inside);

        lemmata_ptl::glyph::union(vec![
            shapes::hbar_b(self.kit, metrics, l, r, metrics.paren_bot, None)
                .geometry(Affine::IDENTITY),
            shapes::hbar_m(
                self.kit,
                metrics,
                l,
                r,
                mix(metrics.paren_bot, metrics.paren_top, 0.5),
                None,
            )
            .geometry(Affine::IDENTITY),
            shapes::hbar_t(self.kit, metrics, l, r, metrics.paren_top, None)
                .geometry(Affine::IDENTITY),
            shapes::vbar_l(
                self.kit,
                metrics,
                l,
                metrics.paren_bot,
                metrics.paren_top,
                metrics.stroke,
            )
            .geometry(Affine::IDENTITY),
        ])
    }

    /// `barBarLeft`: one stem and one middle bar.
    fn bar_bar_left(&self) -> Geometry {
        let metrics = self.metrics;
        let l = mix(metrics.sb, metrics.right_sb, self.design.bracket_outside)
            - self.hsw_to_v(self.design.bracket_outside_sw_adj) * metrics.stroke;
        let r = mix(metrics.sb, metrics.right_sb, self.design.bracket_inside);

        lemmata_ptl::glyph::union(vec![
            shapes::hbar_m(
                self.kit,
                metrics,
                l,
                r,
                mix(metrics.paren_bot, metrics.paren_top, 0.5),
                None,
            )
            .geometry(Affine::IDENTITY),
            shapes::vbar_l(
                self.kit,
                metrics,
                l,
                metrics.paren_bot,
                metrics.paren_top,
                metrics.stroke,
            )
            .geometry(Affine::IDENTITY),
        ])
    }
}
