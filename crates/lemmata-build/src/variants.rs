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

/// How much wider a set-bigger operator grows, against how much taller.
///
/// A scale multiplies the stroke, and the weight of a vertical stroke is its
/// width. Growing width by the height's factor is what made the summation's ink
/// as heavy as the text was tall. A modest widening keeps the operator legible
/// as one shape without thickening its strokes by the height's factor.
pub const WIDEN: f64 = 1.25;

/// The scales a delimiter is derived at, beyond its own size.
pub const SCALES: &[f64] = &[1.5, 2.0];

/// The scales a large operator is derived at, stretched or set bigger.
///
/// One further than a delimiter gets, because a display operator is set larger
/// than a stretched delimiter and the renderer can only pick from what it is
/// given. `displayOperatorMinHeight` decides which of these it picks.
pub const OPERATOR_SCALES: &[f64] = &[1.5, 2.0, 2.25];

/// The glyphs a renderer stretches horizontally.
///
/// A horizontal arrow grows by lengthening its shaft, so its head keeps the
/// size it was drawn at. The set is the arrows an equation reaches for:
/// implication and equivalence, maps-to, the harpoons and the dashed forms.
pub const HORIZONTAL: &[u32] = &[
    0x2190, 0x2192, 0x2194, 0x21A0, 0x21A2, 0x21A3, 0x21A6, 0x21A9, 0x21AA, 0x21AE, 0x21BC, 0x21BD,
    0x21C0, 0x21C1, 0x21C4, 0x21C6, 0x21D0, 0x21D2, 0x21D4, 0x21DA, 0x21DB, 0x21E0, 0x21E2,
];

/// The scales a horizontal glyph is derived at, beyond its own size.
pub const HORIZONTAL_SCALES: &[f64] = &[2.0, 3.0];

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

/// The suffix a derived horizontal variant carries.
///
/// The `.h` keeps it apart from the vertical `.s` set: a glyph carries only one
/// of the two, but the private-use codes they take must stay unique and the
/// names are what a reader follows.
#[must_use]
pub fn horizontal_variant_name(base: &str, index: usize) -> String {
    format!("{base}.h{index}")
}

/// The derived size variants.
///
/// A delimiter is scaled in y alone, which keeps a vertical stroke at its width.
/// A large operator is scaled in both axes and its advance grows with it, since
/// a wider drawing in a cell that did not grow would run into its neighbour.
/// A horizontal arrow is the mirror: it scales in x, and its advance grows with
/// the drawing so the shaft reaches into the longer cell.
// The advance of a grown operator is rounded into the 16-bit field the table
// stores, which cannot overflow a glyph at any scale worth deriving. The
// arithmetic is kept plain rather than fused, because these transforms decide
// the bytes and a last-bit difference between the native and wasm builds would
// move them.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::suboptimal_flops
)]
#[must_use]
pub fn derive(glyphs: &[GlyphData]) -> Vec<GlyphData> {
    let mut derived = Vec::new();
    for glyph in glyphs {
        if !glyph.unicodes.iter().any(|code| STRETCHY.contains(code)) {
            continue;
        }
        let operator = glyph.unicodes.iter().any(|code| OPERATORS.contains(code));
        let uniform = glyph.unicodes.iter().any(|code| SET_BIGGER.contains(code));
        // Width and height grow by different factors: height by the scale, width
        // by `WIDEN` at most, so the stroke weight stays near the drawn one.
        let widen = if uniform { WIDEN } else { 1.0 };
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
                            x: centre_x
                                + (point.x - centre_x) * widen
                                + f64::from(glyph.advance) * (widen - 1.0) / 2.0,
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
                advance: {
                    let grown = (f64::from(glyph.advance) * widen).round();
                    u16::try_from(grown as i32).unwrap_or(glyph.advance)
                },
                contours,
            });
        }
    }

    // The horizontal set is the mirror of the vertical one. An arrow keeps its
    // height and grows in x, so scaling about the left edge leaves the tail
    // where it was and carries the head into the wider cell.
    for glyph in glyphs {
        if !glyph.unicodes.iter().any(|code| HORIZONTAL.contains(code)) {
            continue;
        }
        for (index, scale) in HORIZONTAL_SCALES.iter().enumerate() {
            let contours = glyph
                .contours
                .iter()
                .map(|contour| {
                    contour
                        .iter()
                        .map(|point| lemmata_geom::QuadifyPoint {
                            x: point.x * scale,
                            ..*point
                        })
                        .collect()
                })
                .collect();
            let code = PRIVATE_USE_START + u32::try_from(derived.len()).unwrap_or(0);
            let grown = (f64::from(glyph.advance) * scale).round();
            derived.push(GlyphData {
                name: horizontal_variant_name(&glyph.name, index),
                unicodes: vec![code],
                advance: u16::try_from(grown as i32).unwrap_or(glyph.advance),
                contours,
            });
        }
    }
    derived
}
