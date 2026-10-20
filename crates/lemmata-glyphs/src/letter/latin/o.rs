//! The letter `o` from `letter/latin/o.ptl`, which also builds `O`.
//!
//! The file is one ring with many overlays. Ported here: `O`, `smcpO`, `o`, the
//! two slashes, the two barred forms and the two blackboard ones. The Cyrillic,
//! Polish, numeral and turned forms, the half rings and the horn compositions
//! need dots, transforms or glyphs outside the file and are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::control::Item;
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::{Metrics, WeightControl};
use lemmata_ptl::shapes::{self, Design};
use lemmata_ptl::spiro_kit::Kit;
use lemmata_ptl::util::mix;

/// The `o` families this module builds.
pub struct LatinO<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LatinO<'_> {
    const fn left_sb(&self) -> f64 {
        self.metrics.sb
    }

    const fn right_sb(&self) -> f64 {
        self.metrics.width - self.metrics.sb
    }

    fn weight(&self) -> WeightControl {
        WeightControl::new(
            self.metrics.stroke,
            self.metrics.width,
            self.metrics.sb,
            self.metrics.hv_contrast,
        )
    }

    fn ring(&self, top: f64, width: Option<f64>, ada: Option<f64>, adb: Option<f64>) -> Geometry {
        shapes::oshape(
            self.kit,
            self.metrics,
            self.design,
            top,
            0.0,
            self.left_sb(),
            self.right_sb(),
            width,
            ada,
            adb,
        )
        .geometry(kurbo::Affine::IDENTITY)
    }

    fn outline(
        &self,
        top: f64,
        width: Option<f64>,
        ada: Option<f64>,
        adb: Option<f64>,
    ) -> Geometry {
        let outline = shapes::oshape_outline_no_overshoot(
            self.kit,
            self.metrics,
            self.design,
            top,
            0.0,
            self.left_sb(),
            self.right_sb(),
            width,
            ada,
            adb,
        );

        Geometry::Spiro {
            gizmo: kurbo::Affine::IDENTITY,
            closed: outline.closed,
            knots: outline.knots,
        }
    }

    /// `O`.
    #[must_use]
    pub fn capital(&self) -> Geometry {
        self.ring(
            self.metrics.cap,
            None,
            Some(self.metrics.arch_depth_a),
            Some(self.metrics.arch_depth_b),
        )
    }

    /// `smcpO`.
    #[must_use]
    pub fn small_capital(&self) -> Geometry {
        self.ring(
            self.metrics.xh,
            None,
            Some(self.metrics.arch_depth_a),
            Some(self.metrics.arch_depth_b),
        )
    }

    /// `o`.
    #[must_use]
    pub fn small(&self) -> Geometry {
        self.ring(self.metrics.xh, None, None, None)
    }

    fn slash(&self, top: f64) -> Geometry {
        let metrics = self.metrics;
        let fine = 0.5 * metrics.overlay_stroke;
        self.kit
            .dispiro(vec![
                Item::Af(self.kit.widths_center(Some(2.0 * fine))),
                self.kit.knot(
                    KnotType::Left,
                    self.left_sb() + metrics.o + fine,
                    mix(top, 0.0, 1.05),
                    Vec::new(),
                ),
                self.kit.knot(
                    KnotType::Right,
                    self.right_sb() - metrics.o - fine,
                    mix(0.0, top, 1.05),
                    Vec::new(),
                ),
            ])
            .geometry(kurbo::Affine::IDENTITY)
    }

    /// `OSlash`.
    #[must_use]
    pub fn capital_slash(&self) -> Geometry {
        Geometry::Combine(vec![self.capital(), self.slash(self.metrics.cap)])
    }

    /// `oSlash`.
    #[must_use]
    pub fn small_slash(&self) -> Geometry {
        Geometry::Combine(vec![self.small(), self.slash(self.metrics.xh)])
    }

    fn barred(&self, top: f64, width: Option<f64>, ada: Option<f64>, adb: Option<f64>) -> Geometry {
        let metrics = self.metrics;
        let stroke = width.unwrap_or(metrics.stroke);
        let bar = shapes::hbar_m(
            self.kit,
            metrics,
            self.left_sb() + metrics.ox,
            self.right_sb() - metrics.ox,
            top * 0.5,
            Some(stroke),
        );

        Geometry::Combine(vec![
            self.ring(top, width, ada, adb),
            lemmata_ptl::glyph::intersection(vec![
                self.outline(top, width, ada, adb),
                bar.geometry(kurbo::Affine::IDENTITY),
            ]),
        ])
    }

    /// `OBar`.
    #[must_use]
    pub fn capital_bar(&self) -> Geometry {
        let stroke = self
            .weight()
            .advice_stroke2(2.0, 3.0, self.metrics.cap, 1.0);

        self.barred(
            self.metrics.cap,
            Some(stroke),
            Some(self.metrics.arch_depth_a),
            Some(self.metrics.arch_depth_b),
        )
    }

    /// `oBar`.
    #[must_use]
    pub fn small_bar(&self) -> Geometry {
        let stroke = self.weight().advice_stroke2(2.0, 3.0, self.metrics.xh, 1.0);

        self.barred(self.metrics.xh, Some(stroke), None, None)
    }

    fn blackboard(&self, top: f64, ada: Option<f64>, adb: Option<f64>) -> Geometry {
        let metrics = self.metrics;
        let bbs = shapes::bb_stroke(self.df);
        let bbd = shapes::bb_distance(metrics, bbs);
        let left = self.left_sb() + metrics.ox + bbd;
        let right = self.right_sb() - metrics.ox - bbd;
        let sides = lemmata_ptl::glyph::union(vec![
            shapes::vbar_l(self.kit, metrics, left, 0.0, top, bbs)
                .geometry(kurbo::Affine::IDENTITY),
            shapes::vbar_r(self.kit, metrics, right, 0.0, top, bbs)
                .geometry(kurbo::Affine::IDENTITY),
        ]);

        Geometry::Combine(vec![
            self.ring(top, Some(bbs), ada, adb),
            lemmata_ptl::glyph::intersection(vec![self.outline(top, Some(bbs), ada, adb), sides]),
        ])
    }

    /// `mathbb/O`.
    #[must_use]
    pub fn capital_blackboard(&self) -> Geometry {
        self.blackboard(
            self.metrics.cap,
            Some(self.metrics.arch_depth_a),
            Some(self.metrics.arch_depth_b),
        )
    }

    /// `mathbb/o`.
    #[must_use]
    pub fn small_blackboard(&self) -> Geometry {
        self.blackboard(self.metrics.xh, None, None)
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        vec![
            ("O".to_string(), self.capital()),
            ("smcpO".to_string(), self.small_capital()),
            ("o".to_string(), self.small()),
            ("OSlash".to_string(), self.capital_slash()),
            ("oSlash".to_string(), self.small_slash()),
            ("OBar".to_string(), self.capital_bar()),
            ("oBar".to_string(), self.small_bar()),
            ("mathbb/O".to_string(), self.capital_blackboard()),
            ("mathbb/o".to_string(), self.small_blackboard()),
        ]
    }
}
