//! The arrow-bar ligature glyphs from `symbol/ligation.ptl`.
//!
//! Only the `plus`, `hyphen`, `equal` and `ident` bars are ported. They are the
//! rods of the arrow, comparison and comment sequences. Waves, tildes and the
//! rest of the ligation file are not ported yet.
//!
//! The names are upstream's source names, before the build renames generated
//! glyphs to `.gNNNNN.join-*`. The fixture maps each captured target back to one
//! of these so a build can draw it for the plan instead of replaying the
//! reference outline.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::glyph;
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::shapes::{self, Design};
use lemmata_ptl::spiro_kit::Kit;
use lemmata_ptl::util::mix;

/// `DesignParameters.equal_wideness`, from `meta/aesthetics.ptl`.
const EQUAL_WIDENESS: f64 = 0.075;

/// One bar family's geometry.
#[derive(Clone, Copy)]
enum Bar {
    Plus,
    Hyphen,
    Equal,
    Ident,
}

impl Bar {
    const fn name(self) -> &'static str {
        match self {
            Self::Plus => "plus",
            Self::Hyphen => "hyphen",
            Self::Equal => "equal",
            Self::Ident => "ident",
        }
    }

    const fn double(self) -> bool {
        matches!(self, Self::Equal | Self::Ident)
    }
}

