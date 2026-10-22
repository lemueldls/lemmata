//! Writes an SVG comparing our outlines with Iosevka's.
//!
//! Each cell fills the reference outline in gray, fills the region where the two
//! disagree in red, and strokes ours in blue. Skips when no capture is present.

#![allow(clippy::cast_precision_loss, clippy::suboptimal_flops)]

use std::{fmt::Write as _, fs};

use kurbo::{BezPath, ParamCurve, PathEl, Point, Shape};
use lemmata_capture::dump_path;

/// A path under the workspace root.
fn workspace_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}
use lemmata_geom::{QuadifyKind, QuadifyPoint};
use linesweeper::BinaryOp;
use serde_json::Value;

const SAMPLES_PER_SEGMENT: usize = 16;

fn contour_path(points: &[QuadifyPoint]) -> BezPath {
    let mut path = BezPath::new();
    let Some(first) = points.first() else {
        return path;
    };
    path.move_to(Point::new(first.x, first.y));
    let mut index = 1;
    while index < points.len() {
        let point = &points[index];
        if point.kind == QuadifyKind::Quadratic && index + 1 < points.len() {
            let end = &points[index + 1];
            path.quad_to(Point::new(point.x, point.y), Point::new(end.x, end.y));
            index += 2;
        } else {
            path.line_to(Point::new(point.x, point.y));
            index += 1;
        }
    }
    path.close_path();

    path
}

fn reference_contour_path(contour: &Value) -> BezPath {
    let mut path = BezPath::new();
    let points = contour.as_array().unwrap();
    if points.is_empty() {
        return path;
    }
    path.move_to(Point::new(
        points[0]["x"].as_f64().unwrap(),
        points[0]["y"].as_f64().unwrap(),
    ));
    let mut index = 1;
    while index < points.len() {
        let point = &points[index];
        if point["quadratic"].as_bool().unwrap() && index + 1 < points.len() {
            let end = &points[index + 1];
            path.quad_to(
                Point::new(point["x"].as_f64().unwrap(), point["y"].as_f64().unwrap()),
                Point::new(end["x"].as_f64().unwrap(), end["y"].as_f64().unwrap()),
            );
            index += 2;
        } else {
            path.line_to(Point::new(
                point["x"].as_f64().unwrap(),
                point["y"].as_f64().unwrap(),
            ));
            index += 1;
        }
    }
    path.close_path();

    path
}

/// SVG path data for a kurbo path.
fn data(path: &BezPath) -> String {
    let mut out = String::new();
    for element in path.elements() {
        match element {
            PathEl::MoveTo(p) => {
                let _ = write!(out, "M{:.2} {:.2}", p.x, p.y);
            }
            PathEl::LineTo(p) => {
                let _ = write!(out, "L{:.2} {:.2}", p.x, p.y);
            }
            PathEl::QuadTo(c, p) => {
                let _ = write!(out, "Q{:.2} {:.2} {:.2} {:.2}", c.x, c.y, p.x, p.y);
            }
            PathEl::CurveTo(c1, c2, p) => {
                let _ = write!(
                    out,
                    "C{:.2} {:.2} {:.2} {:.2} {:.2} {:.2}",
                    c1.x, c1.y, c2.x, c2.y, p.x, p.y
                );
            }
            PathEl::ClosePath => out.push('Z'),
        }
    }

    out
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
    use kurbo::ParamCurveNearest;
    let segments: Vec<_> = other.segments().collect();
    let mut worst: f64 = 0.0;
    for p in samples {
        let mut nearest = f64::INFINITY;
        for segment in &segments {
            nearest = nearest.min(segment.nearest(*p, 1e-3).distance_sq.sqrt());
        }

        if nearest > worst && !other.contains(*p) {
            worst = nearest;
        }
    }

    worst
}

struct Compared {
    name: String,
    reference: BezPath,
    ours: BezPath,
    disagreement: BezPath,
    drift: f64,
}

#[test]
fn write_difference_svg() {
    let Ok(bytes) = fs::read(dump_path()) else {
        eprintln!("skipping: no capture");
        return;
    };
    let glyphs = lemmata_capture::load(&bytes).expect("decode capture");
    let reference: Value = serde_json::from_str(
        &fs::read_to_string(format!("{}.json", dump_path().display())).expect("reference"),
    )
    .expect("parse reference");
    let reference = reference.as_array().expect("array");

    let mut compared = Vec::new();
    for (glyph, expected) in glyphs.iter().zip(reference) {
        if lemmata_capture::UNREPLAYABLE.contains(&glyph.name.as_str()) {
            continue;
        }
        let mut ours = BezPath::new();
        for contour in lemmata_capture::contours(glyph) {
            ours.extend(contour_path(&contour).iter());
        }
        let mut theirs = BezPath::new();
        for contour in expected["contours"].as_array().unwrap() {
            theirs.extend(reference_contour_path(contour).iter());
        }
        let drift =
            boundary_drift(&sample(&ours), &theirs).max(boundary_drift(&sample(&theirs), &ours));
        // The region the two disagree on, as one path.
        let mut disagreement = BezPath::new();
        if let Ok(regions) = lemmata_geom::boolean::boolean(&ours, &theirs, BinaryOp::Xor) {
            for region in regions {
                disagreement.extend(region.iter());
            }
        }
        compared.push(Compared {
            name: glyph.name.clone(),
            reference: theirs,
            ours,
            disagreement,
            drift,
        });
    }

    let svg = render(&compared);
    let out = workspace_path("target/lemmata-outlines.svg");
    fs::write(&out, svg).expect("write svg");
    let worst: Vec<String> = compared
        .iter()
        .filter(|entry| entry.drift > 0.05)
        .map(|entry| format!("{} {:.2}", entry.name, entry.drift))
        .collect();
    eprintln!("wrote {}", out.display());
    eprintln!("over 0.05 units: {}", worst.join(", "));
}

