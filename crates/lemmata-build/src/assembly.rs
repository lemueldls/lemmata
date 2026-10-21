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
use lemmata_geom::{QuadifyKind, QuadifyPoint};

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
fn slice_y(glyph: &GlyphData, low: f64, high: f64) -> Vec<Vec<QuadifyPoint>> {
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
        let Some(range) = x_range(&slice_y(glyph, probe, probe + 4.0)) else {
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
/// `overlap` bounds how far a band's ink may wander over its height before the
/// piece is judged unable to repeat. It is half the connector the assembly joins
/// along, because a wander shows at the join as well as at the repeat, and a
/// step the neighbouring piece's ink does not cover is visible.
///
/// The caller owns `first_code`, because the pieces share the private-use block
/// with the size variants and two derived glyphs must not share a code.
#[must_use]
pub fn derive(glyphs: &[GlyphData], first_code: u32, overlap: u16) -> Vec<GlyphData> {
    let tolerance = f64::from(overlap) / 2.0;
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
        // middle is tried first and taken as soon as it is straight enough,
        // because moving the band up leaves the piece below it holding more of
        // the glyph than the piece above: a brace's point then lands at the
        // bottom of the assembly instead of in the middle. A later candidate is
        // only for a glyph whose middle carries something that cannot repeat.
        let mut cut: Option<(f64, f64)> = None;
        for (centre, fraction) in BANDS {
            let low = height
                .mul_add(centre - fraction / 2.0, f64::from(y_min))
                .round();
            let high = height
                .mul_add(centre + fraction / 2.0, f64::from(y_min))
                .round();
            if spread(glyph, low, high) <= tolerance {
                cut = Some((low, high));
                break;
            }
        }
        let Some((low, high)) = cut else {
            continue;
        };

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
                contours: grounded(slice_y(glyph, *band_low, *band_high)),
            })
            .collect();
        // A glyph with no ink in one of the bands has no assembly, and half an
        // assembly is worse than none: the table would name parts that do not
        // tile.
        if pieces_of.iter().any(|piece| piece.contours.is_empty()) {
            continue;
        }

        // The extender is the part a renderer repeats, so a band that still
        // curves shows every repeat as a step. Its own ink settles the bar:
        // the same width, from the same cut to the next, with straight sides.
        // The bar's width is the stroke's cross-section, not the band's
        // bounding box. A curved band is widest where its tangent is not
        // vertical, and using that width made the bar heavier than the curve it
        // joins. The midpoint is the straightest part of the band.
        let middle = f64::midpoint(low, high);
        if let (Some((x_low, x_high)), Some((y_low, y_high))) = (
            x_range(&slice_y(glyph, middle - 1.0, middle + 1.0)),
            y_range(&pieces_of[EXTENDER].contours),
        ) {
            pieces_of[EXTENDER].contours = vec![bar(x_low, x_high, y_low, y_high)];
        }
        for piece in &mut pieces_of {
            let code = first_code + u32::try_from(pieces.len()).unwrap_or(0);
            piece.unicodes = vec![code];
            pieces.push(piece.clone());
        }
    }

    pieces
}

/// The pieces of a horizontally stretched glyph.
pub const HORIZONTAL_PIECES: [&str; 3] = ["hp0", "hp1", "hp2"];

/// The name a horizontal piece of `base` carries.
#[must_use]
pub fn horizontal_piece_name(base: &str, index: usize) -> String {
    format!("{base}.{}", HORIZONTAL_PIECES[index])
}

/// The glyph's ink between two horizontal positions.
///
/// The mirror of `slice_y`: the cut is a rectangle around the column, tall
/// enough that only the two vertical cut lines decide the result.
fn slice_x(glyph: &GlyphData, low: f64, high: f64) -> Vec<Vec<QuadifyPoint>> {
    let Some((_, y_min, _, y_max)) = glyph.bounding_box() else {
        return Vec::new();
    };
    let bounds = Rect::new(low, f64::from(y_min) - 1.0, high, f64::from(y_max) + 1.0);
    let Ok(paths) =
        lemmata_geom::boolean::intersection(&glyph_path(&glyph.contours), &bounds.to_path(0.1))
    else {
        return Vec::new();
    };

    lemmata_geom::quadify(&lemmata_geom::paths_to_shape(&paths))
}

