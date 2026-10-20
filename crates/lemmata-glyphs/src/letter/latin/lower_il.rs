//! The lower-case `i` and `l` from `letter/latin/lower-il.ptl`.
//!
//! One table drives both letters: a stem, a serif set and a centre. Ported here
//! are the serifless, serifed and bottom-hooked variants, which need only the
//! vertical bar and the horizontal serif. The hooks, tails, decompressed forms
//! and the phonetic forms need `CurlyTail`, `DiagTail` or `HalfRingDescender`
//! and are not ported. The anchors are not either.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::shapes::{self, Design, HSerifSide};
use lemmata_ptl::spiro_kit::Kit;
use lemmata_ptl::util::mix;

/// The three bodies ported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Serifless,
    Serifed,
    HookyBottom,
}

/// The variants, in the table's order.
pub const VARIANTS: &[(&str, Body)] = &[
    ("serifless", Body::Serifless),
    ("serifed", Body::Serifed),
    ("hookyBottom", Body::HookyBottom),
];

/// The lower-case `i` and `l` under construction.
pub struct LowerIl<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerIl<'_> {
    fn hsw_to_v(&self, value: f64) -> f64 {
        value * self.metrics.hv_contrast
    }

    fn middle(&self) -> f64 {
        self.df.width / 2.0
    }

    /// `IBalance`: the serifed stem's offset.
    fn i_balance(&self) -> f64 {
        self.df.adws() * self.df.adws() * (self.metrics.long_jut * 0.04)
    }

    /// `IBalance2`: the hooked stem's offset.
    fn i_balance2(&self) -> f64 {
        self.df.adws() * (self.metrics.long_jut * 0.14)
    }

    /// `XMiddleT`.
    fn x_middle(&self, body: Body) -> f64 {
        match body {
            Body::Serifless => self.middle(),
            Body::Serifed => self.middle() + self.i_balance(),
            Body::HookyBottom => self.middle() - self.i_balance2(),
        }
    }

    /// The stem and the body's serif.
    #[must_use]
    pub fn shape(&self, body: Body, top: f64, bot: f64) -> Geometry {
        let metrics = self.metrics;
        let stroke = metrics.stroke;
        let x = self.x_middle(body);
        let mut parts = vec![
            shapes::vbar_m(self.kit, metrics, x, bot, top, stroke)
                .geometry(kurbo::Affine::IDENTITY),
        ];
        match body {
            Body::Serifless => {}
            Body::HookyBottom => {
                let jut = metrics.jut.max(mix(
                    self.hsw_to_v(0.5 * stroke),
                    metrics.long_jut,
                    self.df.adws(),
                ));
                parts.push(Geometry::Tagged(Box::new(
                    shapes::hserif(
                        self.kit,
                        metrics,
                        HSerifSide::RightBottom,
                        x,
                        bot,
                        jut,
                        stroke,
                        None,
                    )
                    .geometry(kurbo::Affine::IDENTITY),
                )));
            }
            Body::Serifed => {
                let jut = metrics.mid_jut_center.max(mix(
                    self.hsw_to_v(0.5 * stroke),
                    metrics.long_jut,
                    self.df.adws(),
                ));
                parts.push(Geometry::Tagged(Box::new(
                    shapes::hserif_mb(self.kit, metrics, self.middle(), bot, jut, Some(stroke))
                        .geometry(kurbo::Affine::IDENTITY),
                )));
                // `ILSerifs.Serifed`: the top serif, pulled in by however far
                // the stem sits off the middle.
                let top_jut = metrics.jut.max(
                    mix(
                        self.hsw_to_v(0.5 * stroke),
                        metrics.long_jut,
                        self.df.adws(),
                    ) - (x - self.middle()),
                );
                parts.push(Geometry::Tagged(Box::new(
                    shapes::hserif(
                        self.kit,
                        metrics,
                        HSerifSide::LeftTop,
                        x,
                        top,
                        top_jut,
                        stroke,
                        None,
                    )
                    .geometry(kurbo::Affine::IDENTITY),
                )));
            }
        }

        Geometry::Combine(parts)
    }

    /// The four families, each at the height its table names.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let metrics = self.metrics;
        let mut out = Vec::new();
        for (suffix, body) in VARIANTS {
            out.push((
                format!("dotlessi.{suffix}"),
                self.shape(*body, metrics.xh, 0.0),
            ));
            out.push((
                format!("dotlessiCap.{suffix}"),
                self.shape(*body, metrics.cap, 0.0),
            ));
            out.push((
                format!("l.{suffix}"),
                self.shape(*body, metrics.ascender, 0.0),
            ));
            out.push((
                format!("cyrl/palochka.{suffix}"),
                self.shape(*body, metrics.ascender, 0.0),
            ));
        }

        out
    }
}
