//! Size variants for the glyphs a renderer stretches, derived rather than
//! drawn. See `docs/math.md`.

use crate::font::GlyphData;

/// The glyphs a renderer stretches vertically.
pub const STRETCHY: &[u32] = &[
    0x0028, 0x0029, 0x005B, 0x005D, 0x007B, 0x007D, 0x221A, 0x222B, 0x2211, 0x220F, 0x2308, 0x2309,
    0x230A, 0x230B, 0x27E8, 0x27E9,
];

/// The glyphs a renderer sets bigger rather than stretches.
///
/// A delimiter stretches: a parenthesis is a curve that grows taller while the
/// stroke keeps its width, so scaling y alone is what a taller one looks like.
/// The summation and the product are not stretched but set at a larger size, so
/// their display forms are scaled drawings, wider as well as taller. Deriving a
/// taller sigma without widening it is what made it look thin at display size.
///
/// The integral is not among them. Its limits sit in the gaps of the shape, so a
/// wider drawing pushes them out and it stops reading as one operator. It is
/// stretched like a delimiter and takes the larger size the others get.
pub const SET_BIGGER: &[u32] = &[0x220F, 0x2211];

/// The glyphs that carry a display size, stretched or set bigger.
pub const OPERATORS: &[u32] = &[0x220F, 0x2211, 0x222B];

/// The scales a delimiter is derived at, beyond its own size.
pub const SCALES: &[f64] = &[1.5, 2.0];

/// The scales a large operator is derived at, stretched or set bigger.
///
/// One further than a delimiter gets, because a display operator is set larger
/// than a stretched delimiter and the renderer can only pick from what it is
/// given. `displayOperatorMinHeight` decides which of these it picks.
pub const OPERATOR_SCALES: &[f64] = &[1.5, 2.0, 2.25];

/// The scales a glyph is derived at.
#[must_use]
pub const fn scales(operator: bool) -> &'static [f64] {
    if operator { OPERATOR_SCALES } else { SCALES }
}

/// Where the derived variants are encoded.
///
/// They are not characters, so nothing types them, but a subsetter keeps what
/// is reachable and drops what is not: a variant named only by the MATH table
/// disappears and leaves the table pointing at nothing. Iosevka encodes its own
/// derived texture variants in this block for the same reason.
pub const PRIVATE_USE_START: u32 = 0xF000;

/// The suffix a derived variant carries.
#[must_use]
pub fn variant_name(base: &str, index: usize) -> String {
    format!("{base}.s{index}")
}

/// The derived size variants.
///
/// A delimiter is scaled in y alone, which keeps a vertical stroke at its width.
/// A large operator is scaled in both axes and its advance grows with it, since
/// a wider drawing in a cell that did not grow would run into its neighbour.
// The advance of a grown operator is rounded into the 16-bit field the table
// stores, which cannot overflow a glyph at any scale worth deriving.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[must_use]
pub fn derive(glyphs: &[GlyphData]) -> Vec<GlyphData> {
    let mut derived = Vec::new();
    for glyph in glyphs {
        if !glyph.unicodes.iter().any(|code| STRETCHY.contains(code)) {
            continue;
        }
        let operator = glyph.unicodes.iter().any(|code| OPERATORS.contains(code));
        let uniform = glyph.unicodes.iter().any(|code| SET_BIGGER.contains(code));
        let centre_x = f64::from(glyph.advance) / 2.0;
        // A grown operator's cell is wider, so the drawing has to move with it:
        // scaled about the old cell's centre alone, its ink would start left of
        // the new origin and the operator would sit off-centre in the line.
        let scales = scales(operator);
        let (low, high) = glyph
            .contours
            .iter()
            .flatten()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), point| {
                (low.min(point.y), high.max(point.y))
            });
        if !low.is_finite() {
            continue;
        }
        let centre = f64::midpoint(low, high);
        for (index, scale) in scales.iter().enumerate() {
            let contours = glyph
                .contours
                .iter()
                .map(|contour| {
                    contour
                        .iter()
                        .map(|point| lemmata_geom::QuadifyPoint {
                            x: if uniform {
                                centre_x
                                    + (point.x - centre_x) * scale
                                    + f64::from(glyph.advance) * (scale - 1.0) / 2.0
                            } else {
                                point.x
                            },
                            y: centre + (point.y - centre) * scale,
                            ..*point
                        })
                        .collect()
                })
                .collect();
            let code = PRIVATE_USE_START + u32::try_from(derived.len()).unwrap_or(0);
            derived.push(GlyphData {
                name: variant_name(&glyph.name, index),
                unicodes: vec![code],
                advance: if uniform {
                    let grown = (f64::from(glyph.advance) * scale).round();
                    u16::try_from(grown as i32).unwrap_or(glyph.advance)
                } else {
                    glyph.advance
                },
                contours,
            });
        }
    }
    derived
}
