//! Texture healing, for `TXTR`.
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
//! The resource is the empty space, so a narrow letter is a *giver* and a wide
//! one a *taker*. The form names are the drawing's edge: a giver shrinks on the
//! side it faces the taker, and the taker extends into what the giver gives up.
//! A giver between two takers shrinks both sides, and a taker between two givers
//! extends both.
//!
//! Six forms per letter, which is upstream's set: `ExtL`, `ExtR`, `ExtLR`,
//! `ShrL`, `ShrR`, `ShrLR`. Every letter carries all six, and the rules choose
//! by role and by what the neighbours have become. Upstream draws them by
//! re-running the definition with `advanceScale` at `1 ± 1/12`; until that is
//! wired into both front ends this derives the form by scaling the captured
//! outline, which is exact for a straight stem and approximate for a shape
//! whose ink depends on the frame.
//!
//! A dot is the exception a scale gets wrong. It is small and it sits above the
//! drawing's middle, so it is moved with the form and never scaled: the `i` and
//! the `j` keep a round dot.

use std::collections::BTreeMap;

use crate::font::GlyphData;

/// How far a form reaches, as a fraction of the advance.
///
/// Upstream uses `1/12` for both the extension and the shrink.
pub const SHIFT: f64 = 1.0 / 12.0;

/// The letters are split into fours by how much of the cell they leave empty.
/// The first quarter gives space and the last quarter takes it.
const PARTS: usize = 4;

/// One of the six forms a letter carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Form {
    /// Extend the left edge.
    ExtL,
    /// Extend the right edge.
    ExtR,
    /// Extend both edges.
    ExtLR,
    /// Pull the left edge in.
    ShrL,
    /// Pull the right edge in.
    ShrR,
    /// Pull both edges in.
    ShrLR,
}

impl Form {
    /// The suffix a form carries.
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::ExtL => "extL",
            Self::ExtR => "extR",
            Self::ExtLR => "extLR",
            Self::ShrL => "shrL",
            Self::ShrR => "shrR",
            Self::ShrLR => "shrLR",
        }
    }

    /// How much the drawing scales along x.
    const fn scale(self) -> f64 {
        match self {
            Self::ExtL | Self::ExtR | Self::ExtLR => 1.0 + SHIFT,
            Self::ShrL | Self::ShrR | Self::ShrLR => 1.0 - SHIFT,
        }
    }

    /// How far the drawing moves, as a fraction of the advance.
    ///
    /// The edge the form names takes the extra room, so the drawing moves away
    /// from it: a left form moves left by half the added width.
    const fn shift(self) -> f64 {
        match self {
            Self::ExtL | Self::ShrR => -0.5 * SHIFT,
            Self::ExtR | Self::ShrL => 0.5 * SHIFT,
            Self::ExtLR | Self::ShrLR => 0.0,
        }
    }
}

/// Every form, in the order the rules name them.
pub const FORMS: &[Form] = &[
    Form::ExtL,
    Form::ExtR,
    Form::ExtLR,
    Form::ShrL,
    Form::ShrR,
    Form::ShrLR,
];

/// What a glyph does in a pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// A letter with room to spare, which gives it to the neighbour.
    Gives,
    /// A letter whose ink nearly fills the cell, which takes what a neighbour
    /// gives.
    Takes,
    /// A mark with room to spare that stays where it is. A period that moved
    /// would stop reading as a period, so it lets the letter beside it move
    /// instead.
    Stationary,
}

/// The name a form carries.
#[must_use]
pub fn form_name(base: &str, form: Form) -> String {
    format!("{base}.{}", form.suffix())
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
/// of the font. The emptiest quarter give space and the fullest quarter take it.
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
            narrowest = narrowest.min(*value);
            Role::Gives
        } else if rank + cut >= ranked.len() {
            Role::Takes
        } else {
            continue;
        };
        roles.push((*index, role));
    }

    // A mark that leaves more room than the narrowest giving letter has to
    // spare, but must not spend it itself.
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

/// The six forms a glyph carries, translated by its advance.
#[must_use]
pub fn forms(glyph: &GlyphData, advance: u16) -> Vec<(Form, Vec<Vec<lemmata_geom::QuadifyPoint>>)> {
    let shift = f64::from(advance) * SHIFT;

    FORMS
        .iter()
        .map(|form| {
            let contours = formed(glyph, form.scale(), form.shift() * shift);
            (*form, contours)
        })
        .collect()
}

/// The glyph's contours scaled along x about their centre, then moved by `dx`.
///
/// A small contour riding above the rest is a dot, and a dot scaled about the
/// drawing's centre comes out an ellipse: the `i` and the `j` would grow ovals.
/// It is moved with the drawing and never scaled, so it stays round and over
/// the stem it belongs to.
#[allow(clippy::suboptimal_flops)]
fn formed(glyph: &GlyphData, scale: f64, dx: f64) -> Vec<Vec<lemmata_geom::QuadifyPoint>> {
    let Some((x_min, y_min, x_max, y_max)) = glyph.bounding_box() else {
        return glyph.contours.clone();
    };
    let centre = f64::midpoint(f64::from(x_min), f64::from(x_max));
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
            // those are part of what scales.
            let dot =
                extent(contour) * 4.0 < largest && contour.iter().all(|point| point.y > middle);
            contour
                .iter()
                .map(|point| lemmata_geom::QuadifyPoint {
                    x: if dot {
                        point.x + dx
                    } else {
                        centre + (point.x - centre) * scale + dx
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

/// The drawn forms, by glyph and form: a glyph the port builds in the scaled
/// frame, before its side is chosen.
pub type Drawn = BTreeMap<(String, Form), Vec<Vec<lemmata_geom::QuadifyPoint>>>;

/// The forms for every glyph that has a role, encoded from `first_code` upward.
#[must_use]
pub fn derive(glyphs: &[GlyphData], first_code: u32) -> Vec<GlyphData> {
    derive_with(glyphs, first_code, &Drawn::new())
}

/// The forms, using `drawn` where the port built the glyph and scaling the
/// captured outline everywhere else.
///
/// A drawn form is the definition re-run at `1 ± 1/12`, so a dot stays a dot and
/// a serif is re-drawn rather than stretched. Only the side is applied here,
/// because the caller does not know the glyph's advance.
#[must_use]
pub fn derive_with(glyphs: &[GlyphData], first_code: u32, drawn: &Drawn) -> Vec<GlyphData> {
    let mut derived = Vec::new();
    for (index, role) in classify(glyphs) {
        if role == Role::Stationary {
            continue;
        }
        let glyph = &glyphs[index];
        let shift = f64::from(glyph.advance) * SHIFT;
        for (form, scaled) in forms(glyph, glyph.advance) {
            let contours = match drawn.get(&(glyph.name.clone(), form)) {
                Some(contours) => translated(contours, form.shift() * shift),
                None => scaled,
            };
            let code = first_code + u32::try_from(derived.len()).unwrap_or(0);
            derived.push(GlyphData {
                name: form_name(&glyph.name, form),
                unicodes: vec![code],
                advance: glyph.advance,
                contours,
            });
        }
    }

    derived
}

/// The contours moved along x.
fn translated(
    contours: &[Vec<lemmata_geom::QuadifyPoint>],
    dx: f64,
) -> Vec<Vec<lemmata_geom::QuadifyPoint>> {
    contours
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
