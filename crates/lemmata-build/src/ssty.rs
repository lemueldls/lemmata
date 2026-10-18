//! Script-size alternates, for `ssty`.
//!
//! r[impl math.script-style]
//!
//! A renderer sets script and scriptscript sizes by scaling a glyph down, which
//! closes its counters and thins its strokes. The specification gives it a way
//! to ask for a different drawing instead: `ssty` carries a list of alternates
//! per glyph and the renderer picks one by index, so the first is the script
//! size and the second the scriptscript size. It is an alternate substitution
//! rather than a single one, which is why `lemmata-layout` needs a lookup type
//! of its own for it.
//!
//! Two reference math fonts were read for the shape of that list. Latin Modern
//! Math covers 516 glyphs with two alternates each, STIX Two Math 529 with two
//! or four. Both widen the drawing and hold or barely raise the height: Latin
//! Modern's script alternate is 7 to 11 percent wider with the height unchanged,
//! STIX's 13 percent wider and 8 percent taller. The shared intent is wider
//! rather than taller, because a taller glyph collides with the line above, so
//! the derivation here holds the height and widens.
//!
//! What it does not do is thicken the strokes independently of the counters.
//! Latin Modern's alternate holds the counters at their size and thickens the
//! stems by about 14 percent, which is a redraw rather than a transform of the
//! outline. A horizontal scale widens both, which serves the same end by a
//! different route.

use crate::font::GlyphData;

/// The x-scale of each alternate: script, then scriptscript.
///
/// The first is Latin Modern Math's measured 7 to 11 percent ink widening,
/// rounded to 10. The second is its 19 to 25 percent, rounded to 25. The advance
/// takes the same factor, so the alternate is the base widened in x, cell and
/// drawing together.
pub const SCALES: &[f64] = &[1.10, 1.25];

/// The suffix an alternate carries, indexed by the scale's position.
#[must_use]
pub fn alternate_name(base: &str, index: usize) -> String {
    match index {
        0 => format!("{base}.st"),
        _ => format!("{base}.sts"),
    }
}

/// Whether the set is a math font's.
///
/// The math alphanumerics are the block only a math font carries, so holding one
/// is what says so. It matters because the feature is registered under the
/// `math` script, which a text run never selects: deriving alternates for a text
/// font's letters would put glyphs in it that nothing can reach.
fn is_math_font(glyphs: &[GlyphData]) -> bool {
    glyphs.iter().any(|glyph| {
        glyph
            .unicodes
            .iter()
            .any(|code| (0x1D400..=0x1D7FF).contains(code))
    })
}

/// Whether a renderer sets this glyph at script size.
///
/// It sets variables and subscripts there, so the letters and digits are the
/// whole of it: the math alphanumerics, and the plain Latin and Greek a renderer
/// reaches for when no style is asked. The delimiters are in as well, since it
/// scales those at script size as well as stretches them. The large operators
/// are not, because it stretches rather than scales them, and neither is the
/// radical, for the same reason.
fn is_scriptable(glyph: &GlyphData, math_font: bool) -> bool {
    let in_range = |low: u32, high: u32| {
        glyph
            .unicodes
            .iter()
            .any(|code| (low..=high).contains(code))
    };
    if in_range(0x1D400, 0x1D7FF) {
        return true;
    }
    if !math_font {
        return false;
    }

    in_range(0x0030, 0x0039)
        || in_range(0x0041, 0x005A)
        || in_range(0x0061, 0x007A)
        || in_range(0x0391, 0x03C9)
        || glyph.unicodes.iter().any(|code| {
            crate::variants::STRETCHY.contains(code)
                && !crate::variants::OPERATORS.contains(code)
                && *code != 0x221A
        })
}

/// The script-size alternates of the glyphs a renderer sets at script size,
/// encoded from `first_code` upward.
// The advance is rounded into the 16-bit field the table stores, which cannot
// overflow a glyph at any scale worth deriving.
// The arithmetic is kept plain rather than fused: the alternate's proportions
// are a design choice and a last-bit difference between the native and wasm
// builds must not be able to move a glyph across a threshold.
#[allow(clippy::suboptimal_flops)]
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn derive(glyphs: &[GlyphData], first_code: u32) -> Vec<GlyphData> {
    let math_font = is_math_font(glyphs);
    let mut alternates = Vec::new();
    for glyph in glyphs {
        if !is_scriptable(glyph, math_font) {
            continue;
        }
        let centre = f64::from(glyph.advance) / 2.0;
        for (index, scale) in SCALES.iter().enumerate() {
            let contours = glyph
                .contours
                .iter()
                .map(|contour| {
                    contour
                        .iter()
                        .map(|point| lemmata_geom::QuadifyPoint {
                            x: centre + (point.x - centre) * scale,
                            ..*point
                        })
                        .collect()
                })
                .collect();
            let code = first_code + u32::try_from(alternates.len()).unwrap_or(0);
            let grown = (f64::from(glyph.advance) * scale).round();
            alternates.push(GlyphData {
                name: alternate_name(&glyph.name, index),
                unicodes: vec![code],
                advance: u16::try_from(grown as i32).unwrap_or(glyph.advance),
                contours,
            });
        }
    }

    alternates
}