/// The grid, plus a magnified row for the worst offenders.
fn render(compared: &[Compared]) -> String {
    const COLUMNS: usize = 10;
    const CELL_W: f64 = 132.0;
    const CELL_H: f64 = 176.0;
    const SCALE: f64 = 0.13;
    const MARGIN: f64 = 14.0;

    let rows = compared.len().div_ceil(COLUMNS);
    let mut worst: Vec<usize> = (0..compared.len()).collect();
    worst.sort_by(|a, b| compared[*b].drift.partial_cmp(&compared[*a].drift).unwrap());
    worst.truncate(4);

    let grid_h = rows as f64 * CELL_H;
    let detail_h = 3.0 * CELL_H;
    let width = COLUMNS as f64 * CELL_W + MARGIN * 2.0;
    let height = MARGIN * 3.0 + 60.0 + grid_h + 40.0 + detail_h;

    let mut svg = String::new();
    let _ = write!(
        svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0}" height="{height:.0}" viewBox="0 0 {width:.0} {height:.0}">
<rect width="100%" height="100%" fill="#ffffff"/>
<style>
  text {{ font: 11px ui-sans-serif, system-ui, sans-serif; fill: #444; }}
  text.title {{ font-size: 17px; font-weight: 600; fill: #111; }}
  text.note {{ font-size: 12px; fill: #555; }}
  .reference {{ fill: #d8d8d8; stroke: #b0b0b0; stroke-width: 0.4; }}
  .ours {{ fill: none; stroke: #1565c0; stroke-width: 0.5; }}
  .disagreement {{ fill: #e53935; fill-opacity: 0.85; }}
  .cell {{ fill: #fbfbfb; stroke: #eeeeee; }}
</style>
"##
    );
    let _ = write!(
        svg,
        r#"<text class="title" x="{MARGIN}" y="30">Lemmata outlines against Iosevka v34.9.0 (RefA Regular)</text>
<text class="note" x="{MARGIN}" y="48">Gray: Iosevka. Blue: ours. Red: the region where they disagree, which is the drift.</text>
"#
    );

    let top = MARGIN * 2.0 + 52.0;
    for (index, entry) in compared.iter().enumerate() {
        let column = index % COLUMNS;
        let row = index / COLUMNS;
        let x = MARGIN + column as f64 * CELL_W;
        let y = top + row as f64 * CELL_H;
        let _ = write!(
            svg,
            r#"<rect class="cell" x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="6"/>"#,
            x + 2.0,
            y + 2.0,
            CELL_W - 4.0,
            CELL_H - 4.0
        );
        // Center the glyph in the cell: font units are 1000 tall, y up.
        let _ = write!(
            svg,
            r#"<g transform="translate({:.1},{:.1}) scale({SCALE},-{SCALE})">"#,
            x + CELL_W / 2.0 - 250.0 * SCALE,
            y + CELL_H - 26.0
        );
        let _ = write!(
            svg,
            r#"<path class="reference" d="{}"/>"#,
            data(&entry.reference)
        );
        let _ = write!(svg, r#"<path class="ours" d="{}"/>"#, data(&entry.ours));
        let _ = write!(
            svg,
            r#"<path class="disagreement" d="{}"/>"#,
            data(&entry.disagreement)
        );
        svg.push_str("</g>");
        let label = if entry.drift > 0.05 {
            format!("{}  {:.2}", entry.name, entry.drift)
        } else {
            entry.name.clone()
        };
        let _ = write!(
            svg,
            r#"<text x="{:.1}" y="{:.1}" text-anchor="middle">{label}</text>"#,
            x + CELL_W / 2.0,
            y + CELL_H - 8.0
        );
    }

    let detail_top = top + grid_h + 46.0;
    let _ = write!(
        svg,
        r#"<text class="title" x="{MARGIN}" y="{:.1}">The four widest, magnified</text>"#,
        detail_top - 14.0
    );
    for (slot, index) in worst.iter().enumerate() {
        let entry = &compared[*index];
        let y = detail_top + slot as f64 * CELL_H;
        let _ = write!(
            svg,
            r#"<text x="{MARGIN}" y="{:.1}">{name}  drift {drift:.2} units</text>"#,
            y + 14.0,
            name = entry.name,
            drift = entry.drift
        );
        let _ = write!(
            svg,
            r#"<g transform="translate({:.1},{:.1}) scale(0.42,-0.42)">"#,
            MARGIN + 240.0,
            y + CELL_H - 8.0
        );
        let _ = write!(
            svg,
            r#"<path class="reference" d="{}"/>"#,
            data(&entry.reference)
        );
        let _ = write!(svg, r#"<path class="ours" d="{}"/>"#, data(&entry.ours));
        let _ = write!(
            svg,
            r#"<path class="disagreement" d="{}"/>"#,
            data(&entry.disagreement)
        );
        svg.push_str("</g>");
    }

    svg.push_str("</svg>\n");

    svg
}
