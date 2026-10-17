//! Compares our geometry against Iosevka's, stage by stage.
//!
//! The glyph capture carries each stage's output, so a divergence can be
//! pinned to the stage that produced it: expansion, the spiro solve, the arcs
//! of the tree, or the arcs after overlap resolution. Skips when no capture is
//! present.
//!
//! These comparisons are exact where they can be. The last one, which runs
//! through overlap resolution, is not: the two boolean engines differ on
//! near-tangential crossings, and that residual is what the glyph tests bound.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::suboptimal_flops
)]

use std::fs;
use std::path::{Path, PathBuf};

use kurbo::{BezPath, ParamCurve, ParamCurveNearest, Point};
use lemmata_geom::{Bez3, Geometry, MonoKnot};
use serde_json::Value;

/// Samples per segment.
const SAMPLES_PER_SEGMENT: usize = 16;
/// Knot positions are exact between the two implementations.
const KNOT_TOLERANCE: f64 = 1e-6;
/// Raw arc samples are exact too.
const ARC_TOLERANCE: f64 = 1e-6;

fn workspace_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn dump_path() -> PathBuf {
    std::env::var_os("LEMMATA_GLYPH_DUMP")
        .map_or_else(|| workspace_path("target/glyphs.bin"), PathBuf::from)
}

fn read_reference(suffix: &str) -> Value {
    let path = dump_path();
    let name = if suffix.is_empty() {
        format!("{}.json", path.display())
    } else {
        format!("{}.{suffix}.json", path.display())
    };
    serde_json::from_str(
        &fs::read_to_string(&name).unwrap_or_else(|error| panic!("reading {name}: {error}")),
    )
    .expect("parse reference")
}

/// Pre-order walk collecting di-spiro nodes, matching the capture's order.
fn collect_di_spiro<'a>(geom: &'a Geometry, out: &mut Vec<&'a Geometry>) {
    match geom {
        Geometry::DiSpiro { .. } => out.push(geom),
        Geometry::Combine(parts) => {
            for part in parts {
                collect_di_spiro(part, out);
            }
        }
        Geometry::Boolean { operands, .. } => {
            for operand in operands {
                collect_di_spiro(operand, out);
            }
        }
        Geometry::Transformed { geom, .. }
        | Geometry::RemoveHoles { geom, .. }
        | Geometry::Simplify { geom, .. }
        | Geometry::Stroke { geom, .. }
        | Geometry::Tagged(geom)
        | Geometry::Radical(geom) => collect_di_spiro(geom, out),
        _ => {}
    }
}

fn path_from_ours(contours: &[Vec<Bez3>]) -> BezPath {
    let mut path = BezPath::new();
    for contour in contours {
        let Some(first) = contour.first() else {
            continue;
        };
        path.move_to(Point::new(first.a.x, first.a.y));
        for arc in contour {
            path.curve_to(
                Point::new(arc.b.x, arc.b.y),
                Point::new(arc.c.x, arc.c.y),
                Point::new(arc.d.x, arc.d.y),
            );
        }
        path.close_path();
    }
    path
}

fn path_from_json(contours: &Value) -> BezPath {
    let mut path = BezPath::new();
    let Some(contours) = contours.as_array() else {
        return path;
    };
    let p = |v: &Value| Point::new(v[0].as_f64().unwrap(), v[1].as_f64().unwrap());
    for contour in contours {
        let arcs = contour.as_array().unwrap();
        let Some(first) = arcs.first() else {
            continue;
        };
        path.move_to(p(&first[0]));
        for arc in arcs {
            path.curve_to(p(&arc[1]), p(&arc[2]), p(&arc[3]));
        }
        path.close_path();
    }
    path
}

fn sample(path: &BezPath) -> Vec<Point> {
    let mut out = Vec::new();
    for segment in path.segments() {
        for index in 0..=SAMPLES_PER_SEGMENT {
            out.push(segment.eval(index as f64 / SAMPLES_PER_SEGMENT as f64));
        }
    }
    out
}

/// Depth by which points of one outline stick out of the other.
fn boundary_drift(samples: &[Point], other: &BezPath) -> f64 {
    use kurbo::Shape;
    let segments: Vec<_> = other.segments().collect();
    let mut worst: f64 = 0.0;
    for p in samples {
        let mut nearest = f64::INFINITY;
        for segment in &segments {
            let distance = segment.nearest(*p, 1e-3).distance_sq.sqrt();
            if distance < nearest {
                nearest = distance;
            }
        }
        if nearest > worst && !other.contains(*p) {
            worst = nearest;
        }
    }
    worst
}

