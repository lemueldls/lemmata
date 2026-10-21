//! The asterisk family from `symbol/punctuation/asterisk.ptl`.
//!
//! Every star is the same generator: `sides` petals from the symbol's middle to
//! a circle of radius `LongJut * 1.2`, each petal a two-knot di-spiro whose
//! widths taper from twice `fine` at the centre to twice `final` at the tip.
//! The table varies the number of sides, the rotation and the stroke.
//!
//! The `high`, `slashTop`, `halfXH` and `slashBot` forms translate the same
//! drawing. `dualAsterisk` stacks two copies and `asterism` stacks three, scaled
//! to the width kind it is built for. The selected `asterisk` and the bare
//! `asterism.NWID`/`WWID` names are copies of the default penta shape.
//!
//! `slavonicAsterisk` is not ported: it is a different drawing built from two
//! three-corner spiro outlines and a boolean difference.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops, clippy::cast_precision_loss)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::shapes::Design;
use lemmata_ptl::spiro_kit::Kit;
use lemmata_ptl::util::mix;

/// Which stroke a kind uses.
#[derive(Clone, Copy)]
enum StrokeSource {
    Stroke,
    Geometry,
    Advice(f64),
}

/// One row of `asteriskKinds`.
struct Kind {
    sides: usize,
    rot: f64,
    stroke: StrokeSource,
    name: &'static str,
    dual: Option<&'static str>,
    triple: Option<&'static str>,
    high: Option<&'static str>,
    st: Option<&'static str>,
    smid: &'static str,
    half_xh: Option<&'static str>,
    sb: Option<&'static str>,
}

/// The kinds the table builds, in its order.
const KINDS: &[Kind] = &[
    Kind {
        sides: 5,
        rot: 0.0,
        stroke: StrokeSource::Stroke,
        name: "asterisk",
        dual: Some("dualAsterisk"),
        triple: Some("asterism"),
        high: Some("pentaHigh"),
        st: Some("pentaST"),
        smid: "pentaSMid",
        half_xh: Some("pentaHalfXH"),
        sb: Some("pentaSB"),
    },
    Kind {
        sides: 6,
        rot: 0.0,
        stroke: StrokeSource::Stroke,
        name: "asterisk",
        dual: Some("dualAsterisk"),
        triple: Some("asterism"),
        high: Some("hexHigh"),
        st: Some("hexST"),
        smid: "hexSMid",
        half_xh: Some("hexHalfXH"),
        sb: Some("hexSB"),
    },
    Kind {
        sides: 5,
        rot: 1.0,
        stroke: StrokeSource::Stroke,
        name: "asterisk",
        dual: Some("dualAsterisk"),
        triple: Some("asterism"),
        high: Some("turnPentaHigh"),
        st: Some("turnPentaST"),
        smid: "turnPentaSMid",
        half_xh: Some("turnPentaHalfXH"),
        sb: Some("turnPentaSB"),
    },
    Kind {
        sides: 6,
        rot: 0.5,
        stroke: StrokeSource::Stroke,
        name: "asterisk",
        dual: Some("dualAsterisk"),
        triple: Some("asterism"),
        high: Some("turnHexHigh"),
        st: Some("turnHexST"),
        smid: "turnHexSMid",
        half_xh: Some("turnHexHalfXH"),
        sb: Some("turnHexSB"),
    },
    Kind {
        sides: 5,
        rot: 0.0,
        stroke: StrokeSource::Geometry,
        name: "opAsterisk",
        dual: None,
        triple: None,
        high: None,
        st: None,
        smid: "penta",
        half_xh: None,
        sb: None,
    },
    Kind {
        sides: 6,
        rot: 0.0,
        stroke: StrokeSource::Geometry,
        name: "opAsterisk",
        dual: None,
        triple: None,
        high: None,
        st: None,
        smid: "hex",
        half_xh: None,
        sb: None,
    },
    Kind {
        sides: 5,
        rot: 1.0,
        stroke: StrokeSource::Geometry,
        name: "opAsterisk",
        dual: None,
        triple: None,
        high: None,
        st: None,
        smid: "turnPenta",
        half_xh: None,
        sb: None,
    },
    Kind {
        sides: 6,
        rot: 0.5,
        stroke: StrokeSource::Geometry,
        name: "opAsterisk",
        dual: None,
        triple: None,
        high: None,
        st: None,
        smid: "turnHex",
        half_xh: None,
        sb: None,
    },
    Kind {
        sides: 8,
        rot: 0.5,
        stroke: StrokeSource::Advice(3.0),
        name: "flower",
        dual: None,
        triple: None,
        high: Some("high"),
        st: Some("ST"),
        smid: "sMid",
        half_xh: None,
        sb: None,
    },
];

/// `AsteriskKinds` also names the default penta shape, which the selected
/// `asterisk` and the bare `asterism.NWID`/`WWID` copies follow.
const DEFAULT_SMID: &str = "pentaSMid";

