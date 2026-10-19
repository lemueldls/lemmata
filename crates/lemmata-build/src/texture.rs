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
//! Every form here is a transform of one drawing, and a transform cannot draw.
//! A stem widened is a stem thickened, and a dot caught in the scale is an
//! ellipse, which is why one is left out. Drawn forms are the glyph source's to
//! give: the parameters that place ink in a cell are already the build's, and
//! what is missing is the half that turns them back into outlines. Until that
//! exists this is the compromise, and it is a good one only where a letter is
//! close to filling its cell to begin with.
//!
//! The resource is the empty space, not the ink, so the narrow letters are the
//! givers and the wide ones are the takers. That is the reverse of what the two
//! names suggest while the ink is what you have in mind.
//!
//! Two roles, both measured rather than listed. A glyph gives space when its
//! ink leaves much of the cell empty, and takes space when its ink nearly fills
//! it. The forms are the same drawing shifted, and the pair is what makes room:
//! the giver moves toward the taker and the taker moves toward the giver, which
//! closes the gap between them without moving anything else.
//!
//! A glyph with room on both sides is the case that needs a second look. A
//! giver between two takers moved toward one of them closes one gap by what it
//! opens on the other, which leaves the pair no more even than it found it, so
//! it takes both sides and widens instead. The wider form is the only one that
//! grows: every other form keeps the drawing's size and changes only where in
//! the cell it sits.

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
    /// A letter with room to spare, which cedes it to the neighbour.
    Gives,
    /// A glyph whose ink nearly fills the cell, which takes what a neighbour
    /// cedes.
    Takes,
    /// A mark with room to spare that stays where it is. A period that moved
    /// would stop reading as a period, so it lets the letter beside it move
    /// instead.
    Stationary,
}

impl Role {
    /// The forms a glyph of this role carries.
    #[must_use]
    pub const fn forms(self) -> &'static [Side] {
        match self {
            // A taker can reach out both ways at once, which is what a glyph
            // between two givers needs.
            Self::Takes => &[Side::Left, Side::Right, Side::Both],
            // A giver with a taker on one side cedes that side. With a taker on
            // both it has room on both, and ceding one would close one gap by
            // exactly what it opened on the other, which is worse than leaving
            // it alone. So it takes both sides at once and both gaps close by
            // the same amount.
            Self::Gives => &[Side::Left, Side::Right, Side::Both],
            // A stationary glyph has no forms at all: it is a context, not a
            // participant.
            Self::Stationary => &[],
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

/// A letter of one codepoint.
fn is_letter(glyph: &GlyphData) -> bool {
    glyph.unicodes.len() == 1
        && glyph
            .unicodes
            .first()
            .is_some_and(|code| char::from_u32(*code).is_some_and(|c| c.is_ascii_alphabetic()))
}

/// A mark of one codepoint: punctuation, and not a digit.
///
/// A figure that shifted would stop lining up with the figures around it, so
/// digits are left out of this the way the design it follows leaves them out.
fn is_mark(glyph: &GlyphData) -> bool {
    glyph.unicodes.len() == 1
        && glyph
            .unicodes
            .first()
            .is_some_and(|code| char::from_u32(*code).is_some_and(|c| !c.is_alphanumeric()))
}

/// What each glyph does in a pair, by rank against the rest, as an index into
/// `glyphs` and the role.
///
/// A rank rather than a threshold, because what counts as narrow is a property
/// of the font. This one fills its cells more evenly than the design texture
/// healing was invented for, so an absolute cut either misses the narrow
/// letters or takes in half the alphabet.
///
/// The emptiest quarter give space, and the fullest quarter take it. A mark in
/// the giving quarter stands still instead: it has the room, but it is the
/// letter beside it that should move. Monaspace chooses its set by hand; a rank
/// is what a derivation has.
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

    let mut narrowest = f64::INFINITY;
    let mut roles = Vec::new();
    for (rank, (value, index)) in ranked.iter().enumerate() {
        let role = if rank < cut {
            if is_letter(&glyphs[*index]) {
                narrowest = narrowest.min(*value);
                Role::Gives
            } else {
                Role::Stationary
            }
        } else if rank + cut >= ranked.len() {
            Role::Takes
        } else {
            continue;
        };
        roles.push((*index, role));
    }

    // A mark that leaves more room than the narrowest giving letter has to
    // spare, but must not spend it itself. Ranking the marks alongside the
    // letters instead would widen the quarters and take in letters that are
    // neither narrow nor wide, which is worse than leaving them out.
    for (index, glyph) in glyphs.iter().enumerate() {
        if is_letter(glyph) || !is_mark(glyph) {
            continue;
        }
        if empty(glyph).is_some_and(|value| value > narrowest) {
            roles.push((index, Role::Stationary));
        }
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
///
/// A small contour riding above the rest is a dot, and a dot scaled about the
/// drawing's centre comes out an ellipse: the `i` and the `j` would grow ovals.
/// It is left exactly where it is instead. The stem under it widens about the
/// same centre, so a dot that sat over the stem still does, and it is still
/// round.
#[allow(clippy::suboptimal_flops)]
fn widened(glyph: &GlyphData, shift: f64) -> Vec<Vec<lemmata_geom::QuadifyPoint>> {
    let Some((x_min, y_min, x_max, y_max)) = glyph.bounding_box() else {
        return glyph.contours.clone();
    };
    let width = f64::from(x_max - x_min);
    if width <= 0.0 {
        return glyph.contours.clone();
    }
    let centre = f64::midpoint(f64::from(x_min), f64::from(x_max));
    let scale = (width + 2.0 * shift) / width;
    let middle = f64::midpoint(f64::from(y_min), f64::from(y_max));
    let largest = glyph
        .contours
        .iter()
        .map(|contour| extent(contour))
        .fold(0.0_f64, f64::max);

    glyph
        .contours
        .iter()
        .map(|contour| {
            // A dot is small beside the drawing and sits above its middle. Both
            // halves matter: a small contour low down is a serif or a tail, and
            // those are part of what widens.
            let dot =
                extent(contour) * 4.0 < largest && contour.iter().all(|point| point.y > middle);
            contour
                .iter()
                .map(|point| lemmata_geom::QuadifyPoint {
                    x: if dot {
                        point.x
                    } else {
                        centre + (point.x - centre) * scale
                    },
                    ..*point
                })
                .collect()
        })
        .collect()
}

/// How much of the drawing a contour takes up, as its bounding box.
fn extent(contour: &[lemmata_geom::QuadifyPoint]) -> f64 {
    let x_min = contour.iter().fold(f64::INFINITY, |low, p| low.min(p.x));
    let x_max = contour
        .iter()
        .fold(f64::NEG_INFINITY, |high, p| high.max(p.x));
    let y_min = contour.iter().fold(f64::INFINITY, |low, p| low.min(p.y));
    let y_max = contour
        .iter()
        .fold(f64::NEG_INFINITY, |high, p| high.max(p.y));

    ((x_max - x_min) * (y_max - y_min)).max(0.0)
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
