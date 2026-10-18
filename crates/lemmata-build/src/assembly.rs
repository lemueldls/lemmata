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
//! The band is what makes a piece repeatable, so the cut is measured: a band is
//! kept only when its ink stays within the overlap the join uses, across its
//! whole height rather than at its two ends. The straightest of the candidate
//! bands wins, which is what moves the cut off a radical's arm and off a
//! brace's point. See `docs/math.md`.

use kurbo::{Rect, Shape};
use lemmata_geom::QuadifyPoint;

use crate::font::{GlyphData, glyph_path};

/// The bands a glyph's extender may be cut from, as the fraction of the height
/// it takes and where its centre sits.
///
/// The middle is tried first. A glyph whose lower middle carries a second
/// stroke, which is what a radical's arm is, repeats both if the band is taken
/// there, so the later candidates move up and narrow until the band holds one
/// stroke. A band that never gets there leaves the glyph without an assembly.
const BANDS: [(f64, f64); 4] = [(0.5, 0.25), (0.6, 0.25), (0.6, 0.15), (0.7, 0.15)];

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

/// The horizontal extent of a piece's ink.
fn x_range(contours: &[Vec<QuadifyPoint>]) -> Option<(f64, f64)> {
    let low = contours
        .iter()
        .flatten()
        .fold(f64::INFINITY, |low, point| low.min(point.x));
    let high = contours
        .iter()
        .flatten()
        .fold(f64::NEG_INFINITY, |high, point| high.max(point.x));

    (low.is_finite() && high.is_finite()).then_some((low, high))
}

/// How far a band's ink wanders sideways over its height.
///
/// A piece that stacks on itself has to be a bar, so this is what decides
/// whether a band can repeat, and zero means it repeats exactly. Comparing only
/// the two ends is not enough: a brace's middle holds its point, and the ink
/// bulges out and back within the band while both ends sit on the stem, so the
/// ends agree and the piece still scallops. The middle is sampled for that
/// reason.
fn spread(glyph: &GlyphData, low: f64, high: f64) -> f64 {
    let middle = f64::midpoint(low, high);
    let mut ranges = Vec::new();
    for probe in [low, middle - 2.0, high - 4.0] {
        let Some(range) = x_range(&slice(glyph, probe, probe + 4.0)) else {
            return f64::INFINITY;
        };
        ranges.push(range);
    }

    // The largest disagreement between the probes, at the left edge of the ink
    // and at the right.
    let across = |index: usize| {
        let values: Vec<f64> = ranges
            .iter()
            .map(|range| if index == 0 { range.0 } else { range.1 })
            .collect();
        values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - values.iter().copied().fold(f64::INFINITY, f64::min)
    };

    across(0).max(across(1))
}

/// The pieces for every stretchy glyph, encoded from `first_code` upward.
///
/// `overlap` is the connector the assembly joins along, which is also the step a
/// piece may take where it repeats: a step no larger than the overlap is covered
/// by the neighbouring piece's ink.
///
/// The caller owns `first_code`, because the pieces share the private-use block
/// with the size variants and two derived glyphs must not share a code.
#[must_use]
pub fn derive(glyphs: &[GlyphData], first_code: u32, overlap: u16) -> Vec<GlyphData> {
    let tolerance = f64::from(overlap);
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

        // Whole-unit cuts keep the joins on the grid the writer rounds to. The
        // straightest band wins rather than the first that fits, so a glyph
        // whose middle carries something else moves off it.
        let mut best: Option<(f64, f64, f64)> = None;
        for (centre, fraction) in BANDS {
            let low = height
                .mul_add(centre - fraction / 2.0, f64::from(y_min))
                .round();
            let high = height
                .mul_add(centre + fraction / 2.0, f64::from(y_min))
                .round();
            let wander = spread(glyph, low, high);
            if best.is_none_or(|(current, ..)| wander < current) {
                best = Some((wander, low, high));
            }
        }
        let Some((wander, low, high)) = best else {
            continue;
        };
        // A band that wanders further than the overlap it joins along cannot
        // repeat, and a glyph whose every band does leaves the table without an
        // assembly rather than with one that scallops.
        if wander > tolerance {
            continue;
        }

        // The outer bands reach past the ink so that only the inner cut decides
        // where they end.
        let bands = [
            (f64::from(y_min) - 1.0, low),
            (low, high),
            (high, f64::from(y_max) + 1.0),
        ];
        let mut pieces_of: Vec<GlyphData> = bands
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
        if pieces_of.iter().any(|piece| piece.contours.is_empty()) {
            continue;
        }
        for piece in &mut pieces_of {
            let code = first_code + u32::try_from(pieces.len()).unwrap_or(0);
            piece.unicodes = vec![code];
            pieces.push(piece.clone());
        }
    }

    pieces
}
