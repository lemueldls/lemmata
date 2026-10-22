//! Compares the ported control layer and knot builders against Iosevka's.
//!
//! `corpus/spiro-kit.json` describes control sequences; the golden file carries
//! the knots upstream's `SpiroFlattener` and `BiKnotCollector` produce from
//! them. Every field is compared exactly.

#![allow(clippy::cast_precision_loss)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use kurbo::Vec2;
use lemmata_ptl::{
    control::{Af, Item},
    spiro_kit::{Direction, Family, Kit, Position},
};
use serde_json::Value;

fn workspace_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn number(value: &Value) -> f64 {
    value.as_f64().expect("number")
}

fn direction(name: &str) -> Direction {
    match name {
        "up" | "u" => Direction::Up,
        "down" | "d" => Direction::Down,
        "left" | "l" => Direction::Left,
        "right" | "r" => Direction::Right,
        "ru" => Direction::RightUp,
        "rd" => Direction::RightDown,
        "lu" => Direction::LeftUp,
        "ld" => Direction::LeftDown,
        other => panic!("unknown direction {other}"),
    }
}

fn family(name: &str) -> Family {
    match name {
        "g4" => Family::G4,
        "g2" => Family::G2,
        "corner" => Family::Corner,
        "straight" => Family::Straight,
        "g2c" => Family::G2Corner,
        "cg2" => Family::CornerG2,
        "flatc" => Family::FlatCorner,
        "ccurl" => Family::CornerCurl,
        other => panic!("unknown family {other}"),
    }
}

fn amendment(kit: &Kit, spec: &Value) -> Af {
    let dir = || Vec2::new(number(&spec["x"]), number(&spec["y"]));
    match spec["kind"].as_str().expect("amendment kind") {
        "widths" => kit.widths(number(&spec["l"]), number(&spec["r"])),
        "widths.lhs" => kit.widths_lhs(spec.get("w").map(number)),
        "widths.rhs" => kit.widths_rhs(spec.get("w").map(number)),
        "widths.center" => kit.widths_center(spec.get("w").map(number)),
        "widths.heading" => kit.widths_heading(number(&spec["l"]), number(&spec["r"]), dir()),
        "widths.lhs.heading" => kit.widths_lhs_heading(spec.get("w").map(number), dir()),
        "widths.rhs.heading" => kit.widths_rhs_heading(spec.get("w").map(number), dir()),
        "widths.center.heading" => kit.widths_center_heading(spec.get("w").map(number), dir()),
        "heading" => kit.heading(dir()),
        "unimportant" => kit.unimportant(),
        "important" => kit.important(),
        "disable-contrast" => kit.disable_contrast(),
        other => panic!("unknown amendment {other}"),
    }
}

fn build(kit: &Kit, spec: &Value) -> Item {
    let af = match spec.get("af") {
        Some(af) if !af.is_null() => amendment(kit, af),
        _ => Vec::new(),
    };
    let x = || spec.get("x").map(number).unwrap_or(0.0);
    let y = || spec.get("y").map(number).unwrap_or(0.0);
    let kind = spec["kind"].as_str().expect("kind");

    match kind {
        "virt" => kit.virt(x(), y(), af),
        "close" => kit.close(),
        "end" => kit.end(),
        "archv" => {
            kit.archv(
                spec.get("samples")
                    .map(|v| usize::try_from(v.as_u64().unwrap()).unwrap()),
                spec.get("superness").map(number),
            )
        }
        "arcvh" => {
            kit.arcvh(
                spec.get("samples")
                    .map(|v| usize::try_from(v.as_u64().unwrap()).unwrap()),
                spec.get("superness").map(number),
            )
        }
        "alsoThru" => kit.also_thru(number(&spec["rx"]), number(&spec["ry"]), af),
        "alsoThruThem" => {
            let rs = spec["rs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|entry| {
                    let entry = entry.as_array().unwrap();
                    (number(&entry[0]), number(&entry[1]), number(&entry[2]))
                })
                .collect();
            let ty = kit
                .simple(spec["ty"].as_str().unwrap())
                .expect("interpolator knot type");

            kit.also_thru_them(rs, af, ty)
        }
        "decor" => {
            kit.decor(
                spec["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|item| build(kit, item))
                    .collect(),
            )
        }
        _ => {
            // A plain knot, or a directed pair named `family.direction.position`.
            if let Some(ty) = kit.simple(kind) {
                return kit.knot(ty, x(), y(), af);
            }
            let parts: Vec<&str> = kind.split('.').collect();

            match parts.as_slice() {
                [family_name, direction_name, position_name] => {
                    let position = match *position_name {
                        "start" => Position::Start,
                        "mid" => Position::Mid,
                        "end" => Position::End,
                        other => panic!("unknown position {other}"),
                    };
                    kit.pair(
                        family(family_name),
                        direction(direction_name),
                        position,
                        x(),
                        y(),
                        af,
                    )
                }
                _ => panic!("unknown control {kind}"),
            }
        }
    }
}