fn shape_drift(ours: &BezPath, theirs: &BezPath) -> f64 {
    boundary_drift(&sample(ours), theirs).max(boundary_drift(&sample(theirs), ours))
}

/// The expanded sides, which everything downstream is built from.
#[test]
fn expansions_match() {
    let Ok(data) = fs::read(dump_path()) else {
        eprintln!("skipping: no capture");
        return;
    };
    let glyphs = lemmata_glyphs::load(&data).expect("decode capture");
    let reference = read_reference("expansions");
    let reference = reference.as_array().expect("array");

    let mut worst = 0.0_f64;
    let mut nodes = 0usize;
    for (glyph, expected) in glyphs.iter().zip(reference) {
        let mut found = Vec::new();
        collect_di_spiro(&glyph.geometry, &mut found);
        let expansions = expected["expansions"].as_array().unwrap();
        assert_eq!(found.len(), expansions.len(), "{}: node count", glyph.name);
        for (node, expected_node) in found.iter().zip(expansions) {
            let Geometry::DiSpiro {
                gizmo,
                contrast,
                closed,
                knots,
            } = node
            else {
                unreachable!()
            };
            let result = lemmata_geom::di_spiro_expand(*gizmo, *contrast, *closed, knots);
            nodes += 1;
            for (side, ours) in [
                ("lhs", &result.lhs_untransformed),
                ("rhs", &result.rhs_untransformed),
            ] {
                let theirs = expected_node[side].as_array().unwrap();
                assert_eq!(
                    ours.len(),
                    theirs.len(),
                    "{}: {side} knot count",
                    glyph.name
                );
                for (knot, expected_knot) in ours.iter().zip(theirs) {
                    let ex = expected_knot[0].as_f64().unwrap();
                    let ey = expected_knot[1].as_f64().unwrap();
                    worst = worst.max((knot.x - ex).abs().max((knot.y - ey).abs()));
                    // Types and importance drive the solver and the run
                    // grouping, so they have to survive the expansion.
                    assert_eq!(
                        format!("{:?}", knot.ty),
                        type_name(expected_knot[2].as_str().unwrap()),
                        "{}: {side} knot type",
                        glyph.name
                    );
                    assert_eq!(
                        knot.unimportant,
                        expected_knot[3].as_u64().unwrap() == 1,
                        "{}: {side} importance",
                        glyph.name
                    );
                }
            }
        }
    }
    eprintln!("expansions: {nodes} nodes, worst knot deviation {worst:.6}");
    assert!(worst <= KNOT_TOLERANCE, "worst knot deviation {worst}");
}

fn type_name(name: &str) -> String {
    match name {
        "open" => "Open".to_string(),
        "open_end" => "OpenEnd".to_string(),
        "left" => "Left".to_string(),
        "right" => "Right".to_string(),
        "g2" => "G2".to_string(),
        "g4" => "G4".to_string(),
        "corner" => "Corner".to_string(),
        other => panic!("unknown knot type {other}"),
    }
}

