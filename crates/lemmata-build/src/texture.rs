//! Texture healing, for `calt`.
//!
//! r[impl build.texture]
//!
//! A monospace grid forces every letter into the same advance, so a narrow `i`
//! carries more whitespace than it needs while a wide `m` is squeezed. Texture
//! healing substitutes a form that gives that whitespace to the neighbour that
//! wants it, and the whole trick is that the advance does not change: only the
//! drawing moves inside the cell, so the grid is untouched and the text still
//! lines up.
//!
//! Two roles, both measured rather than listed. A glyph gives space when its
//! ink leaves much of the cell empty, and takes space when its ink nearly fills
//! it. The forms are the same drawing shifted, and the pair is what makes room:
//! the giver moves toward the taker and the taker moves toward the giver, which
//! closes the gap between them without moving anything else.

use crate::font::GlyphData;

/// How far a form moves, as a fraction of the advance.
///
/// Monaspace tunes this per pair against drawn alternates. A derivation has one
/// number, and a sixteenth of the cell is enough to be seen without the ink
/// reaching the edge.
const SHIFT: f64 = 1.0 / 16.0;

/// The letters are split into fours by how much of the cell they leave empty.
/// The first quarter gives space and the last quarter takes it.
const PARTS: usize = 4;

/// Which side of its cell a form moves toward.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Both,
}

impl Side {
    /// The suffix a form carries.
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Both => "both",
        }
    }
}

/// What a glyph does in a pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Gives,
    Takes,
}

impl Role {
    /// The forms a glyph of this role carries.
    #[must_use]
    pub const fn forms(self) -> &'static [Side] {
        match self {
            // A taker can reach out both ways at once, which is what a glyph
            // between two givers needs. A giver cedes one side at a time,
            // because the other side has nothing to give.
            Self::Takes => &[Side::Left, Side::Right, Side::Both],
            Self::Gives => &[Side::Left, Side::Right],
        }
    }
}

/// The name a form carries.
#[must_use]
pub fn form_name(base: &str, side: Side) -> String {
    format!("{base}.{}", side.suffix())
}

/// The share of the cell the glyph's ink leaves empty.
fn empty(glyph: &GlyphData) -> Option<f64> {
    let (x_min, _, x_max, _) = glyph.bounding_box()?;

    Some(1.0 - f64::from(x_max - x_min) / f64::from(glyph.advance))
}

/// A letter of one codepoint, which is what the ranking is over.
///
/// Punctuation is left out. It leaves the most room of anything in the font,
/// but a period that moved would stop being a period, which is why the design
/// this follows keeps those still and lets the letter beside them move.
fn is_letter(glyph: &GlyphData) -> bool {
    glyph.unicodes.len() == 1
        && glyph
            .unicodes
            .first()
            .is_some_and(|code| char::from_u32(*code).is_some_and(|c| c.is_ascii_alphabetic()))
}

/// What each glyph does in a pair, by rank among the letters, as an index into
/// `glyphs` and the role.
///
/// A rank rather than a threshold, because what counts as narrow is a property
/// of the font. This one fills its cells more evenly than the design texture
/// healing was invented for, so an absolute cut either misses the narrow
/// letters or takes in half the alphabet. The narrowest quarter of the letters
/// gives space and the widest quarter takes it. Monaspace chooses its set by
/// hand; a rank is what a derivation has.
#[must_use]
pub fn classify(glyphs: &[GlyphData]) -> Vec<(usize, Role)> {
    let mut ranked: Vec<(f64, usize)> = glyphs
        .iter()
        .enumerate()
        .filter(|(_, glyph)| is_letter(glyph))
        .filter_map(|(index, glyph)| Some((empty(glyph)?, index)))
        .collect();
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
    let cut = ranked.len().div_ceil(PARTS);
    if cut == 0 {
        return Vec::new();
    }

    let mut roles = Vec::new();
    for (rank, (_, index)) in ranked.iter().enumerate() {
        let role = if rank < cut {
            Role::Gives
        } else if rank + cut >= ranked.len() {
            Role::Takes
        } else {
            continue;
        };
        roles.push((*index, role));
    }

    roles
}

/// The forms a glyph carries, in the order `Side::forms` names them.
#[must_use]
pub fn forms(
    glyph: &GlyphData,
    role: Role,
    advance: u16,
) -> Vec<(Side, Vec<Vec<lemmata_geom::QuadifyPoint>>)> {
    let shift = f64::from(advance) * SHIFT;
    role.forms()
        .iter()
        .map(|side| {
            let contours = match side {
                // A form that moves is the drawing translated, which is what
                // keeps the letterform and the cell both unchanged.
                Side::Left => moved(glyph, -shift),
                Side::Right => moved(glyph, shift),
                // A form that takes both sides has to grow, so it is scaled
                // about its own centre until it reaches `shift` further out on
                // each side.
                Side::Both => widened(glyph, shift),
            };
            (*side, contours)
        })
        .collect()
}

/// The glyph's contours translated along x.
fn moved(glyph: &GlyphData, dx: f64) -> Vec<Vec<lemmata_geom::QuadifyPoint>> {
    glyph
        .contours
        .iter()
        .map(|contour| {
            contour
                .iter()
                .map(|point| lemmata_geom::QuadifyPoint {
                    x: point.x + dx,
                    ..*point
                })
                .collect()
        })
        .collect()
}

/// The glyph's contours widened about their centre until each side reaches
/// `shift` further out.
#[allow(clippy::suboptimal_flops)]
fn widened(glyph: &GlyphData, shift: f64) -> Vec<Vec<lemmata_geom::QuadifyPoint>> {
    let Some((x_min, _, x_max, _)) = glyph.bounding_box() else {
        return glyph.contours.clone();
    };
    let width = f64::from(x_max - x_min);
    if width <= 0.0 {
        return glyph.contours.clone();
    }
    let centre = f64::midpoint(f64::from(x_min), f64::from(x_max));
    let scale = (width + 2.0 * shift) / width;

    glyph
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
        .collect()
}

/// The forms for every glyph that has a role, encoded from `first_code` upward.
#[must_use]
pub fn derive(glyphs: &[GlyphData], first_code: u32) -> Vec<GlyphData> {
    let mut derived = Vec::new();
    for (index, role) in classify(glyphs) {
        let glyph = &glyphs[index];
        for (side, contours) in forms(glyph, role, glyph.advance) {
            let code = first_code + u32::try_from(derived.len()).unwrap_or(0);
            derived.push(GlyphData {
                name: form_name(&glyph.name, side),
                unicodes: vec![code],
                advance: glyph.advance,
                contours,
            });
        }
    }

    derived
}
