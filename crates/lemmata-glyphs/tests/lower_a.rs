//! Compares the ported double-storey `a` against the build's own.
//!
//! The capture carries `a`'s geometry tree, which is the union of the two
//! `dispiro` sequences `DoubleStorey.Serifless` builds. Comparing the bi-knots
//! of those sequences catches a mistake in the shape logic before the solver
//! or the boolean stage can hide it. Skips without a capture.

#![allow(clippy::cast_precision_loss)]

mod util;

use std::fs;

use lemmata_glyphs::letter_model::DivFrame;
use lemmata_glyphs::lower_a::{DoubleStorey, HOOK_SLAB_NONE};
use lemmata_glyphs::shapes::Design;
use lemmata_glyphs::spiro_kit::Kit;

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
    let glyphs = lemmata_glyphs::load(&data).expect("decode capture");
    let a = glyphs.iter().find(|glyph| glyph.name == "a").expect("a");

    let mut captured = Vec::new();
    lemmata_glyphs::collect_di_spiro(&a.geometry, &mut captured);
    assert_eq!(captured.len(), 2, "a: dispiro count");

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
    let ours = body.serifless(HOOK_SLAB_NONE, None);
    assert_eq!(ours.len(), 2, "serifless: dispiro count");

    let mut worst = (0.0_f64, "none".to_string());
    for (index, (dispiro, expected)) in ours.iter().zip(&captured).enumerate() {
        let lemmata_geom::Geometry::DiSpiro {
            contrast,
            closed,
            knots,
            ..
        } = expected
        else {
            unreachable!()
        };
        assert_eq!(dispiro.contrast, *contrast, "operand {index}: contrast");
        assert_eq!(dispiro.closed, *closed, "operand {index}: closed");
        assert_eq!(
            dispiro.knots.len(),
            knots.len(),
            "operand {index}: knot count"
        );
        for (knot, expected_knot) in dispiro.knots.iter().zip(knots) {
            let deviation = (knot.x - expected_knot.x)
                .abs()
                .max((knot.y - expected_knot.y).abs());
            if deviation > worst.0 {
                worst = (
                    deviation,
                    format!("operand {index} at ({}, {})", knot.x, knot.y),
                );
            }
            assert!(
                deviation <= TOLERANCE,
                "operand {index}: knot at ({}, {}) is off by {deviation}",
                knot.x,
                knot.y
            );
            assert_eq!(
                knot.ty, expected_knot.ty,
                "operand {index}: knot type at ({}, {})",
                knot.x, knot.y
            );
            assert_eq!(
                knot.unimportant, expected_knot.unimportant,
                "operand {index}: importance at ({}, {})",
                knot.x, knot.y
            );
            assert_eq!(
                knot.proposed_normal.is_some(),
                expected_knot.proposed_normal.is_some(),
                "operand {index}: whether a normal was proposed at ({}, {})",
                knot.x,
                knot.y
            );
        }
    }
    eprintln!(
        "lower a: {} operands, {} knots, worst deviation {:.9} on {}",
        ours.len(),
        ours.iter()
            .map(|dispiro| dispiro.knots.len())
            .sum::<usize>(),
        worst.0,
        worst.1
    );
}