/// The asterisk family this module builds.
pub struct Asterisk<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl Asterisk<'_> {
    fn stroke(&self, source: StrokeSource) -> f64 {
        match source {
            StrokeSource::Stroke => self.metrics.stroke,
            StrokeSource::Geometry => self.metrics.geometry_stroke,
            StrokeSource::Advice(c) => self.metrics.advice_stroke(c, 1.0, 1.0),
        }
    }

    /// `yHigh` and the three other reference heights.
    fn heights(&self) -> (f64, f64, f64, f64) {
        let metrics = self.metrics;
        let y_high = metrics.paren_top - metrics.long_jut * 2.0;
        let y_high_dt = metrics.paren_top - metrics.long_jut * 1.5;
        let y_low = mix(metrics.symbol_mid, y_high, -1.0);
        let y_low_dt = mix(metrics.symbol_mid, y_high_dt, -1.0);

        (y_high, y_high_dt, y_low, y_low_dt)
    }

    /// The petals of one row, before any translation.
    fn base(&self, kind: &Kind) -> Geometry {
        let metrics = self.metrics;
        let middle = metrics.middle;
        let radius = metrics.long_jut * 1.2;
        let stroke = self.stroke(kind.stroke);
        let fine = stroke * 0.4;
        let tip = 0.5 * stroke.min(radius * std::f64::consts::PI / kind.sides as f64);
        let mut parts = Vec::with_capacity(kind.sides);
        for j in 0..kind.sides {
            let angle = (2.0 * j as f64 / kind.sides as f64 + kind.rot) * std::f64::consts::PI;
            let s = angle.sin();
            let c = angle.cos();
            parts.push(
                self.kit
                    .dispiro(vec![
                        self.kit.knot(
                            KnotType::Left,
                            middle,
                            metrics.symbol_mid,
                            self.kit.widths_center(Some(2.0 * fine)),
                        ),
                        self.kit.knot(
                            KnotType::Right,
                            middle + radius * s,
                            metrics.symbol_mid + radius * c,
                            self.kit.widths_center(Some(2.0 * tip)),
                        ),
                    ])
                    .geometry(kurbo::Affine::IDENTITY),
            );
        }

        Geometry::Combine(parts)
    }

    /// `ApparentTranslate 0 dy`.
    fn apparent_translate(&self, dy: f64) -> kurbo::Affine {
        kurbo::Affine::translate((self.metrics.tan_slope * dy, dy))
    }

    /// `ScaleAround Middle SymbolMid scale scale`.
    fn scale_around(&self, scale: f64) -> kurbo::Affine {
        let middle = self.metrics.middle;
        let symbol_mid = self.metrics.symbol_mid;

        kurbo::Affine::translate((middle, symbol_mid))
            * kurbo::Affine::scale(scale)
            * kurbo::Affine::translate((-middle, -symbol_mid))
    }

    fn translated(&self, base: &Geometry, dy: f64) -> Geometry {
        Geometry::Transformed {
            tf: self.apparent_translate(dy),
            geom: Box::new(base.clone()),
        }
    }

    /// The three stacked copies of `asterism`, at one width kind.
    fn triple(&self, base: &Geometry, wide: bool) -> Geometry {
        let metrics = self.metrics;
        let (_, y_high_dt, _, y_low_dt) = self.heights();
        let mosaic_width = if wide {
            metrics.wide_width0
        } else {
            metrics.width
        };
        let scalar: f64 = if wide { 2.0 } else { 1.0 };
        let scale = (scalar / 2.0).sqrt();
        let about = self.scale_around(scale);
        let mut parts = Vec::with_capacity(3);
        for (fraction, dy) in [
            (0.5, y_high_dt - metrics.symbol_mid),
            (0.75, y_low_dt - metrics.symbol_mid),
            (0.25, y_low_dt - metrics.symbol_mid),
        ] {
            let dx = mosaic_width * fraction - metrics.width / 2.0;
            let translation = kurbo::Affine::translate((dx + metrics.tan_slope * dy, dy));
            parts.push(Geometry::Transformed {
                tf: translation * about,
                geom: Box::new(base.clone()),
            });
        }

        Geometry::Combine(parts)
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let metrics = self.metrics;
        let (y_high, y_high_dt, y_low, y_low_dt) = self.heights();
        let mut out = Vec::new();
        let mut default_base = None;
        for kind in KINDS {
            let base = self.base(kind);
            let smid = format!("{}.{}", kind.name, kind.smid);
            out.push((smid, base.clone()));
            if kind.smid == DEFAULT_SMID {
                default_base = Some(base.clone());
            }
            for (suffix, dy) in [
                (kind.high, 2.0 * (y_high - metrics.symbol_mid)),
                (kind.st, y_high - metrics.symbol_mid),
                (kind.half_xh, metrics.xh / 2.0 - metrics.symbol_mid),
                (kind.sb, y_low - metrics.symbol_mid),
            ] {
                if let Some(suffix) = suffix {
                    out.push((
                        format!("{}.{suffix}", kind.name),
                        self.translated(&base, dy),
                    ));
                }
            }
            if let Some(dual) = kind.dual {
                out.push((
                    format!("{dual}.{}", kind.smid),
                    Geometry::Combine(vec![
                        self.translated(&base, y_high_dt - metrics.symbol_mid),
                        self.translated(&base, y_low_dt - metrics.symbol_mid),
                    ]),
                ));
            }
            if let Some(triple) = kind.triple {
                for (suffix, wide) in [("NWID", false), ("WWID", true)] {
                    out.push((
                        format!("{triple}.{suffix}.{}", kind.smid),
                        self.triple(&base, wide),
                    ));
                }
            }
        }

        if let Some(base) = default_base {
            for (suffix, wide) in [("NWID", false), ("WWID", true)] {
                out.push((format!("asterism.{suffix}"), self.triple(&base, wide)));
            }
        }

        out
    }
}
