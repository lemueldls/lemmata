//! Size variants for the glyphs a renderer stretches, derived rather than
//! drawn. See `docs/math.md`.

use crate::font::GlyphData;

/// The glyphs a renderer stretches vertically.
pub const STRETCHY: &[u32] = &[
    0x0028, 0x0029, 0x005B, 0x005D, 0x007B, 0x007D, 0x221A, 0x222B, 0x2211, 0x220F, 0x2308, 0x2309,
    0x230A, 0x230B, 0x27E8, 0x27E9,
];

/// The scales a stretchy glyph is derived at, beyond its own size.
pub const SCALES: &[f64] = &[1.5, 2.0];

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

/// The derived size variants. Scaling y alone keeps a vertical stroke at its
/// width, since x is untouched.
#[must_use]
pub fn derive(glyphs: &[GlyphData]) -> Vec<GlyphData> {
    let mut derived = Vec::new();
    for glyph in glyphs {
        if !glyph.unicodes.iter().any(|code| STRETCHY.contains(code)) {
            continue;
        }
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
        for (index, scale) in SCALES.iter().enumerate() {
            let contours = glyph
                .contours
                .iter()
                .map(|contour| {
                    contour
                        .iter()
                        .map(|point| lemmata_geom::QuadifyPoint {
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
                advance: glyph.advance,
                contours,
            });
        }
    }
    derived
}