/// The spiro solve, on the knots the expansion produced.
#[test]
fn raw_solve_arcs_match() {
    let Ok(data) = fs::read(dump_path()) else {
        eprintln!("skipping: no capture");
        return;
    };
    let glyphs = lemmata_glyphs::load(&data).expect("decode capture");
    let reference = read_reference("expansions");
    let reference = reference.as_array().expect("array");

    let mut worst = 0.0_f64;
    let mut nodes = 0usize;
    for (glyph, expected) in glyphs.iter().zip(reference) {
        let mut found = Vec::new();
        collect_di_spiro(&glyph.geometry, &mut found);
        let expansions = expected["expansions"].as_array().unwrap();
        for (node, expected_node) in found.iter().zip(expansions) {
            if expected_node["raw"].is_null() {
                continue;
            }
            let Geometry::DiSpiro {
                gizmo,
                contrast,
                closed,
                knots,
            } = node
            else {
                unreachable!()
            };
            let result = lemmata_geom::di_spiro_expand(*gizmo, *contrast, *closed, knots);
            // The same knot list the capture fed to spiro-js, and the same one
            // `DiSpiroGeometry.toBezArcs` uses: the right side reversed, its
            // left/right types swapped, and for an open skeleton the four ends
            // cornered before the two sides are joined.
            let (lhs, rhs) = (
                result.lhs_untransformed,
                reverse_side(result.rhs_untransformed),
            );
            let lists: Vec<(&str, Vec<MonoKnot>)> = if *closed {
                vec![("lhs", lhs), ("rhs", rhs)]
            } else {
                vec![("open", corner_ends(lhs, rhs))]
            };
            nodes += 1;
            for (key, knots) in lists {
                let plain: Vec<lemmata_geom::Knot> = knots
                    .iter()
                    .map(|k| lemmata_geom::Knot::new(k.x, k.y, k.ty))
                    .collect();
                let ours = lemmata_geom::spiro_arcs(&plain, true);
                let theirs = expected_node["raw"][key].as_array().unwrap();
                assert_eq!(
                    ours.len(),
                    theirs.len(),
                    "{} {key}: raw arc count",
                    glyph.name
                );
                for (arc, expected_arc) in ours.iter().zip(theirs) {
                    let samples = expected_arc.as_array().unwrap();
                    for (index, fraction) in [0.0, 0.25, 0.5, 0.75, 1.0].into_iter().enumerate() {
                        let point = arc.eval(fraction);
                        let sample = samples[index].as_array().unwrap();
                        let ex = sample[0].as_f64().unwrap();
                        let ey = sample[1].as_f64().unwrap();
                        worst = worst.max((point.x - ex).abs().max((point.y - ey).abs()));
                    }
                    let meta = samples[5].as_array().unwrap();
                    worst = worst
                        .max((arc.arc_length - meta[4].as_f64().unwrap()).abs())
                        .max((arc.bend - meta[5].as_f64().unwrap()).abs());
                }
            }
        }
    }
    eprintln!("raw solve: {nodes} nodes, worst sample deviation {worst:.6}");
    assert!(worst <= ARC_TOLERANCE, "worst raw arc deviation {worst}");
}

/// `BiKnot.reverseType` plus a reverse, as the di-spiro path applies to the
/// right side.
fn reverse_side(mut rhs: Vec<MonoKnot>) -> Vec<MonoKnot> {
    for knot in &mut rhs {
        knot.ty = match knot.ty {
            lemmata_geom::KnotType::Left => lemmata_geom::KnotType::Right,
            lemmata_geom::KnotType::Right => lemmata_geom::KnotType::Left,
            other => other,
        };
    }
    rhs.reverse();
    rhs
}

/// `DiSpiroGeometry`'s open case: corner the four ends, then concatenate.
fn corner_ends(mut lhs: Vec<MonoKnot>, mut rhs: Vec<MonoKnot>) -> Vec<MonoKnot> {
    if !lhs.is_empty() {
        lhs[0].ty = lemmata_geom::KnotType::Corner;
        let last = lhs.len() - 1;
        lhs[last].ty = lemmata_geom::KnotType::Corner;
    }
    if !rhs.is_empty() {
        rhs[0].ty = lemmata_geom::KnotType::Corner;
        let last = rhs.len() - 1;
        rhs[last].ty = lemmata_geom::KnotType::Corner;
    }
    lhs.into_iter().chain(rhs).collect()
}

/// The tree's own arcs, before overlap resolution.
#[test]
fn tree_arcs_match() {
    let Ok(data) = fs::read(dump_path()) else {
        eprintln!("skipping: no capture");
        return;
    };
    let glyphs = lemmata_glyphs::load(&data).expect("decode capture");
    let reference = read_reference("tree-arcs");
    let reference = reference.as_array().expect("array");

    let mut worst = (0.0_f64, String::new());
    let mut over = 0usize;
    let mut exact = 0usize;
    for (glyph, expected) in glyphs.iter().zip(reference) {
        let ours = path_from_ours(&glyph.geometry.to_bez_arcs());
        let theirs = path_from_json(&expected["arcs"]);
        let deviation = shape_drift(&ours, &theirs);
        if deviation <= 0.25 {
            exact += 1;
        }
        if deviation > worst.0 {
            worst = (deviation, glyph.name.clone());
        }
        if deviation > 1.0 {
            over += 1;
            eprintln!("{}: tree drift {deviation:.2}", glyph.name);
        }
    }
    eprintln!(
        "tree arcs: {} of {} within 0.25, worst {:.3} on {worst_name}",
        exact,
        glyphs.len(),
        worst.0,
        worst_name = worst.1
    );
    assert!(over < glyphs.len() / 4, "{over} glyphs over 1 unit");
}

