//! Dotless forms, for `dtls`.
//!
//! r[impl math.script-style]
//!
//! The specification asks for dotless forms of the letters that carry a dot, so
//! that an accent can sit on the stem instead of beside the dot. `i` and `j`
//! draw the dot as a contour of its own, which is what makes the form derivable
//! rather than drawn: the stem stays and the dot goes.

use crate::font::GlyphData;

/// The suffix a dotless form carries.
#[must_use]
pub fn dotless_name(base: &str) -> String {
    format!("{base}.dotless")
}

/// The contour that is the dot, if the glyph has one.
///
/// A dot is the ink entirely above everything else in the glyph, small, and
/// clear of the x-height. Each of those carries weight. The stem of an `i`
/// crosses the x-height, which separates the two; sitting above every other
/// contour is what separates a dot from the flourish a fraktur letter carries
/// and from the second dot of a diaeresis, which stands beside the first rather
/// than above the letter; and being small separates it from the ascender of an
/// `l`. A glyph that is only a dot has no stem to keep.
fn dot(glyph: &GlyphData, x_height: f64) -> Option<usize> {
    let (_, y_min, _, y_max) = glyph.bounding_box()?;
    let height = f64::from(y_max - y_min);
    let (mut found, mut highest) = (None, f64::NEG_INFINITY);
    for (index, contour) in glyph.contours.iter().enumerate() {
        let top = contour
            .iter()
            .fold(f64::NEG_INFINITY, |top, point| top.max(point.y));
        if top > highest {
            (found, highest) = (Some(index), top);
        }
    }
    let index = found?;
    let contour = &glyph.contours[index];
    let low = contour
        .iter()
        .fold(f64::INFINITY, |low, point| low.min(point.y));
    // Everything else has to sit below the mark, or the mark is not above the
    // letter: a diaeresis stands beside its pair rather than above the stem.
    let below = glyph
        .contours
        .iter()
        .enumerate()
        .filter(|(other, _)| *other != index)
        .flat_map(|(_, contour)| contour.iter())
        .fold(f64::NEG_INFINITY, |top, point| top.max(point.y));
    let is_dot = low > x_height && low > below && highest - low < height * 0.25;

    (glyph.contours.len() > 1 && is_dot && is_blob(contour)).then_some(index)
}

/// Whether a contour is a blob rather than a stroke.
///
/// This is what separates a tittle from an accent, and it has to, because an
/// accent is also a small mark sitting above the letter. A tittle is filled and
/// as tall as it is wide; a grave or a tilde is long and thin, and a
/// circumflex is wide and hollow against its box.
// The arithmetic is kept plain rather than fused: this decides whether a glyph
// gets a form, and a last-bit difference between the native and wasm builds
// must not be able to move it across a threshold.
#[allow(clippy::suboptimal_flops)]
fn is_blob(contour: &[lemmata_geom::QuadifyPoint]) -> bool {
    let (mut x0, mut x1) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY);
    for point in contour {
        x0 = x0.min(point.x);
        x1 = x1.max(point.x);
        y0 = y0.min(point.y);
        y1 = y1.max(point.y);
    }
    let (width, tall) = (x1 - x0, y1 - y0);
    if width <= 0.0 || tall <= 0.0 {
        return false;
    }
    let aspect = tall / width;
    // The shoelace over the points, which for a contour this small is close
    // enough to its area to tell filled from hollow.
    let mut twice = 0.0;
    for (index, point) in contour.iter().enumerate() {
        let next = &contour[(index + 1) % contour.len()];
        twice += point.x * next.y - next.x * point.y;
    }
    let filled = (twice / 2.0).abs() / (width * tall);

    (0.6..=1.6).contains(&aspect) && filled > 0.7
}

/// The dotless forms of the letters that carry a dot, encoded from `first_code`
/// upward.
#[must_use]
pub fn derive(glyphs: &[GlyphData], x_height: f64, first_code: u32) -> Vec<GlyphData> {
    let mut forms = Vec::new();
    for glyph in glyphs {
        if !glyph
            .unicodes
            .iter()
            .any(|code| char::from_u32(*code).is_some_and(char::is_alphabetic))
        {
            continue;
        }
        let Some(index) = dot(glyph, x_height) else {
            continue;
        };
        let mut contours = glyph.contours.clone();
        contours.remove(index);
        let code = first_code + u32::try_from(forms.len()).unwrap_or(0);
        forms.push(GlyphData {
            name: dotless_name(&glyph.name),
            unicodes: vec![code],
            advance: glyph.advance,
            contours,
        });
    }

    forms
}
