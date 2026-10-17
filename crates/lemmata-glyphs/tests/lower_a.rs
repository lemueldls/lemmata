//! Compares the ported double-storey `a` variants against the build's own.
//!
//! The capture carries each variant's geometry tree, which is the union of the
//! `dispiro` sequences the variant's body builds. Comparing the bi-knots of
//! those sequences catches a mistake in the shape logic before the solver or
//! the boolean stage can hide it. Skips without a capture.

#![allow(clippy::cast_precision_loss)]

mod util;

use std::fs;

use lemmata_glyphs::letter_model::DivFrame;
use lemmata_glyphs::lower_a::{DoubleStorey, HOOK_SLAB_NONE};
use lemmata_glyphs::shapes::Design;
use lemmata_glyphs::spiro_kit::{Dispiro, Kit};
use serde_json::Value;

use util::{dump_path, metrics_from_capture};

/// Knot positions and normals are exact between the two implementations.
const TOLERANCE: f64 = 1e-6;

#[test]
fn double_storey_matches_capture() {
    let Some(metrics) = metrics_from_capture() else {
        eprintln!("skipping: no capture");
        return;
    };
    let Ok(data) = fs::read(dump_path()) else {
        eprintln!("skipping: no capture");
        return;
    };
    let Ok(expansions) = fs::read_to_string(format!("{}.expansions.json", dump_path().display()))
    else {
        eprintln!("skipping: no expansions");
        return;
    };
    let expansions: Value = serde_json::from_str(&expansions).expect("parse expansions");
    let expansions = expansions.as_array().expect("array");

    let glyphs = lemmata_glyphs::load(&data).expect("decode capture");
    let design = Design::default();
    let df = DivFrame::new(metrics.clone(), Some(1.0), None, None);
    let kit = Kit::new(
        metrics.stroke,
        design.superness,
        metrics.contrast,
        metrics.tiny,
    );
    let body = DoubleStorey {
        df: &df,
        metrics: &metrics,
        kit: &kit,
        design: &design,
    };

    let cases: Vec<(&str, Vec<Dispiro>)> = vec![
        ("a", body.serifless(HOOK_SLAB_NONE, None)),
        ("a.doubleStoreySerifed", body.serifed(HOOK_SLAB_NONE, None)),
    ];

    let mut worst = (0.0_f64, "none".to_string());
    let mut knots = 0usize;
    for (name, ours) in &cases {
        let glyph = glyphs
            .iter()
            .find(|glyph| glyph.name == *name)
            .unwrap_or_else(|| panic!("{name} is not in the capture"));
        let mut captured = Vec::new();
        lemmata_glyphs::collect_di_spiro(&glyph.geometry, &mut captured);
        assert_eq!(captured.len(), ours.len(), "{name}: dispiro count");

        for (index, (dispiro, expected)) in ours.iter().zip(&captured).enumerate() {
            let lemmata_geom::Geometry::DiSpiro {
                contrast,
                closed,
                knots: expected_knots,
                ..
            } = expected
            else {
                unreachable!()
            };
            assert_eq!(dispiro.contrast, *contrast, "{name} {index}: contrast");
            assert_eq!(dispiro.closed, *closed, "{name} {index}: closed");
            assert_eq!(
                dispiro.knots.len(),
                expected_knots.len(),
                "{name} {index}: knot count"
            );
            knots += dispiro.knots.len();
            for (knot, expected_knot) in dispiro.knots.iter().zip(expected_knots) {
                let deviation = (knot.x - expected_knot.x)
                    .abs()
                    .max((knot.y - expected_knot.y).abs());
                if deviation > worst.0 {
                    worst = (
                        deviation,
                        format!("{name} {index} at ({}, {})", knot.x, knot.y),
                    );
                }
                assert!(
                    deviation <= TOLERANCE,
                    "{name} {index}: knot at ({}, {}) is off by {deviation}",
                    knot.x,
                    knot.y
                );
                assert_eq!(
                    knot.ty, expected_knot.ty,
                    "{name} {index}: knot type at ({}, {})",
                    knot.x, knot.y
                );
                assert_eq!(
                    knot.unimportant, expected_knot.unimportant,
                    "{name} {index}: importance at ({}, {})",
                    knot.x, knot.y
                );
                // A heading decides the normal the solver starts from, so it
                // has to match, not just the knot's position.
                match (&knot.proposed_normal, &expected_knot.proposed_normal) {
                    (Some(a), Some(b)) => assert!(
                        (a.x - b.x).abs().max((a.y - b.y).abs()) <= 1e-9,
                        "{name} {index}: normal at ({}, {}) is ({}, {}), not ({}, {})",
                        knot.x,
                        knot.y,
                        a.x,
                        a.y,
                        b.x,
                        b.y
                    ),
                    (None, None) => {}
                    _ => panic!(
                        "{name} {index}: one side has a normal at ({}, {}) and the other does not",
                        knot.x, knot.y
                    ),
                }
            }
        }

        // The expansion is what the solver consumes, so the port has to
        // reproduce the build's sides too, not just the bi-knots.
        let expected_expansions = expansions
            .iter()
            .find(|entry| entry["name"] == *name)
            .expect("expansions entry")["expansions"]
            .as_array()
            .expect("array");
        assert_eq!(
            expected_expansions.len(),
            ours.len(),
            "{name}: expansion count"
        );
        for (index, (dispiro, expected)) in ours.iter().zip(expected_expansions).enumerate() {
            let result = lemmata_geom::di_spiro_expand(
                kurbo::Affine::IDENTITY,
                dispiro.contrast,
                dispiro.closed,
                &dispiro.knots,
            );
            for (side, side_knots) in [
                ("lhs", &result.lhs_untransformed),
                ("rhs", &result.rhs_untransformed),
            ] {
                let theirs = expected[side].as_array().unwrap();
                assert_eq!(
                    side_knots.len(),
                    theirs.len(),
                    "{name} {index}: {side} knot count"
                );
                for (knot, expected_knot) in side_knots.iter().zip(theirs) {
                    let deviation = (knot.x - expected_knot[0].as_f64().unwrap())
                        .abs()
                        .max((knot.y - expected_knot[1].as_f64().unwrap()).abs());
                    assert!(
                        deviation <= TOLERANCE,
                        "{name} {index}: {side} knot at ({}, {}) is off by {deviation}",
                        knot.x,
                        knot.y
                    );
                }
            }
        }
    }
    eprintln!(
        "lower a: {} variants, {} knots, worst deviation {:.9} on {}",
        cases.len(),
        knots,
        worst.0,
        worst.1
    );
}
