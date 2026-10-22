//! The lowercase k from `letter/latin/k.ptl`.
//!
//! Ported: the straight serifless form, which the default sans plan selects.
//! The curly, cursive and symmetric bodies, the bottom-right and top serifs and
//! the descender and palatal forms are not ported.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::{Geometry, KnotType};
use lemmata_ptl::{
    control::Item,
    metrics::Metrics,
    shapes::{self, Design},
    spiro_kit::Kit,
    util::mix,
};

/// The lowercase k forms this module builds.
pub struct LowerK<'a> {
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerK<'_> {
    /// `KLegs.Straight` with no serifs and no right hook.
    fn straight_serifless(&self) -> Geometry {
        let metrics = self.metrics;
        let top = metrics.xh;
        let stroke = metrics.stroke;
        let x_bar_left = metrics.sb + metrics.width / 32.0;
        let ksh_left = metrics.sb + metrics.width / 32.0;
        let ksh_right = metrics.right_sb + metrics.width / 32.0;
        let fine = metrics.advice_stroke(3.5, 1.0, 1.0);
        let ox = -4.0 * metrics.ox - stroke / 3.0;
        let y_attach1 = mix(0.0, top, 0.42) - stroke;
        let far = metrics.upm * 16.0;

        let stem = shapes::vbar_l(self.kit, metrics, x_bar_left, 0.0, metrics.ascender, stroke)
            .geometry(kurbo::Affine::IDENTITY);

        let arm = self
            .kit
            .dispiro(vec![
                Item::Af(self.kit.widths_rhs(Some(stroke))),
                self.kit
                    .knot(KnotType::Left, ksh_right - ox, top, Vec::new()),
                self.kit.knot(
                    KnotType::Right,
                    ksh_left + stroke,
                    y_attach1,
                    self.kit.widths_rhs(Some(fine)),
                ),
            ])
            .geometry(kurbo::Affine::IDENTITY);
        let mask = spiro_geometry(shapes::rect(self.kit, top, 0.0, ksh_left, far));
        let upper = lemmata_ptl::glyph::intersection(vec![mask.clone(), arm]);

        let lower_arm = self
            .kit
            .dispiro(vec![
                self.kit.knot(
                    KnotType::Left,
                    ksh_right - ox,
                    0.0,
                    self.kit.widths_lhs(Some(stroke)),
                ),
                self.kit.knot(
                    KnotType::Right,
                    ksh_left + stroke,
                    mix(0.0, top, 0.72) + stroke,
                    self.kit.widths_lhs(Some(fine)),
                ),
            ])
            .geometry(kurbo::Affine::IDENTITY);
        let union_mask = self.kit.spiro_outline(vec![
            self.kit
                .knot(KnotType::Corner, ksh_left + stroke, 0.0, Vec::new()),
            self.kit.knot(
                KnotType::Corner,
                ksh_left + stroke,
                y_attach1 + 1.0,
                Vec::new(),
            ),
            self.kit.knot(
                KnotType::Corner,
                ksh_right - ox - metrics.tiny,
                top,
                Vec::new(),
            ),
            self.kit
                .knot(KnotType::Corner, ksh_right - ox, top, Vec::new()),
            self.kit
                .knot(KnotType::Corner, ksh_right - ox, 0.0, Vec::new()),
        ]);
        let lower =
            lemmata_ptl::glyph::intersection(vec![mask, spiro_geometry(union_mask), lower_arm]);

        Geometry::Combine(vec![stem, upper, lower])
    }

    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        vec![("k.straightSerifless".to_string(), self.straight_serifless())]
    }
}

fn spiro_geometry(outline: lemmata_ptl::spiro_kit::SpiroOutline) -> Geometry {
    Geometry::Spiro {
        gizmo: kurbo::Affine::IDENTITY,
        closed: outline.closed,
        knots: outline.knots,
    }
}
