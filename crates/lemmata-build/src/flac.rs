//! Flattened accents, for `flac`.
//!
//! r[impl math.script-style]
//!
//! The specification gives `flac` flattened forms of the accents, for a renderer
//! to use when the base is tall enough that the ordinary accent would reach past
//! the line. It is a single substitution, so the crate can already carry it, and
//! the accents are the combining marks the math capture holds.
//!
//! The reference is STIX Two Math, which maps ten combining accents to a
//! `mathcap` form. Read off the outlines, the flattened form is the accent made
//! shorter and wider: the grave and the acute lose the most, 0.84 of their
//! height and 1.27 of their width, and the macron the least, 0.96 and 1.01. So
//! the derivation takes one factor in the middle of that range rather than a
//! table per accent. The height is scaled about the accent's own bottom, which
//! is the edge it attaches by, so a flattened accent sits where the ordinary one
//! did.
//!
//! Only the accents above the base are flattened. An accent below hangs from the
//! other edge, where a tall base is not the problem.

use crate::font::GlyphData;

/// The suffix a flattened accent carries.
#[must_use]
pub fn flattened_name(base: &str) -> String {
    format!("{base}.mathcap")
}

/// How much shorter a flattened accent is.
pub const FLATTEN: f64 = 0.9;

/// How much wider it is.
///
/// The reference widens as it flattens, which keeps the stroke weight: a scale
/// in y alone thins every horizontal stroke by the same factor.
pub const WIDEN: f64 = 1.1;

/// Whether a glyph is an accent that sits above the base.
///
/// The glyph source says so in the name, `graveAbove` against `graveBelow`, and
/// a combining mark carries no advance of its own.
fn is_above(glyph: &GlyphData) -> bool {
    glyph.advance == 0 && glyph.name.contains("Above") && !glyph.contours.is_empty()
}

/// The flattened forms of the accents above the base, encoded from `first_code`
/// upward.
// The arithmetic is kept plain rather than fused: the flattened proportions are
// a design choice and a last-bit difference between the native and wasm builds
// must not be able to move a glyph across a threshold.
#[allow(clippy::suboptimal_flops)]
#[must_use]
pub fn derive(glyphs: &[GlyphData], first_code: u32) -> Vec<GlyphData> {
    let mut flattened = Vec::new();
    for glyph in glyphs {
        if !is_above(glyph) {
            continue;
        }
        let Some((x_min, y_min, x_max, _)) = glyph.bounding_box() else {
            continue;
        };
        let centre = f64::midpoint(f64::from(x_min), f64::from(x_max));
        let bottom = f64::from(y_min);
        let contours = glyph
            .contours
            .iter()
            .map(|contour| {
                contour
                    .iter()
                    .map(|point| {
                        lemmata_geom::QuadifyPoint {
                            x: centre + (point.x - centre) * WIDEN,
                            y: bottom + (point.y - bottom) * FLATTEN,
                            ..*point
                        }
                    })
                    .collect()
            })
            .collect();
        let code = first_code + u32::try_from(flattened.len()).unwrap_or(0);
        flattened.push(GlyphData {
            name: flattened_name(&glyph.name),
            unicodes: vec![code],
            advance: glyph.advance,
            contours,
        });
    }

    flattened
}
