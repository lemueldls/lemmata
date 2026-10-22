//! The ASCII quotes from `symbol/punctuation/quotes-and-primes.ptl`.
//!
//! Ported: the straight single quote and the double quote built from it. The
//! curly, low, reversed and prime forms, the selectors and the stress marks are
//! not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    letter_model::DivFrame,
    metrics::Metrics,
    shapes::{Design, downward},
    spiro_kit::Kit,
};

/// The quote glyphs this module builds.
pub struct Quotes<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl Quotes<'_> {
    fn df(&self) -> DivFrame {
        DivFrame::new(self.metrics.clone(), Some(1.0), None, None)
    }

    /// `StraightQuoteShape`.
    fn straight(&self, df: &DivFrame, top: f64, bottom: f64) -> Geometry {
        let metrics = self.metrics;
        let heading = downward(metrics.hv_contrast);

        self.kit
            .dispiro(vec![
                lemmata_ptl::control::Item::Af(self.kit.widths_center(Some(metrics.stroke * 1.1))),
                self.kit.pair(
                    lemmata_ptl::spiro_kit::Family::G4,
                    lemmata_ptl::spiro_kit::Direction::Down,
                    lemmata_ptl::spiro_kit::Position::Start,
                    df.middle(),
                    top,
                    self.kit.heading(heading.clone()),
                ),
                self.kit.knot(
                    KnotType::G4,
                    df.middle(),
                    bottom,
                    self.kit
                        .widths_center_heading(Some(metrics.stroke * 0.95), heading),
                ),
            ])
            .geometry(kurbo::Affine::IDENTITY)
    }

    /// `asciiSingleQuote`.
    fn single(&self) -> Geometry {
        let df = self.df();

        self.straight(&df, self.metrics.cap * 1.05, self.metrics.xh * 0.85)
    }

    /// `asciiDoubleQuote`, two singles either side of the middle.
    fn double(&self) -> Geometry {
        let metrics = self.metrics;
        let dist = 0.2 * metrics.width;

        Geometry::Combine(vec![
            Geometry::Transformed {
                tf: kurbo::Affine::translate((-dist, 0.0)),
                geom: Box::new(self.single()),
            },
            Geometry::Transformed {
                tf: kurbo::Affine::translate((dist, 0.0)),
                geom: Box::new(self.single()),
            },
        ])
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        vec![
            ("asciiSingleQuote".to_string(), self.single()),
            ("asciiDoubleQuote".to_string(), self.double()),
        ]
    }
}
