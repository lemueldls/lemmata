//! The plan's two promises: an empty plan is today's font, and a changed field
//! reaches the tables.

use lemmata_build::font::{GlyphData, Metrics, Names, build};
use lemmata_plan::Plan;

/// A font with the plan's metrics, built from the capture like the CLI does.
fn font_with(plan: &Plan) -> Vec<u8> {
    let metrics = plan.to_font_metrics();
    let glyphs = vec![GlyphData {
        name: "space".to_string(),
        unicodes: vec![32],
        advance: 500,
        contours: Vec::new(),
    }];
    build(&glyphs, &metrics, &Names::default(), &[]).expect("build")
}

#[test]
fn an_empty_plan_is_the_writer_default() {
    let plan = Plan::default();
    assert_eq!(plan.to_font_metrics().units_per_em, 1000);
    // Written out rather than compared by type, so a change to either default
    // has to be deliberate.
    assert_eq!(plan, lemmata_plan::from_json("{}").expect("parse"));
    assert_eq!(plan, lemmata_plan::from_json("").expect("parse"));
    assert_eq!(
        plan.to_font_metrics().cap_height,
        Metrics::default().cap_height
    );
}

#[test]
fn a_changed_field_reaches_the_font() {
    let base = font_with(&Plan::default());
    let tall = Plan {
        x_height: 600,
        ..Plan::default()
    };
    assert_ne!(base, font_with(&tall), "xHeight did not reach the font");

    let slanted = lemmata_plan::from_json(r#"{"italicAngle": -12.5}"#).expect("parse");
    assert_eq!(slanted.italic_angle, -12.5);
    assert_ne!(
        base,
        font_with(&slanted),
        "italicAngle did not reach the font"
    );

    let heavier = lemmata_plan::from_json(r#"{"weight": 700}"#).expect("parse");
    assert_eq!(heavier.weight_class, 700);
    assert_ne!(base, font_with(&heavier), "weight did not reach the font");
}

#[test]
fn a_partial_plan_keeps_the_rest() {
    let plan = lemmata_plan::from_json(r#"{"capHeight": 800, "xHeight": 550}"#).expect("parse");
    assert_eq!(plan.cap_height, 800);
    assert_eq!(plan.x_height, 550);
    assert_eq!(plan.units_per_em, Plan::default().units_per_em);
    assert_eq!(plan.descender, Plan::default().descender);
}

#[test]
fn a_malformed_plan_is_an_error() {
    assert!(lemmata_plan::from_json("[1, 2]").is_err());
    assert!(lemmata_plan::from_json(r#"{"capHeight": "tall"}"#).is_err());
}
