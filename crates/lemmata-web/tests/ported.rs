//! Checks that the site builds the ported glyphs from the runtime.
//!
//! The registry is compared with the capture the page ships: every glyph the
//! port draws has to cover the same region as the captured one, because the
//! swap must not move the drawing. Skips without a capture.

#![allow(clippy::cast_precision_loss)]

use std::fs;

use lemmata_capture::{contours, dump_path, load, metrics_from_capture};
use lemmata_plan::Plan;
use read_fonts::TableProvider;

const BOX_TOLERANCE: f64 = 1.0;

fn bounding_box(contours: &lemmata_source::Contours) -> Option<(f64, f64, f64, f64)> {
    contours.iter().flatten().fold(None, |acc, point| {
        Some(
            acc.map_or((point.x, point.y, point.x, point.y), |(l, b, r, t)| {
                (
                    l.min(point.x),
                    b.min(point.y),
                    r.max(point.x),
                    t.max(point.y),
                )
            }),
        )
    })
}

#[test]
fn the_site_draws_the_ported_glyphs() {
    if metrics_from_capture().is_none() {
        eprintln!("skipping: no capture");
        return;
    }
    let Ok(data) = fs::read(dump_path()) else {
        eprintln!("skipping: no capture");
        return;
    };
    let captured = load(&data).expect("decode capture");
    let ported = lemmata_source::ported(&Plan::default());

    for name in [
        "a.doubleStoreySerifed",
        "a.singleStoreySerifless",
        "aCap.singleStoreyTailed",
        "b.serifless",
        "d.toothedSerifless",
        "D.standardSerifless",
        "E.serifless",
        "F.serifless",
        "f.serifless",
        "I.serifed",
        "L.serifless",
        "p.earedSerifless",
    ] {
        assert!(ported.contains_key(name), "{name} is not ported");
    }

    let mut compared = 0usize;
    for glyph in &captured {
        let Some(ours) = ported.get(&glyph.name) else {
            continue;
        };
        let Some(our_box) = bounding_box(ours) else {
            continue;
        };
        let theirs = contours(glyph);
        let Some(their_box) = bounding_box(&theirs) else {
            continue;
        };
        compared += 1;
        for (side, (our, their)) in [
            ("left", (our_box.0, their_box.0)),
            ("bottom", (our_box.1, their_box.1)),
            ("right", (our_box.2, their_box.2)),
            ("top", (our_box.3, their_box.3)),
        ] {
            assert!(
                (our - their).abs() <= BOX_TOLERANCE,
                "{}: outline {side} is {our}, not {their}",
                glyph.name
            );
        }
    }
    assert!(compared > 100, "only {compared} glyphs were compared");
    eprintln!(
        "ported: {} glyphs, {compared} compared with the capture",
        ported.len()
    );
}

#[test]
fn the_build_succeeds_with_the_ported_glyphs() {
    let Ok(capture) = fs::read(dump_path()) else {
        eprintln!("skipping: no capture");
        return;
    };

    let bytes = lemmata_web::build(&capture, "").expect("build");
    let font = read_fonts::FontRef::new(&bytes).expect("reopen");
    assert!(
        font.table_data(read_fonts::types::Tag::new(b"glyf"))
            .is_some(),
        "the font has no outlines"
    );
}

#[test]
fn a_plan_can_choose_a_variant_the_capture_never_held() {
    let Ok(capture) = fs::read(dump_path()) else {
        eprintln!("skipping: no capture");
        return;
    };
    let names = lemmata_web::registry_names();
    for name in [
        "b.flatBottomSerifed",
        "d.flatBottomSerifed",
        "p.flatTopSerifed",
    ] {
        assert!(
            names.lines().any(|candidate| candidate == name),
            "{name} is not in the registry"
        );
    }

    let plain = lemmata_web::build(&capture, "").expect("build");
    let chosen = lemmata_web::build(&capture, r#"{"variants": {"b": "flat-bottom-serifed"}}"#)
        .expect("build with a ported variant");
    assert_ne!(plain, chosen, "the b variant did not reach the font");
}

#[test]
fn the_registry_font_maps_the_variants() {
    let bytes = lemmata_web::registry_font("").expect("registry font");
    let font = read_fonts::FontRef::new(&bytes).expect("reopen");
    let cmap = font.cmap().expect("cmap");
    for codepoint in [0xE000_u32, 0xE001, 0xE002] {
        assert!(
            cmap.map_codepoint(codepoint).is_some(),
            "U+{codepoint:04X} is not mapped"
        );
    }
    eprintln!("registry font: {} bytes", bytes.len());
}

#[test]
fn the_registry_group_font_maps_one_group() {
    let bytes = lemmata_web::registry_group("", "a").expect("group font");
    let font = read_fonts::FontRef::new(&bytes).expect("reopen");
    let cmap = font.cmap().expect("cmap");
    for codepoint in [0xE000_u32, 0xE001, 0xE002] {
        assert!(
            cmap.map_codepoint(codepoint).is_some(),
            "U+{codepoint:04X} is not mapped"
        );
    }
    let bold = lemmata_web::registry_group(r#"{"weight": 700}"#, "a").expect("bold group font");
    assert_ne!(bytes, bold, "the group font did not follow the plan");
}

#[test]
fn the_registry_index_font_maps_each_letter() {
    let bytes = lemmata_web::registry_index("").expect("index font");
    let font = read_fonts::FontRef::new(&bytes).expect("reopen");
    let cmap = font.cmap().expect("cmap");
    for letter in ['a', 'b', 'd', 'D'] {
        assert!(
            cmap.map_codepoint(u32::from(letter)).is_some(),
            "{letter} is not mapped"
        );
    }
}

#[test]
fn a_preview_responds_to_the_plan() {
    let Ok(capture) = fs::read(dump_path()) else {
        eprintln!("skipping: no capture");
        return;
    };

    let light = lemmata_web::build_preview(&capture, "", "aaa").expect("preview");
    let bold = lemmata_web::build_preview(&capture, r#"{"weight":700}"#, "aaa").expect("preview");
    assert_ne!(light, bold, "the preview drew the same font at two weights");
    read_fonts::FontRef::new(&light).expect("reopen the light preview");
    read_fonts::FontRef::new(&bold).expect("reopen the bold preview");
}