/// The contours moved so their left edge sits on the origin.
///
/// A horizontal assembly places each part at the advance before it, so a
/// piece's ink has to start at zero for the part's advance to be the width the
/// next one begins after.
fn grounded_x(contours: Vec<Vec<QuadifyPoint>>) -> Vec<Vec<QuadifyPoint>> {
    let low = contours
        .iter()
        .flatten()
        .fold(f64::INFINITY, |low, point| low.min(point.x));
    if !low.is_finite() {
        return contours;
    }

    contours
        .into_iter()
        .map(|contour| {
            contour
                .into_iter()
                .map(|point| QuadifyPoint {
                    x: point.x - low,
                    ..point
                })
                .collect()
        })
        .collect()
}

/// A closed four-corner rectangle: the extender's straight replacement.
fn bar(x_low: f64, x_high: f64, y_low: f64, y_high: f64) -> Vec<QuadifyPoint> {
    vec![
        QuadifyPoint {
            kind: QuadifyKind::Corner,
            x: x_low,
            y: y_low,
        },
        QuadifyPoint {
            kind: QuadifyKind::Corner,
            x: x_high,
            y: y_low,
        },
        QuadifyPoint {
            kind: QuadifyKind::Corner,
            x: x_high,
            y: y_high,
        },
        QuadifyPoint {
            kind: QuadifyKind::Corner,
            x: x_low,
            y: y_high,
        },
    ]
}

/// The vertical extent of a piece's ink.
fn y_range(contours: &[Vec<QuadifyPoint>]) -> Option<(f64, f64)> {
    let low = contours
        .iter()
        .flatten()
        .fold(f64::INFINITY, |low, point| low.min(point.y));
    let high = contours
        .iter()
        .flatten()
        .fold(f64::NEG_INFINITY, |high, point| high.max(point.y));

    (low.is_finite() && high.is_finite()).then_some((low, high))
}

