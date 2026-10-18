//! Character variants: the drawing a plan selects for a group of glyphs.
//!
//! r[impl plan.iosevka-keys]
//!
//! Iosevka names a variant in kebab case in the plan and in camel case in the
//! glyph source, so `a = "double-storey-serifed"` is the glyph
//! `a.doubleStoreySerifed`. The name is translated here rather than in the plan,
//! so a plan and an Iosevka plan read the same.
//!
//! The choice moves the *drawing* onto the glyph that carries the group's
//! codepoints rather than moving the codepoint to the variant's glyph, because
//! every feature the font writes names glyphs: `dtls` replaces `i`, texture
//! healing replaces `a` and `m`, and `ssty` names the math letters. A cmap that
//! pointed at `i.serifless` would leave all of them matching nothing. Iosevka
//! makes the same move, and it is why its compiled font still calls the chosen
//! variant `a`.

use crate::font::GlyphData;

/// The glyph a plan's choice names.
#[must_use]
pub fn variant_name(group: &str, variant: &str) -> String {
    format!("{group}.{}", camel(variant))
}

/// A kebab-case variant name in camel case: `double-storey-serifed` is
/// `doubleStoreySerifed`.
///
/// The glyph source composes the same name from one affix per stage, and for
/// every variant of the `a` family the two agree, which is checked against the
/// capture in `tests/variants.rs`.
#[must_use]
pub fn camel(variant: &str) -> String {
    let mut parts = variant.split('-');
    let mut name = parts.next().unwrap_or_default().to_string();
    for part in parts {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            name.extend(first.to_uppercase());
            name.push_str(chars.as_str());
        }
    }
    name
}

/// A camel-case glyph suffix back in the kebab case a plan writes:
/// `doubleStoreySerifed` is `double-storey-serifed`.
///
/// The page offers the variants a capture holds, and it offers them as plan
/// names, so the two directions have to agree. That they do for every variant of
/// every letter is checked in `tests/variants.rs`.
#[must_use]
pub fn kebab(variant: &str) -> String {
    let mut out = String::with_capacity(variant.len() + 4);
    for character in variant.chars() {
        if character.is_ascii_uppercase() {
            out.push('-');
            out.extend(character.to_lowercase());
        } else {
            out.push(character);
        }
    }

    out
}

/// The glyph set with the plan's choices applied.
///
/// A group whose variant is not in the set is left alone, so a capture that
/// lacks a family costs that one choice rather than the build.
#[must_use]
pub fn apply(glyphs: &[GlyphData], variants: &[(String, String)]) -> Vec<GlyphData> {
    if variants.is_empty() {
        return glyphs.to_vec();
    }
    let find = |name: &str| glyphs.iter().position(|glyph| glyph.name == name);
    let mut chosen: Vec<(usize, usize)> = Vec::new();
    for (group, variant) in variants {
        let Some(variant_index) = find(&variant_name(group, variant)) else {
            continue;
        };
        let Some(base_index) = find(group) else {
            continue;
        };
        chosen.push((base_index, variant_index));
    }

    let mut out = Vec::with_capacity(glyphs.len());
    for (index, glyph) in glyphs.iter().enumerate() {
        // The group's other drawings go. They are the alternatives the plan did
        // not choose, and one of them is about to become a duplicate of the
        // glyph the choice moved onto.
        if chosen.iter().any(|(base, variant)| {
            glyph.unicodes.is_empty()
                && (index == *variant
                    || glyph
                        .name
                        .strip_prefix(&glyphs[*base].name)
                        .is_some_and(|rest| rest.starts_with('.')))
        }) {
            continue;
        }
        match chosen.iter().find(|(base, _)| *base == index) {
            // The name and the codepoints stay: the choice is a drawing, and the
            // glyph the features name has to keep being the one that is reachable.
            Some((_, variant_index)) => {
                let variant = &glyphs[*variant_index];
                out.push(GlyphData {
                    name: glyph.name.clone(),
                    unicodes: glyph.unicodes.clone(),
                    advance: variant.advance,
                    contours: variant.contours.clone(),
                });
            }
            None => out.push(glyph.clone()),
        }
    }

    out
}