const fn knot_type_name(ty: lemmata_geom::KnotType) -> &'static str {
    use lemmata_geom::KnotType;
    match ty {
        KnotType::G4 => "g4",
        KnotType::G2 => "g2",
        KnotType::Corner => "corner",
        KnotType::Left => "left",
        KnotType::Right => "right",
        KnotType::Open => "open",
        KnotType::OpenEnd => "open_end",
    }
}

/// Coordinates are compared to this. The arc interpolators go through `cos`,
/// `sin` and `pow`, and V8's implementations are not bit-identical to Rust's,
/// which shows up around 3e-14. Everything structural is exact.
const TOLERANCE: f64 = 1e-9;

#[test]
fn control_layer_matches_upstream() {
    let corpus: Value =
        serde_json::from_str(&fs::read_to_string(workspace_path("corpus/spiro-kit.json")).unwrap())
            .unwrap();
    let golden: Value =
        serde_json::from_str(&fs::read_to_string(workspace_path("golden/spiro-kit.json")).unwrap())
            .unwrap();

    let cases = corpus["cases"].as_array().unwrap();
    let expected_cases = golden["cases"].as_array().unwrap();
    assert_eq!(cases.len(), expected_cases.len());

    let mut worst = 0.0_f64;
    for (case, expected) in cases.iter().zip(expected_cases) {
        let name = case["name"].as_str().unwrap();
        assert_eq!(name, expected["name"].as_str().unwrap(), "case order");
        let kit = Kit::new(
            number(&case["stroke"]),
            case.get("superness").map_or(1.0, number),
            number(&case["contrast"]),
            case.get("tiny").map_or(0.125, number),
        );

        let items: Vec<Item> = case["controls"]
            .as_array()
            .unwrap()
            .iter()
            .map(|spec| build(&kit, spec))
            .collect();
        let collector = kit.collect(items);

        assert_eq!(
            collector.closed,
            expected["closed"].as_bool().unwrap(),
            "{name}: closed"
        );
        assert_eq!(
            collector.contrast,
            number(&expected["contrast"]),
            "{name}: contrast"
        );

        let expected_knots = expected["knots"].as_array().unwrap();
        assert_eq!(
            collector.knots.len(),
            expected_knots.len(),
            "{name}: knot count"
        );

        for (index, (knot, expected_knot)) in collector.knots.iter().zip(expected_knots).enumerate()
        {
            let context = format!("{name} knot {index}");
            assert_eq!(
                knot_type_name(knot.ty),
                expected_knot["type"].as_str().unwrap(),
                "{context}: type"
            );
            for (actual, key) in [
                (knot.x, "x"),
                (knot.y, "y"),
                (knot.d1, "d1"),
                (knot.d2, "d2"),
            ] {
                let deviation = (actual - number(&expected_knot[key])).abs();
                worst = worst.max(deviation);
                assert!(
                    deviation <= TOLERANCE,
                    "{context}: {key} off by {deviation}"
                );
            }
            // The capture writes importance as a number.
            let expected_unimportant = expected_knot["unimportant"]
                .as_bool()
                .unwrap_or_else(|| expected_knot["unimportant"].as_u64() == Some(1));
            assert_eq!(
                knot.unimportant, expected_unimportant,
                "{context}: importance"
            );

            match (&knot.proposed_normal, &expected_knot["proposedNormal"]) {
                (Some(normal), Value::Object(_)) => {
                    worst =
                        worst.max((normal.x - number(&expected_knot["proposedNormal"]["x"])).abs());
                    worst =
                        worst.max((normal.y - number(&expected_knot["proposedNormal"]["y"])).abs());
                }
                (None, Value::Null) => {}
                (actual, expected) => panic!("{context}: normal {actual:?} against {expected}"),
            }
        }
    }
    eprintln!(
        "spiro kit: {} cases, {} knots, worst coordinate deviation {worst:.3e}",
        cases.len(),
        expected_cases
            .iter()
            .map(|case| case["knots"].as_array().unwrap().len())
            .sum::<usize>()
    );
}