/// How far a column's ink wanders up or down over its width.
fn spread_x(glyph: &GlyphData, low: f64, high: f64) -> f64 {
    let middle = f64::midpoint(low, high);
    let mut ranges = Vec::new();
    for probe in [low, middle - 2.0, high - 4.0] {
        let Some(range) = y_range(&slice_x(glyph, probe, probe + 4.0)) else {
            return f64::INFINITY;
        };
        ranges.push(range);
    }

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

/// The pieces for every horizontally stretched glyph, encoded from `first_code`
/// upward.
///
/// A horizontal arrow repeats a column of its shaft, and the same band rules
/// apply: the middle is tried first, and a column only repeats when its ink
/// holds its height across the width.
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn derive_horizontal(glyphs: &[GlyphData], first_code: u32, overlap: u16) -> Vec<GlyphData> {
    let tolerance = f64::from(overlap) / 2.0;
    let mut pieces: Vec<GlyphData> = Vec::new();
    for glyph in glyphs {
        if !glyph
            .unicodes
            .iter()
            .any(|code| crate::variants::HORIZONTAL.contains(code))
        {
            continue;
        }
        let Some((x_min, _, x_max, _)) = glyph.bounding_box() else {
            continue;
        };
        let width = f64::from(x_max - x_min);

        let mut cut: Option<(f64, f64)> = None;
        for (centre, fraction) in BANDS {
            let low = width
                .mul_add(centre - fraction / 2.0, f64::from(x_min))
                .round();
            let high = width
                .mul_add(centre + fraction / 2.0, f64::from(x_min))
                .round();
            if spread_x(glyph, low, high) <= tolerance {
                cut = Some((low, high));
                break;
            }
        }
        let Some((low, high)) = cut else {
            continue;
        };

        let bands = [
            (f64::from(x_min) - 1.0, low),
            (low, high),
            (high, f64::from(x_max) + 1.0),
        ];
        let mut pieces_of: Vec<GlyphData> = bands
            .iter()
            .enumerate()
            .map(|(index, (band_low, band_high))| {
                let contours = grounded_x(slice_x(glyph, *band_low, *band_high));
                // A horizontal part is placed by its width, so the advance is
                // the ink's width once the left edge is on the origin.
                let advance = x_range(&contours)
                    .map_or(0.0, |(low, high)| high - low)
                    .round();
                GlyphData {
                    name: horizontal_piece_name(&glyph.name, index),
                    unicodes: Vec::new(),
                    advance: u16::try_from(advance as i32).unwrap_or(0),
                    contours,
                }
            })
            .collect();
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

/// The pieces of a brace, bottom to top: the lower hook, the lower bar, the
/// point, the upper bar and the upper hook.
///
/// A brace needs five parts rather than three because its point cannot repeat.
/// Reference math fonts use the same shape: two extenders, one each side of the
/// point, and the renderer repeats both.
pub const BRACE_PIECES: [&str; 5] = ["b0", "b1", "b2", "b3", "b4"];

/// Where the two repeatable bars sit in [`BRACE_PIECES`].
pub const BRACE_EXTENDERS: [usize; 2] = [1, 3];

/// The name a brace piece carries.
#[must_use]
pub fn brace_piece_name(base: &str, index: usize) -> String {
    format!("{base}.{}", BRACE_PIECES[index])
}

/// The five pieces of every brace, encoded from `first_code` upward.
///
/// The point is cut out of the middle and the two bars are rectangles at the
/// stem, so a repeat cannot scallop. The hooks keep their drawn curves.
///
/// Returns no pieces when the brace is too short for the cuts to leave any
/// straight stem, which leaves the size variants as the only choice.
#[must_use]
pub fn derive_braces(glyphs: &[GlyphData], first_code: u32, overlap: u16) -> Vec<GlyphData> {
    let stroke = f64::from(overlap);
    let mut pieces: Vec<GlyphData> = Vec::new();
    for glyph in glyphs {
        if !glyph
            .unicodes
            .iter()
            .any(|code| *code == 0x007B || *code == 0x007D)
        {
            continue;
        }
        let Some((_, y_min, _, y_max)) = glyph.bounding_box() else {
            continue;
        };
        let height = f64::from(y_max - y_min);
        if height < 8.0 * stroke {
            continue;
        }
        let bottom = f64::from(y_min);
        let top = f64::from(y_max);
        let mid = f64::midpoint(bottom, top);
        // Every part has to carry two connectors, so it needs at least two
        // strokes of height. The cuts are placed to leave that much for the
        // point, both bars and both hooks.
        let point_half = (2.5 * stroke).min(height / 5.0);
        let hook = point_half;

        let point_low = mid - point_half;
        let point_high = mid + point_half;
        let lower_cut = bottom + hook;
        let upper_cut = top - hook;
        if lower_cut >= point_low
            || point_high >= upper_cut
            || lower_cut - bottom < 2.0 * stroke
            || point_low - lower_cut < 2.0 * stroke
            || upper_cut - point_high < 2.0 * stroke
            || top - upper_cut < 2.0 * stroke
        {
            continue;
        }

        // The stem's horizontal extent, read from a band halfway along each bar
        // where the brace is straight. Both halves share the x range.
        let probe = |low: f64, high: f64| {
            let band = slice_y(glyph, low, high);
            x_range(&band)
        };
        let upper_mid = f64::midpoint(point_high, upper_cut);
        let lower_mid = f64::midpoint(lower_cut, point_low);
        let (Some(upper_range), Some(lower_range)) = (
            probe(upper_mid - stroke / 2.0, upper_mid + stroke / 2.0),
            probe(lower_mid - stroke / 2.0, lower_mid + stroke / 2.0),
        ) else {
            continue;
        };
        let x_low = upper_range.0.min(lower_range.0);
        let x_high = upper_range.1.max(lower_range.1);

        let slices = [
            (bottom - 1.0, lower_cut),
            (point_low, point_high),
            (upper_cut, top + 1.0),
        ];
        let mut hooks: Vec<Vec<Vec<QuadifyPoint>>> = slices
            .iter()
            .map(|(low, high)| grounded(slice_y(glyph, *low, *high)))
            .collect();
        if hooks.iter().any(Vec::is_empty) {
            continue;
        }
        let lower_bar = bar(x_low, x_high, 0.0, point_low - lower_cut);
        let upper_bar = bar(x_low, x_high, 0.0, upper_cut - point_high);

        let mut pieces_of: Vec<GlyphData> = vec![
            GlyphData {
                name: brace_piece_name(&glyph.name, 0),
                unicodes: Vec::new(),
                advance: glyph.advance,
                contours: hooks.remove(0),
            },
            GlyphData {
                name: brace_piece_name(&glyph.name, 1),
                unicodes: Vec::new(),
                advance: glyph.advance,
                contours: vec![lower_bar],
            },
            GlyphData {
                name: brace_piece_name(&glyph.name, 2),
                unicodes: Vec::new(),
                advance: glyph.advance,
                contours: hooks.remove(0),
            },
            GlyphData {
                name: brace_piece_name(&glyph.name, 3),
                unicodes: Vec::new(),
                advance: glyph.advance,
                contours: vec![upper_bar],
            },
            GlyphData {
                name: brace_piece_name(&glyph.name, 4),
                unicodes: Vec::new(),
                advance: glyph.advance,
                contours: hooks.remove(0),
            },
        ];
        for piece in &mut pieces_of {
            let code = first_code + u32::try_from(pieces.len()).unwrap_or(0);
            piece.unicodes = vec![code];
            pieces.push(piece.clone());
        }
    }

    pieces
}