/// The arrow-bar ligature glyphs this module builds.
pub struct Ligation<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl Ligation<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    fn right_sb(&self) -> f64 {
        self.metrics.width - self.metrics.sb
    }

    /// `EqualHalfSpace`.
    fn equal_half_space(&self) -> f64 {
        (self.metrics.oper_top - self.metrics.oper_bot) * 2.0 * EQUAL_WIDENESS
    }

    /// `IdentHalfSpace`.
    fn ident_half_space(&self) -> f64 {
        self.equal_half_space() * 1.5
    }

    /// `EqualHoleWidth`.
    fn equal_hole_width(&self) -> f64 {
        self.metrics.advice_stroke(5.0, 1.0, 1.0)
    }

    /// `LessSlope`.
    fn less_slope(&self) -> f64 {
        (5.0 / 13.0) * (self.metrics.oper_top - self.metrics.oper_bot)
            / (self.right_sb() - self.metrics.sb)
    }

    /// `HyphenNotch`: the two holes a bar leaves when it crosses a neighbour.
    fn notch(&self, x: f64) -> Geometry {
        let metrics = self.metrics;
        let stroke = metrics.operator_stroke;
        let width = self.equal_hole_width();
        let mid = metrics.symbol_mid;

        glyph::union(vec![
            shapes::vbar_m(
                self.kit,
                metrics,
                x,
                mid + stroke / 3.0,
                mid + stroke,
                width,
            )
            .geometry(kurbo::Affine::IDENTITY),
            shapes::vbar_m(
                self.kit,
                metrics,
                x,
                mid - stroke / 3.0,
                mid - stroke,
                width,
            )
            .geometry(kurbo::Affine::IDENTITY),
        ])
    }

    /// `JunctDiamond`: the diamond that joins a bar at `x`.
    fn junction(&self, x: f64, y: f64, sw: f64) -> Geometry {
        let half = self.hsw_to_v(0.5 * sw);
        spiro_geometry(self.kit.spiro_outline(vec![
            self.kit
                .knot(lemmata_geom::KnotType::Corner, x - half, y, Vec::new()),
            self.kit.knot(
                lemmata_geom::KnotType::Corner,
                x,
                y + 0.5 * sw,
                Vec::new(),
            ),
            self.kit.knot(
                lemmata_geom::KnotType::Corner,
                x + half,
                y,
                Vec::new(),
            ),
            self.kit.knot(
                lemmata_geom::KnotType::Corner,
                x,
                y - 0.5 * sw,
                Vec::new(),
            ),
        ]))
    }

    /// The two or three bars a family draws between `left` and `right`.
    fn bar_shape(&self, bar: Bar, left: f64, right: f64) -> Geometry {
        let metrics = self.metrics;
        let mid = metrics.symbol_mid;
        let stroke = metrics.operator_stroke;
        let hbar = |y: f64| {
            shapes::hbar_m(self.kit, metrics, left, right, y, Some(stroke))
                .geometry(kurbo::Affine::IDENTITY)
        };

        match bar {
            Bar::Plus => glyph::union(vec![
                hbar(mid),
                shapes::vbar_m(
                    self.kit,
                    metrics,
                    metrics.middle,
                    metrics.plus_top,
                    metrics.plus_bot,
                    stroke,
                )
                .geometry(kurbo::Affine::IDENTITY),
            ]),
            Bar::Hyphen => hbar(mid),
            Bar::Equal => {
                let half = self.equal_half_space();
                glyph::union(vec![hbar(mid + half), hbar(mid - half)])
            }
            Bar::Ident => {
                let half = self.ident_half_space();
                glyph::union(vec![hbar(mid + half), hbar(mid), hbar(mid - half)])
            }
        }
    }

    /// `Shape.joinCap`: the diamonds at a bar's end.
    fn join_cap(&self, bar: Bar, x: f64) -> Geometry {
        let mid = self.metrics.symbol_mid;
        let stroke = self.metrics.operator_stroke;

        match bar {
            Bar::Plus | Bar::Hyphen => self.junction(x, mid, stroke),
            Bar::Equal => {
                let half = self.equal_half_space();
                glyph::union(vec![
                    self.junction(x, mid + half, stroke),
                    self.junction(x, mid - half, stroke),
                ])
            }
            Bar::Ident => {
                let half = self.ident_half_space();
                glyph::union(vec![
                    self.junction(x, mid + half, stroke),
                    self.junction(x, mid, stroke),
                    self.junction(x, mid - half, stroke),
                ])
            }
        }
    }

    /// `HyphenNotch` or `EqualHole`/`IdentHole` used against a neighbour.
    fn hole(&self, bar: Bar, x: f64) -> Geometry {
        let metrics = self.metrics;
        let half = match bar {
            Bar::Equal => self.equal_half_space(),
            Bar::Ident => self.ident_half_space(),
            _ => 0.0,
        };

        if matches!(bar, Bar::Equal | Bar::Ident) {
            shapes::vbar_m(
                self.kit,
                metrics,
                x,
                metrics.symbol_mid - half,
                metrics.symbol_mid + half,
                self.equal_hole_width(),
            )
            .geometry(kurbo::Affine::IDENTITY)
        } else {
            self.notch(x)
        }
    }

    /// `ArrowBarConfig`: the left and right classes and positions.
    fn arrow_bar_config(&self, bar: Bar) -> Vec<(String, f64, bool, String, f64, bool)> {
        let metrics = self.metrics;
        let width = metrics.width;
        let sb = metrics.sb;
        let right_sb = self.right_sb();
        let stroke = metrics.operator_stroke;
        let slope = self.less_slope();
        let arrowhead_slope = 1.0 / slope;
        let double = bar.double();
        let extend = if double {
            (right_sb - self.equal_half_space() * arrowhead_slope).clamp(0.0, width)
        } else {
            (right_sb - 0.5 * stroke * arrowhead_slope).clamp(0.0, width)
        };
        let join = if double {
            (sb + self.equal_half_space() * arrowhead_slope).clamp(0.0, width)
        } else {
            (sb + 0.5 * stroke * arrowhead_slope).clamp(0.0, width)
        };
        let kern = (2.0 / 3.0) * (width - stroke * 3.0).clamp(0.0, width * 0.4);
        let rod_gap = (width / 8.0).max(metrics.advice_stroke(6.0, 1.0, 1.0));
        let rod_offset = 0.5 * width - self.hsw_to_v(metrics.half_stroke) - rod_gap;
        let (arrow_right, arrow_left) = if double {
            (
                width + sb + self.hsw_to_v(stroke),
                -width + right_sb - self.hsw_to_v(stroke),
            )
        } else {
            (width + extend, -extend)
        };

        let lk: [(&str, f64, bool); 11] = [
            ("f", sb, false),
            ("x", sb - width, false),
            ("y", -join - width, false),
            ("l", arrow_left, false),
            ("l1", arrow_left - kern, false),
            ("lx", mix(arrow_left, 0.0, 0.5), false),
            ("m", -rod_offset, false),
            ("j", -join, false),
            ("j1", -join - kern, false),
            ("c", 0.0, true),
            ("z", -width + right_sb, true),
        ];
        let rk: [(&str, f64, bool); 9] = [
            ("f", right_sb, false),
            ("p", arrow_right - width, false),
            ("r", arrow_right, false),
            ("r1", arrow_right + kern, false),
            ("m", width + rod_offset, false),
            ("j", width + join, false),
            ("j1", width + join + kern, false),
            ("c", width, true),
            ("z", width + sb, true),
        ];

        let mut out = Vec::with_capacity(lk.len() * rk.len());
        for (l_key, left, cap_left) in lk {
            for (r_key, right, cap_right) in rk {
                out.push((
                    l_key.to_string(),
                    left,
                    cap_left,
                    r_key.to_string(),
                    right,
                    cap_right,
                ));
            }
        }

        out
    }

    /// One `prefix.lig.<l><r>.<notch>`.
    fn ligature(
        &self,
        bar: Bar,
        left: f64,
        cap_left: bool,
        right: f64,
        cap_right: bool,
        notch: bool,
    ) -> Geometry {
        let mut parts = vec![self.bar_shape(bar, left, right)];
        if cap_left {
            parts.push(self.join_cap(bar, left));
        }
        if cap_right {
            parts.push(self.join_cap(bar, right));
        }
        let mut shape = vec![glyph::union(parts)];
        if notch {
            let metrics = self.metrics;
            let width = metrics.width;
            let right_sb = self.right_sb();
            if left < metrics.sb - width {
                shape.push(self.hole(bar, -width));
            }
            if left < metrics.sb {
                shape.push(self.hole(bar, 0.0));
            }
            if right > right_sb {
                shape.push(self.hole(bar, width));
            }
            if right > right_sb + width {
                shape.push(self.hole(bar, 2.0 * width));
            }
        }

        if shape.len() == 1 {
            shape.pop().unwrap()
        } else {
            glyph::difference(shape)
        }
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for bar in [Bar::Plus, Bar::Hyphen, Bar::Equal, Bar::Ident] {
            for (l_key, left, cap_left, r_key, right, cap_right) in self.arrow_bar_config(bar) {
                for (suffix, notch) in [("withoutNotch", false), ("withNotch", true)] {
                    let glyph = self.ligature(bar, left, cap_left, right, cap_right, notch);
                    out.push((
                        format!("{}.lig.{l_key}{r_key}.{suffix}", bar.name()),
                        glyph.clone(),
                    ));
                    out.push((
                        format!("{}.arrowBar.{l_key}{r_key}.{suffix}", bar.name()),
                        glyph,
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
