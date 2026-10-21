//! A plan's width is the advance, and the capture is centred in it.
//!
//! The capture was drawn at one width. A plan that asks for another width is
//! asking for a frame the capture does not hold, so the advance has to follow
//! the plan and an unported outline has to move with it. Ported outlines are
//! drawn in the plan's frame already and are left alone.

use lemmata_build::font::GlyphData;
use lemmata_capture::dump_path;
use lemmata_plan::Plan;

fn captured() -> Option<Vec<lemmata_capture::Glyph>> {
    let data = std::fs::read(dump_path()).ok()?;

    Some(lemmata_capture::load(&data).expect("decode the capture"))
}

fn contours<'a>(glyphs: &'a [GlyphData], name: &str) -> &'a [Vec<lemmata_geom::QuadifyPoint>] {
    &glyphs
        .iter()
        .find(|glyph| glyph.name == name)
        .unwrap_or_else(|| panic!("{name} is not in the set"))
        .contours
}

fn left(contours: &[Vec<lemmata_geom::QuadifyPoint>]) -> f64 {
    contours
        .iter()
        .flatten()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min)
}

fn ink(contours: &[Vec<lemmata_geom::QuadifyPoint>]) -> f64 {
    let left = left(contours);
    let right = contours
        .iter()
        .flatten()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);

    right - left
}

#[test]
fn a_wider_plan_is_a_wider_advance() {
    let Some(captured) = captured() else {
        eprintln!("skipping: no capture");
        return;
    };
    let plan = lemmata_plan::from_json(r#"{"width": 600}"#).expect("parse");
    assert_eq!(plan.width, 600.0);

    let glyphs = lemmata_source::glyphs(&plan, &captured);
    let space = glyphs
        .iter()
        .find(|glyph| glyph.name == "space")
        .expect("the capture has a space");
    assert_eq!(
        space.advance, 600,
        "a one-cell glyph takes the plan's width"
    );

    // A combining mark adds no space between its base and what follows, and a
    // two-cell glyph keeps the cell pair the capture drew.
    for glyph in &glyphs {
        if glyph.name.starts_with("dotAbove") {
            assert_eq!(glyph.advance, 0, "{} gained an advance", glyph.name);
        }
    }
}

#[test]
fn an_unported_outline_moves_into_the_wider_frame() {
    let Some(captured) = captured() else {
        eprintln!("skipping: no capture");
        return;
    };
    let narrow = lemmata_source::glyphs(&Plan::default(), &captured);
    let wide = lemmata_source::glyphs(
        &lemmata_plan::from_json(r#"{"width": 600}"#).expect("parse"),
        &captured,
    );

    // `dollar` is not a ported definition, so it is the capture's drawing
    // moved half the extra advance.
    let moved = left(contours(&wide, "dollar")) - left(contours(&narrow, "dollar"));
    assert!(
        (moved - 50.0).abs() < 1e-6,
        "dollar moved {moved}, expected half of the 100-unit gain"
    );
    assert!(
        (ink(contours(&wide, "dollar")) - ink(contours(&narrow, "dollar"))).abs() < 1e-6,
        "the capture's drawing should keep its size"
    );

    // `m` is ported, so the width redraws it instead of moving the capture.
    assert!(
        (ink(contours(&wide, "m")) - ink(contours(&narrow, "m"))).abs() > 0.5,
        "m should be redrawn at the wider grade"
    );
}
