//! The plan's combinations: every choice the capture offers, and every
//! combination of three of them.
//!
//! r[verify plan.combinations]
//!
//! The matrix is the groups the capture holds a family for and the variants each
//! one has, so a group with a family is walked in full rather than sampled.
//! Weight, width and slope are not here: those change every outline, so they are
//! a build per combination rather than a plan field, and one capture holds one of
//! each.
//!
//! The capture is separate from the coverage one because the outline tests are
//! calibrated against that one, and it is separate from the site's because the
//! site ships a font built from its own. Skips without it.

#![allow(clippy::cast_precision_loss)]

use std::fs;
use std::path::{Path, PathBuf};

use lemmata_build::font::{GlyphData, Metrics, Names};
use read_fonts::TableProvider;

fn dump_path() -> PathBuf {
    std::env::var_os("LEMMATA_VARIANTS_DUMP").map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("target/variants.bin")
        },
        PathBuf::from,
    )
}

/// Rounds a captured advance to the units the font table stores.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn advance_to_u16(advance: f64) -> u16 {
    advance.round().clamp(0.0, f64::from(u16::MAX)) as u16
}

fn glyph_data(captured: &[lemmata_capture::Glyph]) -> Vec<GlyphData> {
    captured
        .iter()
        .map(|glyph| GlyphData {
            name: glyph.name.clone(),
            unicodes: glyph.unicodes.clone(),
            advance: advance_to_u16(glyph.advance),
            contours: lemmata_capture::contours(glyph),
        })
        .collect()
}

/// The groups the capture holds a family for, and the variants each one has.
///
/// A group is a glyph with a codepoint and no dot in its name, and its family is
/// every glyph named `group.suffix` where the suffix is plain alphanumeric. The
/// suffix is spelled back as the plan writes it, so the matrix and the plan agree.
fn matrix(glyphs: &[GlyphData]) -> Vec<(usize, Vec<String>)> {
    let mut groups = Vec::new();
    for (index, glyph) in glyphs.iter().enumerate() {
        if glyph.unicodes.is_empty() || glyph.name.contains('.') {
            continue;
        }
        let prefix = format!("{}.", glyph.name);
        let mut variants: Vec<String> = glyphs
            .iter()
            .filter_map(|other| other.name.strip_prefix(&prefix))
            .filter(|suffix| {
                !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_alphanumeric())
            })
            .map(lemmata_build::character::kebab)
            .collect();
        variants.sort();
        variants.dedup();
        if !variants.is_empty() {
            groups.push((index, variants));
        }
    }

    groups
}

/// The bounding box of the glyph a codepoint reaches, read back from the font.
fn box_of(bytes: &[u8], code: u32) -> (i16, i16, i16, i16) {
    let font = read_fonts::FontRef::new(bytes).expect("reopen the font");
    let id = font
        .cmap()
        .expect("cmap")
        .map_codepoint(code)
        .unwrap_or_else(|| panic!("U+{code:04X} is not mapped"));
    let loca = font.loca(None).expect("loca");
    let glyf = font.glyf().expect("glyf");
    let glyph = loca
        .get_glyf(id, &glyf)
        .expect("read a glyph")
        .expect("the glyph has an outline");

    (glyph.x_min(), glyph.y_min(), glyph.x_max(), glyph.y_max())
}

/// The box a captured glyph's contours occupy.
fn captured_box(glyph: &GlyphData) -> (i16, i16, i16, i16) {
    glyph.bounding_box().expect("the glyph has ink")
}

#[test]
fn every_variant_choice_builds() {
    let Ok(data) = fs::read(dump_path()) else {
        eprintln!("skipping: no variants capture");
        return;
    };
    let captured = lemmata_capture::load(&data).expect("decode the capture");
    let glyphs = glyph_data(&captured);
    let groups = matrix(&glyphs);
    assert!(groups.len() >= 3, "the capture has families to walk");

    let mut choices = 0usize;
    for (index, variants) in &groups {
        let base = &glyphs[*index];
        let code = base.unicodes[0];
        for variant in variants {
            let plan = vec![(base.name.clone(), variant.clone())];
            let bytes =
                lemmata_build::font::build(&glyphs, &Metrics::default(), &Names::default(), &plan)
                    .unwrap_or_else(|error| panic!("{} = {variant}: {error}", base.name));

            // The codepoint reaches the drawing the plan named, not the one the
            // capture had it on.
            let drawn = glyphs
                .iter()
                .find(|glyph| {
                    glyph.name == lemmata_build::character::variant_name(&base.name, variant)
                })
                .unwrap_or_else(|| panic!("{} has no {variant}", base.name));
            assert_eq!(
                box_of(&bytes, code),
                captured_box(drawn),
                "{} = {variant} should draw that variant",
                base.name
            );
            choices += 1;
        }
    }
    eprintln!(
        "combinations: {choices} choices over {} groups, every one drawn",
        groups.len()
    );
}

#[test]
fn three_choices_together_build() {
    let Ok(data) = fs::read(dump_path()) else {
        eprintln!("skipping: no variants capture");
        return;
    };
    let captured = lemmata_capture::load(&data).expect("decode the capture");
    let glyphs = glyph_data(&captured);
    let groups = matrix(&glyphs);

    // Three groups crossed two ways each, which is the small full-factorial the
    // requirement names. Two variants per group rather than all of them: a
    // group's whole family is already walked one choice at a time above, and
    // crossing three full families is thousands of builds for the same answer.
    let picked: Vec<(usize, [&String; 2])> = groups
        .iter()
        .filter(|(_, variants)| variants.len() >= 2)
        .take(3)
        .map(|(index, variants)| (*index, [&variants[0], &variants[1]]))
        .collect();
    assert_eq!(picked.len(), 3, "the capture has three groups to cross");

    let mut built = 0usize;
    for a in picked[0].1 {
        for b in picked[1].1 {
            for c in picked[2].1 {
                let plan: Vec<(String, String)> = [
                    (&glyphs[picked[0].0].name, a),
                    (&glyphs[picked[1].0].name, b),
                    (&glyphs[picked[2].0].name, c),
                ]
                .iter()
                .map(|(name, variant)| ((*name).clone(), (*variant).clone()))
                .collect();
                let bytes = lemmata_build::font::build(
                    &glyphs,
                    &Metrics::default(),
                    &Names::default(),
                    &plan,
                )
                .unwrap_or_else(|error| panic!("{plan:?}: {error}"));

                // Each of the three lands on its own codepoint, so a combination
                // is three choices and not the last one winning.
                for (index, variant) in [(picked[0].0, a), (picked[1].0, b), (picked[2].0, c)] {
                    let base = &glyphs[index];
                    let drawn = glyphs
                        .iter()
                        .find(|glyph| {
                            glyph.name
                                == lemmata_build::character::variant_name(&base.name, variant)
                        })
                        .expect("the variant is captured");
                    assert_eq!(
                        box_of(&bytes, base.unicodes[0]),
                        captured_box(drawn),
                        "{} = {variant} in {plan:?}",
                        base.name
                    );
                }
                built += 1;
            }
        }
    }
    assert_eq!(
        built, 8,
        "three groups with two choices each is eight builds"
    );
    eprintln!("combinations: {built} builds over three groups at once");
}