/// The arcs after overlap resolution, which is where the two boolean engines
/// part. This is a measurement, not a target.
#[test]
fn resolved_arcs_are_close() {
    let Ok(data) = fs::read(dump_path()) else {
        eprintln!("skipping: no capture");
        return;
    };
    let glyphs = lemmata_glyphs::load(&data).expect("decode capture");
    let reference = read_reference("arcs");
    let reference = reference.as_array().expect("array");

    let mut worst = (0.0_f64, String::new());
    let mut over = 0usize;
    let mut structural = Vec::new();
    for (glyph, expected) in glyphs.iter().zip(reference) {
        let ours_arcs = Geometry::Simplify {
            gizmo: glyph.gizmo,
            geom: Box::new(glyph.geometry.clone()),
        }
        .to_bez_arcs();
        let their_contours = expected["arcs"].as_array().unwrap();
        let our_counts: Vec<usize> = ours_arcs.iter().map(Vec::len).collect();
        let their_counts: Vec<usize> = their_contours
            .iter()
            .map(|contour| contour.as_array().unwrap().len())
            .collect();
        // Which contour comes first is not a structural difference.
        let (mut ours_sorted, mut theirs_sorted) = (our_counts.clone(), their_counts.clone());
        ours_sorted.sort_unstable();
        theirs_sorted.sort_unstable();
        if ours_sorted != theirs_sorted {
            structural.push(format!(
                "{} {:?} vs {:?}",
                glyph.name, our_counts, their_counts
            ));
        }
        let ours = path_from_ours(&ours_arcs);
        let theirs = path_from_json(&expected["arcs"]);
        let deviation = shape_drift(&ours, &theirs);
        if deviation > worst.0 {
            worst = (deviation, glyph.name.clone());
        }
        if deviation > 1.0 {
            over += 1;
            eprintln!("{}: resolved drift {deviation:.2}", glyph.name);
        }
    }
    eprintln!(
        "resolved arcs: {} of {} differ in arc structure",
        structural.len(),
        glyphs.len()
    );
    for entry in structural.iter().take(10) {
        eprintln!("  {entry}");
    }
    eprintln!(
        "resolved arcs: {over} of {} over 1 unit, worst {:.3} on {worst_name}",
        glyphs.len(),
        worst.0,
        worst_name = worst.1
    );
    assert!(over < glyphs.len() / 4, "{over} glyphs over 1 unit");
}

fn point_of(value: &Value) -> Point {
    Point::new(value["x"].as_f64().unwrap(), value["y"].as_f64().unwrap())
}

fn bez3_of(value: &Value) -> Bez3 {
    Bez3::new(
        point_of(&value["a"]),
        point_of(&value["b"]),
        point_of(&value["c"]),
        point_of(&value["d"]),
    )
}

