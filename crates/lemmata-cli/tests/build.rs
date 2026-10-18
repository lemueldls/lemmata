//! Builds a font from a glyph capture and reads it back.
//!
//! Skips when no capture is present, like the other reference tests.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Rounds a captured advance to the units the font table stores.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn advance_to_u16(advance: f64) -> u16 {
    advance.round().clamp(0.0, f64::from(u16::MAX)) as u16
}

use lemmata_build::font::{GlyphData, Metrics, Names};
use lemmata_capture::Glyph;
use read_fonts::tables::glyf::Glyph as ReadGlyph;
use read_fonts::tables::head::MacStyle;
use read_fonts::tables::math::MathConstant;
use read_fonts::tables::os2::SelectionFlags;
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
    // The font carries `.notdef`, the captured glyphs, and the derived ones:
    // the size variants of the stretchy glyphs and their assembly pieces.
    let derived = lemmata_build::font::derived(&glyph_data(&captured));
    assert_eq!(
        font.maxp().expect("maxp").num_glyphs() as usize,
        captured.len() + 1 + derived.len()
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

/// `r[math.table]`: the constants the profile names survive the write and the
/// read, and the ones it marks mandatory-nonzero are not zero.
#[test]
fn math_table_round_trips() {
    let Some((bytes, _)) = build_font() else {
        eprintln!("skipping: no capture");
        return;
    };
    let font = FontRef::new(&bytes).expect("reopen the font");
    let math = font.math().expect("the font has a MATH table");
    let constants = math.math_constants().expect("MathConstants");

    let metrics = Metrics::default();
    let profile = lemmata_build::math::MathProfile::default();
    let expected = lemmata_build::math::constants(&metrics, &profile);

    // The scale percentages are what the profile decides.
    assert_eq!(
        constants.constant(MathConstant::ScriptPercentScaleDown),
        i32::from(profile.script_percent_scale_down)
    );
    assert_eq!(
        constants.constant(MathConstant::ScriptScriptPercentScaleDown),
        i32::from(profile.script_script_percent_scale_down)
    );

    // The rest follow from the font's measurements.
    assert_eq!(
        constants.constant(MathConstant::AxisHeight),
        i32::from(metrics.x_height / 2)
    );
    assert_eq!(
        constants.constant(MathConstant::AccentBaseHeight),
        i32::from(metrics.cap_height)
    );
    assert_eq!(
        constants.constant(MathConstant::FlattenedAccentBaseHeight),
        i32::from(metrics.cap_height)
    );
    for rule in [
        MathConstant::FractionRuleThickness,
        MathConstant::OverbarRuleThickness,
        MathConstant::UnderbarRuleThickness,
        MathConstant::RadicalRuleThickness,
    ] {
        assert_eq!(
            constants.constant(rule),
            i32::from(metrics.underline_thickness),
            "{rule:?} is drawn at the stroke width"
        );
    }

    // A constant the profile leaves at zero stays zero: the renderer decides.
    assert_eq!(constants.constant(MathConstant::MathLeading), 0);

    // r[math.table]: mandatory-nonzero constants are nonzero.
    for constant in [
        MathConstant::ScriptPercentScaleDown,
        MathConstant::ScriptScriptPercentScaleDown,
        MathConstant::AxisHeight,
    ] {
        assert_ne!(
            constants.constant(constant),
            0,
            "{constant:?} must not be zero"
        );
    }

    // The writer and the reader agree on what the profile produced.
    assert_eq!(
        expected.script_percent_scale_down,
        profile.script_percent_scale_down
    );
}

/// The style bits follow the italic angle, which is the one place the style is
/// decided. A renderer picks an italic face by these bits, so they have to
/// agree with the angle the glyphs were drawn at.
#[test]
fn italic_style_bits_follow_the_angle() {
    let Some((upright, captured)) = build_font() else {
        eprintln!("skipping: no capture");
        return;
    };

    let font = FontRef::new(&upright).expect("reopen the upright font");
    assert_eq!(
        font.head().expect("head").mac_style() & MacStyle::ITALIC,
        MacStyle::empty(),
        "an upright face is not italic"
    );
    assert!(
        font.os2()
            .expect("OS/2")
            .fs_selection()
            .contains(SelectionFlags::REGULAR)
    );
    assert!(
        !font
            .os2()
            .expect("OS/2")
            .fs_selection()
            .contains(SelectionFlags::ITALIC)
    );
    assert_eq!(
        font.post().expect("post").italic_angle().to_f64(),
        0.0,
        "an upright face has no angle"
    );

    // The same glyphs at a slant. The outlines do not move, since they come
    // from the capture; what changes is what the font says about itself.
    let metrics = Metrics {
        italic_angle: 15.0,
        ..Metrics::default()
    };
    let slanted = lemmata_build::font::build(
        &glyph_data(&captured),
        &metrics,
        &Names {
            subfamily: "Italic".to_string(),
            ..Names::default()
        },
    )
    .expect("build the slanted font");

    let font = FontRef::new(&slanted).expect("reopen the slanted font");
    assert_eq!(
        font.head().expect("head").mac_style() & MacStyle::ITALIC,
        MacStyle::ITALIC,
        "a slanted face is italic"
    );
    let selection = font.os2().expect("OS/2").fs_selection();
    assert!(selection.contains(SelectionFlags::ITALIC));
    assert!(
        !selection.contains(SelectionFlags::REGULAR),
        "the specification has REGULAR mean neither bold nor italic"
    );
    assert_eq!(
        font.post().expect("post").italic_angle().to_f64(),
        -15.0,
        "the angle is counter-clockwise, so a face leaning right is negative"
    );

    // The name table carries the style a renderer pairs with the family.
    let name = font.name().expect("name");
    let string_data = name.string_data();
    let subfamily = name
        .name_record()
        .iter()
        .find(|record| record.name_id() == NameId::SUBFAMILY_NAME && record.platform_id() == 3)
        .and_then(|record| record.string(string_data).ok())
        .map(|value| value.to_string())
        .expect("subfamily name");
    assert_eq!(subfamily, "Italic");
}

/// Where the slanted capture lives. `LEMMATA_ITALIC_DUMP` overrides it.
fn italic_dump_path() -> PathBuf {
    std::env::var_os("LEMMATA_ITALIC_DUMP")
        .map_or_else(|| workspace_path("target/math-italic.bin"), PathBuf::from)
}

/// The italic capture's outlines lean, which is what makes the face a face
/// rather than a label. The style bits say italic; this checks the ink.
///
/// The two captures hold different sets in a different order, so the
/// comparison is by codepoint rather than by position.
#[test]
#[allow(clippy::items_after_statements, clippy::cast_precision_loss)]
fn italic_outlines_lean() {
    let Some((_, upright)) = build_font() else {
        eprintln!("skipping: no capture");
        return;
    };
    let Ok(data) = std::fs::read(italic_dump_path()) else {
        eprintln!("skipping: no italic capture");
        return;
    };
    let slanted = lemmata_capture::load(&data).expect("decode the italic capture");

    /// How far a glyph's ink leans: its upper half's centre against its lower
    /// half's.
    fn lean(glyph: &Glyph) -> Option<f64> {
        let points: Vec<_> = lemmata_capture::contours(glyph)
            .into_iter()
            .flatten()
            .collect();
        let low = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let high = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
        let mid = f64::midpoint(low, high);
        let centre = |upper: bool| {
            let xs: Vec<f64> = points
                .iter()
                .filter(|p| (p.y >= mid) == upper)
                .map(|p| p.x)
                .collect();
            if xs.is_empty() {
                return None;
            }
            Some(f64::midpoint(
                xs.iter().copied().fold(f64::INFINITY, f64::min),
                xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            ))
        };
        Some(centre(true)? - centre(false)?)
    }

    let mut by_code = std::collections::HashMap::new();
    for glyph in &slanted {
        for code in &glyph.unicodes {
            by_code.insert(*code, glyph);
        }
    }

    let mut extra = 0.0;
    let mut compared = 0usize;
    for glyph in &upright {
        for code in &glyph.unicodes {
            if !(0x61..=0x7A).contains(code) {
                continue;
            }
            let Some(slanted) = by_code.get(code) else {
                continue;
            };
            let (Some(straight), Some(leaning)) = (lean(glyph), lean(slanted)) else {
                continue;
            };
            extra += leaning - straight;
            compared += 1;
            break;
        }
    }

    assert!(compared >= 20, "only {compared} letters could be compared");
    let mean = extra / compared as f64;
    assert!(
        mean > 5.0,
        "the italic capture leans {mean:.1} units more on average, which is not a slant"
    );
    eprintln!("italic: {compared} letters, {mean:.1} units more lean");
}

/// `r[build.license]`: the font carries the copyright notice and the license
/// statement the OFL asks for, and the license text ships beside it.
#[test]
fn license_records_are_present() {
    let Some((bytes, _)) = build_font() else {
        eprintln!("skipping: no capture");
        return;
    };
    let font = FontRef::new(&bytes).expect("reopen the font");
    let name = font.name().expect("name");
    let string_data = name.string_data();
    let read = |id: NameId| -> Option<String> {
        name.name_record()
            .iter()
            .find(|record| record.name_id() == id && record.platform_id() == 3)
            .and_then(|record| record.string(string_data).ok())
            .map(|value| value.to_string())
    };

    let copyright = read(NameId::COPYRIGHT_NOTICE).expect("a copyright notice");
    assert!(
        copyright.contains("Renzhi Li"),
        "the notice is upstream's, since the glyphs derive from it: {copyright}"
    );
    let description = read(NameId::LICENSE_DESCRIPTION).expect("a license statement");
    assert!(
        description.contains("SIL Open Font License"),
        "the statement names the license: {description}"
    );
    let url = read(NameId::LICENSE_URL).expect("a license URL");
    assert!(url.starts_with("https://"), "the URL is absolute: {url}");

    // The license text ships with the repository rather than only inside the
    // submodule, because a release artifact is distributed without it.
    let text = std::fs::read_to_string(workspace_path("LICENSE-OFL.md"))
        .expect("the license text is in the repository");
    assert!(
        text.contains("SIL Open Font License v1.1"),
        "the shipped text is the OFL itself"
    );
}

/// `r[build.valid]`'s other half: `ots-sanitize` accepts what the writer
/// produces.
///
/// Skips when the tool is absent, like the tests that skip without a capture.
/// `nix develop` provides it, and it is the gate that would name a malformed
/// table rather than leaving a renderer to reject the font with a shrug.
#[test]
fn ots_sanitize_accepts_the_font() {
    if Command::new("ots-sanitize")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("skipping: ots-sanitize is not installed");
        return;
    }
    let Some((bytes, _)) = build_font() else {
        eprintln!("skipping: no capture");
        return;
    };

    let path = workspace_path("target/ots-sanitize-check.ttf");
    std::fs::write(&path, &bytes).expect("write the font for the sanitizer");
    let output = Command::new("ots-sanitize")
        .arg(&path)
        .output()
        .expect("run ots-sanitize");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "ots-sanitize rejected the font:\n{stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("sanitized successfully") || stderr.contains("sanitized successfully"),
        "ots-sanitize said something unexpected:\n{stdout}{stderr}"
    );
}
