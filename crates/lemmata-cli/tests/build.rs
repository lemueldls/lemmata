//! Builds a font from a glyph capture and reads it back.
//!
//! Skips when no capture is present, like the other reference tests.

use std::path::{Path, PathBuf};

/// Rounds a captured advance to the units the font table stores.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn advance_to_u16(advance: f64) -> u16 {
    advance.round().clamp(0.0, f64::from(u16::MAX)) as u16
}

use lemmata_build::font::{GlyphData, Metrics, Names};
use lemmata_capture::Glyph;
use read_fonts::tables::glyf::Glyph as ReadGlyph;
use read_fonts::types::NameId;
use read_fonts::{FontRef, TableProvider};

/// Allowed bounding box difference against Iosevka, in font units.
const BBOX_TOLERANCE: i16 = 10;

fn workspace_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn dump_path() -> PathBuf {
    std::env::var_os("LEMMATA_GLYPH_DUMP")
        .map_or_else(|| workspace_path("target/glyphs.bin"), PathBuf::from)
}

fn glyph_data(captured: &[Glyph]) -> Vec<GlyphData> {
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

fn build_font() -> Option<(Vec<u8>, Vec<Glyph>)> {
    let data = std::fs::read(dump_path()).ok()?;
    let captured = lemmata_capture::load(&data).expect("decode capture");
    let glyphs = glyph_data(&captured);
    let bytes = lemmata_build::font::build(&glyphs, &Metrics::default(), &Names::default())
        .expect("build the font");
    Some((bytes, captured))
}

/// The glyph's bounding box in a font, or `None` for an empty glyph.
fn font_bbox(font: &FontRef, glyph_id: read_fonts::types::GlyphId) -> Option<(i16, i16, i16, i16)> {
    let loca = font.loca(None).expect("loca");
    let glyf = font.glyf().expect("glyf");
    let glyph = loca.get_glyf(glyph_id, &glyf).expect("read glyph")?;
    Some((glyph.x_min(), glyph.y_min(), glyph.x_max(), glyph.y_max()))
}

#[test]
fn built_font_round_trips() {
    // r[verify build.valid]
    let Some((bytes, captured)) = build_font() else {
        eprintln!("skipping: no capture");
        return;
    };
    let font = FontRef::new(&bytes).expect("parse the built font");

    assert_eq!(font.head().expect("head").units_per_em(), 1000);
    assert_eq!(
        font.maxp().expect("maxp").num_glyphs() as usize,
        captured.len() + 1
    );

    let cmap = font.cmap().expect("cmap");
    let hmtx = font.hmtx().expect("hmtx");
    let name = font.name().expect("name");
    let string_data = name.string_data();
    let family = name
        .name_record()
        .iter()
        .find(|record| record.name_id() == NameId::FAMILY_NAME && record.platform_id() == 3)
        .and_then(|record| record.string(string_data).ok())
        .map(|value| value.to_string())
        .expect("family name");
    assert_eq!(family, "Lemmata");

    let mut checked = 0;
    for (glyph, data) in captured.iter().zip(glyph_data(&captured)) {
        for unicode in &glyph.unicodes {
            let Some(ch) = char::from_u32(*unicode) else {
                continue;
            };
            let Some(glyph_id) = cmap.map_codepoint(ch) else {
                continue;
            };
            assert!(glyph_id.to_u32() > 0, "U+{unicode:04X} is not .notdef");
            assert_eq!(
                hmtx.advance(glyph_id).expect("advance"),
                data.advance,
                "U+{unicode:04X} advance"
            );
            match (data.bounding_box(), font_bbox(&font, glyph_id)) {
                (Some(expected), Some(actual)) => assert_eq!(
                    expected, actual,
                    "U+{unicode:04X} bbox (glyph {})",
                    glyph.name
                ),
                (None, None) => {}
                (expected, actual) => {
                    panic!("U+{unicode:04X}: expected {expected:?}, got {actual:?}")
                }
            }
            checked += 1;
        }
    }
    assert!(checked > 0, "the capture had no mapped codepoints");

    // 'a' should be a simple glyph with an outline.
    let glyph_id = cmap.map_codepoint('a').expect("'a'");
    let loca = font.loca(None).expect("loca");
    let glyf = font.glyf().expect("glyf");
    let glyph = loca
        .get_glyf(glyph_id, &glyf)
        .expect("read 'a'")
        .expect("'a' has an outline");
    match glyph {
        ReadGlyph::Simple(simple) => assert!(simple.number_of_contours() > 0),
        ReadGlyph::Composite(_) => panic!("'a' should be a simple glyph"),
    }
}

/// The built font should agree with Iosevka's own outline for the same glyphs.
#[test]
fn built_font_matches_the_reference() {
    let reference_path = workspace_path("target/reference/RefA-Regular.ttf");
    let Ok(reference_bytes) = std::fs::read(&reference_path) else {
        eprintln!(
            "skipping: no reference font at {}",
            reference_path.display()
        );
        return;
    };
    let Some((bytes, captured)) = build_font() else {
        eprintln!("skipping: no capture");
        return;
    };
    let ours = FontRef::new(&bytes).expect("parse the built font");
    let reference = FontRef::new(&reference_bytes).expect("parse the reference font");
    let our_cmap = ours.cmap().expect("our cmap");
    let reference_cmap = reference.cmap().expect("reference cmap");

    // r[verify drift.vmetrics]
    let our_head = ours.head().expect("our head");
    let reference_head = reference.head().expect("reference head");
    assert_eq!(our_head.units_per_em(), reference_head.units_per_em());
    let our_hhea = ours.hhea().expect("our hhea");
    let reference_hhea = reference.hhea().expect("reference hhea");
    assert_eq!(
        our_hhea.ascender().to_i16(),
        reference_hhea.ascender().to_i16(),
        "ascender"
    );
    assert_eq!(
        our_hhea.descender().to_i16(),
        reference_hhea.descender().to_i16(),
        "descender"
    );
    assert_eq!(
        our_hhea.line_gap().to_i16(),
        reference_hhea.line_gap().to_i16(),
        "line gap"
    );
    let our_os2 = ours.os2().expect("our OS/2");
    let reference_os2 = reference.os2().expect("reference OS/2");
    assert_eq!(our_os2.sx_height(), reference_os2.sx_height(), "x-height");
    assert_eq!(
        our_os2.s_cap_height(),
        reference_os2.s_cap_height(),
        "cap height"
    );

    let our_hmtx = ours.hmtx().expect("our hmtx");
    let reference_hmtx = reference.hmtx().expect("reference hmtx");

    // The bounding box is a coarse measure, but it is the one that survives
    // independently of how a boolean engine splits contours.
    let mut worst = 0i16;
    let mut worst_name = String::new();
    let mut compared = 0;
    for glyph in &captured {
        for unicode in &glyph.unicodes {
            let Some(ch) = char::from_u32(*unicode) else {
                continue;
            };
            let (Some(our_id), Some(reference_id)) =
                (our_cmap.map_codepoint(ch), reference_cmap.map_codepoint(ch))
            else {
                continue;
            };
            let (Some(ours_box), Some(reference_box)) = (
                font_bbox(&ours, our_id),
                font_bbox(&reference, reference_id),
            ) else {
                continue;
            };
            // r[verify drift.advance]
            assert_eq!(
                our_hmtx.advance(our_id).expect("our advance"),
                reference_hmtx
                    .advance(reference_id)
                    .expect("reference advance"),
                "U+{unicode:04X} advance"
            );
            let deviation = [
                (ours_box.0 - reference_box.0).abs(),
                (ours_box.1 - reference_box.1).abs(),
                (ours_box.2 - reference_box.2).abs(),
                (ours_box.3 - reference_box.3).abs(),
            ]
            .into_iter()
            .max()
            .unwrap_or(0);
            if deviation > worst {
                worst = deviation;
                worst_name = format!("{} U+{unicode:04X}", glyph.name);
            }
            compared += 1;
        }
    }
    eprintln!("bounding boxes: {compared} compared, worst {worst} on {worst_name}");
    assert!(compared > 0, "no glyphs to compare");
    assert!(
        worst <= BBOX_TOLERANCE,
        "worst bounding box deviation {worst} on {worst_name} over {BBOX_TOLERANCE}"
    );
}

/// Iosevka's names are reserved. The default family must avoid them.
///
/// r[verify build.name]
#[test]
fn output_family_is_not_an_iosevka_name() {
    let Some((bytes, _)) = build_font() else {
        eprintln!("skipping: no capture");
        return;
    };
    let font = FontRef::new(&bytes).expect("parse the built font");
    let name = font.name().expect("name");
    let string_data = name.string_data();
    let family = name
        .name_record()
        .iter()
        .find(|record| record.name_id() == NameId::FAMILY_NAME && record.platform_id() == 3)
        .and_then(|record| record.string(string_data).ok())
        .map(|value| value.to_string())
        .expect("family name");
    for reserved in [
        "Iosevka",
        "Iosevka Aile",
        "Iosevka Comfy",
        "Iosevka Curly",
        "Iosevka Etoile",
        "Iosevka Fixed",
        "Iosevka Slab",
        "Iosevka Term",
    ] {
        assert_ne!(family, reserved, "the family name is reserved by Iosevka");
    }
}
