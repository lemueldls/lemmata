//! Compares the whole registry against the capture.
//!
//! The module tests check one definition each. This one checks that the list the
//! definitions assemble is the list the font can use: the names are unique and
//! the ones the capture holds reproduce its knots. Skips without a capture.

#![allow(clippy::cast_precision_loss)]

use std::collections::BTreeMap;
use std::fs;

use lemmata_geom::{BiKnot, Geometry};
use lemmata_glyphs::registry;
use lemmata_ptl::shapes::Design;

use lemmata_capture::{dump_path, metrics_from_capture};

const TOLERANCE: f64 = 1e-6;

fn knots_of(geometry: &Geometry) -> Vec<Vec<BiKnot>> {
    let mut nodes = Vec::new();
    lemmata_capture::collect_di_spiro(geometry, &mut nodes);

    nodes
        .iter()
        .map(|node| {
            let Geometry::DiSpiro { knots, .. } = node else {
                panic!("expected a di-spiro node");
            };
            knots.clone()
        })
        .collect()
}

#[test]
fn the_registry_matches_the_capture() {
    let Some(metrics) = metrics_from_capture() else {
        eprintln!("skipping: no capture");
        return;
    };
    let Ok(data) = fs::read(dump_path()) else {
        eprintln!("skipping: no capture");
        return;
    };
    let captured = lemmata_capture::load(&data).expect("decode capture");
    let built = registry::build(&metrics, &Design::default());

    let mut by_name: BTreeMap<&str, &Geometry> = BTreeMap::new();
    for (name, geometry) in &built {
        assert!(
            by_name.insert(name, geometry).is_none(),
            "{name} is built twice"
        );
    }

    let mut compared = 0usize;
    let mut knots = 0usize;
    let mut worst = (0.0_f64, "none".to_string());
    for glyph in &captured {
        let Some(ours) = by_name.get(glyph.name.as_str()) else {
            continue;
        };
        compared += 1;
        let ours = knots_of(ours);
        let theirs = knots_of(&glyph.geometry);
        assert_eq!(ours.len(), theirs.len(), "{}: dispiro count", glyph.name);
        for (index, (node, expected)) in ours.iter().zip(&theirs).enumerate() {
            assert_eq!(
                node.len(),
                expected.len(),
                "{} {index}: knot count",
                glyph.name
            );
            knots += node.len();
            for (knot, expected) in node.iter().zip(expected) {
                let deviation = (knot.x - expected.x).abs().max((knot.y - expected.y).abs());
                if deviation > worst.0 {
                    worst = (
                        deviation,
                        format!("{} {index} at ({}, {})", glyph.name, knot.x, knot.y),
                    );
                }
                assert!(
                    deviation <= TOLERANCE,
                    "{} {index}: knot at ({}, {}) is off by {deviation}",
                    glyph.name,
                    knot.x,
                    knot.y
                );
                assert_eq!(knot.ty, expected.ty, "{} {index}: knot type", glyph.name);
                assert_eq!(
                    knot.unimportant, expected.unimportant,
                    "{} {index}: importance",
                    glyph.name
                );
            }
        }
    }
    assert!(compared > 0, "the registry covers none of the capture");
    eprintln!(
        "registry: {} names, {compared} compared, {knots} knots, worst {:.9} on {}",
        built.len(),
        worst.0,
        worst.1
    );
}
