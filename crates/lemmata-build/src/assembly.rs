//! Assembly pieces for the stretchy glyphs.
//!
//! r[impl math.stretch]
//!
//! A renderer builds a delimiter taller than any single glyph by stacking a
//! bottom piece, repeating a middle one, and capping it with a top piece. The
//! pieces here are the glyph cut into three horizontal bands, so the joins are
//! the cuts and the three parts tile the original outline exactly. They are
//! derived rather than drawn, like the size variants and for the same reason:
//! Iosevka ships no MATH table, so it draws no assembly pieces.
//!
//! The band is what makes a piece repeatable. A band whose ink is a straight
//! stem stacks on itself; a band taken from a curved or barred part of a glyph
//! repeats that feature, so an extender is only as good as the glyph's middle.
//! See `docs/math.md`.

use kurbo::{Rect, Shape};
use lemmata_geom::QuadifyPoint;

use crate::font::{GlyphData, glyph_path};

/// How much of the glyph's height the repeatable extender takes, centred on the
/// middle. A narrow band keeps the ink close to a straight bar, which is what
/// the piece has to be to stack on itself.
const BAND: f64 = 0.25;

/// The three pieces, bottom to top. The name is the suffix a piece carries.
pub const PIECES: [&str; 3] = ["a0", "a1", "a2"];

/// Where the repeatable piece sits in `PIECES`.
pub const EXTENDER: usize = 1;

/// The name a piece of `base` carries.
#[must_use]
pub fn piece_name(base: &str, index: usize) -> String {
    format!("{base}.{}", PIECES[index])
}

/// The glyph's ink between two heights, as quadratic contours.
///
/// The cut is a rectangle around the band, wide enough that only the two cut
/// lines decide the result, and clipping is the boolean engine's job rather
/// than this module's.
fn slice(glyph: &GlyphData, low: f64, high: f64) -> Vec<Vec<QuadifyPoint>> {
    let Some((x_min, _, x_max, _)) = glyph.bounding_box() else {
        return Vec::new();
    };
    let bounds = Rect::new(f64::from(x_min) - 1.0, low, f64::from(x_max) + 1.0, high);
    let Ok(paths) =
        lemmata_geom::boolean::intersection(&glyph_path(&glyph.contours), &bounds.to_path(0.1))
    else {
        return Vec::new();
    };

    lemmata_geom::quadify(&lemmata_geom::paths_to_shape(&paths))
}

/// The contours moved so their lowest point sits on the baseline.
///
/// The renderer stacks a part by its origin, so a piece's ink has to start at
/// zero for the part's advance to be the height the next one begins after.
fn grounded(contours: Vec<Vec<QuadifyPoint>>) -> Vec<Vec<QuadifyPoint>> {
    let low = contours
        .iter()
        .flatten()
        .fold(f64::INFINITY, |low, point| low.min(point.y));
    if !low.is_finite() {
        return contours;
    }

    contours
        .into_iter()
        .map(|contour| {
            contour
                .into_iter()
                .map(|point| QuadifyPoint {
                    y: point.y - low,
                    ..point
                })
                .collect()
        })
        .collect()
}

/// The pieces for every stretchy glyph, encoded from `first_code` upward.
///
/// The caller owns `first_code`, because the pieces share the private-use block
/// with the size variants and two derived glyphs must not share a code.
#[must_use]
pub fn derive(glyphs: &[GlyphData], first_code: u32) -> Vec<GlyphData> {
    let mut pieces: Vec<GlyphData> = Vec::new();
    for glyph in glyphs {
        if !glyph
            .unicodes
            .iter()
            .any(|code| crate::variants::STRETCHY.contains(code))
        {
            continue;
        }
        let Some((_, y_min, _, y_max)) = glyph.bounding_box() else {
            continue;
        };
        let height = f64::from(y_max - y_min);
        // Whole-unit cuts keep the joins on the grid the writer rounds to.
        let low = height.mul_add(0.5 - BAND / 2.0, f64::from(y_min)).round();
        let high = height.mul_add(0.5 + BAND / 2.0, f64::from(y_min)).round();
        // The outer bands reach past the ink so that only the inner cut decides
        // where they end.
        let bands = [
            (f64::from(y_min) - 1.0, low),
            (low, high),
            (high, f64::from(y_max) + 1.0),
        ];

        let mut cut: Vec<GlyphData> = bands
            .iter()
            .enumerate()
            .map(|(index, (band_low, band_high))| GlyphData {
                name: piece_name(&glyph.name, index),
                unicodes: Vec::new(),
                advance: glyph.advance,
                contours: grounded(slice(glyph, *band_low, *band_high)),
            })
            .collect();
        // A glyph with no ink in one of the bands has no assembly, and half an
        // assembly is worse than none: the table would name parts that do not
        // tile.
        if cut.iter().any(|piece| piece.contours.is_empty()) {
            continue;
        }
        for piece in &mut cut {
            let code = first_code + u32::try_from(pieces.len()).unwrap_or(0);
            piece.unicodes = vec![code];
            pieces.push(piece.clone());
        }
    }

    pieces
}