/// Rebuilds a fairized contour from the capture.
fn fair_arcs(contours: &Value) -> Vec<Vec<lemmata_geom::FairArc>> {
    contours
        .as_array()
        .unwrap()
        .iter()
        .map(|contour| {
            contour
                .as_array()
                .unwrap()
                .iter()
                .map(|arc| {
                    if arc["kind"] == "straight" {
                        lemmata_geom::FairArc::Straight {
                            a: point_of(&arc["a"]),
                            b: point_of(&arc["b"]),
                        }
                    } else {
                        lemmata_geom::FairArc::Combined(lemmata_geom::CombinedArc::new(
                            arc["segments"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(bez3_of)
                                .collect(),
                        ))
                    }
                })
                .collect()
        })
        .collect()
}

/// Fairization, compared as a shape.
///
/// The arc bookkeeping is not the point: upstream may split a run into two
/// arcs where we keep one and still produce the same curve. This measures the
/// fairized geometry itself, order-insensitively, so a structural difference
/// that changes nothing does not count.
#[test]
fn fairize_stage_matches() {
    let Ok(data) = fs::read(dump_path()) else {
        eprintln!("skipping: no capture");
        return;
    };
    let glyphs = lemmata_glyphs::load(&data).expect("decode capture");
    let reference = read_reference("fairized");
    let reference = reference.as_array().expect("array");

    let mut worst = (0.0_f64, String::new());
    let mut over = 0usize;
    for (glyph, expected) in glyphs.iter().zip(reference) {
        let arcs = Geometry::Simplify {
            gizmo: glyph.gizmo,
            geom: Box::new(glyph.geometry.clone()),
        }
        .to_bez_arcs();
        let ours = lemmata_geom::fairize(&arcs);
        let ours = path_from_fair(&ours);
        let theirs = path_from_fair_json(&expected["contours"]);
        let deviation = shape_drift(&ours, &theirs);
        if deviation > worst.0 {
            worst = (deviation, glyph.name.clone());
        }
        if deviation > 1.0 {
            over += 1;
            eprintln!("  {}: fairize {deviation:.2}", glyph.name);
        }
    }
    eprintln!(
        "fairize: {over} of {} over 1 unit, worst {:.3} on {}",
        glyphs.len(),
        worst.0,
        worst.1
    );
    assert!(
        over < glyphs.len() / 4,
        "{over} glyphs differ after fairization"
    );
}

/// Samples a fairized contour as a path.
fn path_from_fair(contours: &[Vec<lemmata_geom::FairArc>]) -> BezPath {
    let mut path = BezPath::new();
    for contour in contours {
        let Some(first) = contour.first() else {
            continue;
        };
        path.move_to(first.eval(0.0));
        for arc in contour {
            for step in 1..=SAMPLES_PER_SEGMENT {
                path.line_to(arc.eval(step as f64 / SAMPLES_PER_SEGMENT as f64));
            }
        }
        path.close_path();
    }
    path
}

fn path_from_fair_json(contours: &Value) -> BezPath {
    let mut path = BezPath::new();
    for contour in contours.as_array().unwrap() {
        let arcs = contour.as_array().unwrap();
        let Some(first) = arcs.first() else {
            continue;
        };
        let start = if first["kind"] == "straight" {
            point_of(&first["a"])
        } else {
            point_of(&first["segments"][0]["a"])
        };
        path.move_to(start);
        for arc in arcs {
            if arc["kind"] == "straight" {
                path.line_to(point_of(&arc["b"]));
            } else {
                for segment in arc["segments"].as_array().unwrap() {
                    let segment = bez3_of(segment);
                    for step in 1..=SAMPLES_PER_SEGMENT {
                        path.line_to(segment.eval(step as f64 / SAMPLES_PER_SEGMENT as f64));
                    }
                }
            }
        }
        path.close_path();
    }
    path
}

/// Quadification, on the fairized shape the capture recorded. This isolates it
/// from fairization: if fairize matches and this fails, the fault is here.
#[test]
fn quadify_on_captured_fairize_matches() {
    let Ok(data) = fs::read(dump_path()) else {
        eprintln!("skipping: no capture");
        return;
    };
    let glyphs = lemmata_glyphs::load(&data).expect("decode capture");
    let fairized = read_reference("fairized");
    let fairized = fairized.as_array().expect("array");
    let reference = read_reference("");
    let reference = reference.as_array().expect("array");

    let mut worst = (0.0_f64, String::new());
    let mut over = 0usize;
    let mut point_delta = 0i64;
    for ((glyph, expected_fair), expected) in glyphs.iter().zip(fairized).zip(reference) {
        let ours = lemmata_geom::tree::quadify_fair(&fair_arcs(&expected_fair["contours"]));
        let theirs = expected["contours"].as_array().unwrap();
        assert_eq!(ours.len(), theirs.len(), "{}: contour count", glyph.name);
        let mut glyph_worst = 0.0_f64;
        let mut delta = 0i64;
        for (contour, expected_contour) in ours.iter().zip(theirs) {
            let expected_points = expected_contour.as_array().unwrap();
            delta += contour.len() as i64 - expected_points.len() as i64;
            if contour.len() != expected_points.len() {
                glyph_worst = glyph_worst.max(1.0);
                continue;
            }
            for (point, expected_point) in contour.iter().zip(expected_points) {
                glyph_worst = glyph_worst.max(
                    (point.x - expected_point["x"].as_f64().unwrap())
                        .abs()
                        .max((point.y - expected_point["y"].as_f64().unwrap()).abs()),
                );
            }
        }
        point_delta += delta;
        if glyph_worst > worst.0 {
            worst = (glyph_worst, glyph.name.clone());
        }
        if glyph_worst > 1.0 {
            over += 1;
            eprintln!("{}: quadify {glyph_worst:.2}, {delta:+} points", glyph.name);
        }
    }
    eprintln!(
        "quadify on captured fairize: {over} of {} over 1 unit, worst {:.3} on {}, {point_delta:+} points overall",
        glyphs.len(),
        worst.0,
        worst.1
    );
}
