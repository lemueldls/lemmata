//! Writes an SVG of the two outlines rasterized, so the drift can be judged at
//! the size a font is actually used at.
//!
//! Renders each glyph's coverage twice, once from Iosevka's outline and once
//! from ours, and marks the pixels where the coverage differs. Also prints the
//! deviation converted to pixels at common sizes. Skips without a capture.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::suboptimal_flops
)]

use std::fmt::Write as _;
use std::fs;

use kurbo::{BezPath, ParamCurve, Point, Rect, Shape};
use lemmata_geom::{QuadifyKind, QuadifyPoint};
use serde_json::Value;

/// Sub-samples per pixel, per axis.
const SAMPLES: usize = 4;
/// Deviation at or above this pixel coverage difference is marked.
const MARK: f64 = 0.05;

fn workspace_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn dump_path() -> std::path::PathBuf {
    std::env::var_os("LEMMATA_GLYPH_DUMP").map_or_else(
        || workspace_path("target/glyphs.bin"),
        std::path::PathBuf::from,
    )
}

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

fn sample(path: &BezPath) -> Vec<Point> {
    let mut out = Vec::new();
    for segment in path.segments() {
        for index in 0..=16_i32 {
            out.push(segment.eval(f64::from(index) / 16.0));
        }
    }
    out
}

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

/// Coverage of each pixel in a grid over `bounds`, in 0..=1.
fn coverage(path: &BezPath, bounds: Rect, pixels: (usize, usize)) -> Vec<f64> {
    let (width, height) = pixels;
    let mut out = vec![0.0; width * height];
    let step = 1.0 / SAMPLES as f64;
    for row in 0..height {
        for column in 0..width {
            let mut inside = 0usize;
            for sub_row in 0..SAMPLES {
                for sub_column in 0..SAMPLES {
                    let x = bounds.x0
                        + (column as f64 + (sub_column as f64 + 0.5) * step) / width as f64
                            * bounds.width();
                    let y = bounds.y1
                        - (row as f64 + (sub_row as f64 + 0.5) * step) / height as f64
                            * bounds.height();
                    if path.contains(Point::new(x, y)) {
                        inside += 1;
                    }
                }
            }
            out[row * width + column] = inside as f64 / (SAMPLES * SAMPLES) as f64;
        }
    }
    out
}

struct Glyph {
    name: String,
    ours: BezPath,
    theirs: BezPath,
    drift: f64,
}

#[test]
fn write_rendering_svg() {
    let Ok(bytes) = fs::read(dump_path()) else {
        eprintln!("skipping: no capture");
        return;
    };
    let glyphs = lemmata_glyphs::load(&bytes).expect("decode capture");
    let reference: Value = serde_json::from_str(
        &fs::read_to_string(format!("{}.json", dump_path().display())).expect("reference"),
    )
    .expect("parse reference");
    let reference = reference.as_array().expect("array");

    let mut all = Vec::new();
    for (glyph, expected) in glyphs.iter().zip(reference) {
        let mut ours = BezPath::new();
        for contour in lemmata_glyphs::contours(glyph) {
            ours.extend(contour_path(&contour).iter());
        }
        let mut theirs = BezPath::new();
        for contour in expected["contours"].as_array().unwrap() {
            theirs.extend(reference_contour_path(contour).iter());
        }
        let drift =
            boundary_drift(&sample(&ours), &theirs).max(boundary_drift(&sample(&theirs), &ours));
        all.push(Glyph {
            name: glyph.name.clone(),
            ours,
            theirs,
            drift,
        });
    }

    // The deviation in pixels, at sizes a font is used at.
    eprintln!("deviation at common sizes, in pixels:");
    for entry in &all {
        if entry.drift < 0.05 {
            continue;
        }
        let pixels: Vec<String> = [12.0, 16.0, 24.0, 48.0]
            .iter()
            .map(|ppem| format!("{ppem:.0}px {:.3}", entry.drift * ppem / 1000.0))
            .collect();
        eprintln!(
            "  {:<12} {:>5.2} units  {}",
            entry.name,
            entry.drift,
            pixels.join(", ")
        );
    }

    let mut worst: Vec<usize> = (0..all.len()).collect();
    worst.sort_by(|a, b| all[*b].drift.partial_cmp(&all[*a].drift).unwrap());
    worst.truncate(4);

    // Rasterize the four widest at a size where the difference is visible.
    let mut panels = Vec::new();
    for index in worst {
        let entry = &all[index];
        for ppem in [32.0_f64, 128.0] {
            let bounds = entry.ours.bounding_box().union(entry.theirs.bounding_box());
            let scale = ppem / 1000.0;
            let pixels = (
                (bounds.width() * scale).ceil().max(1.0) as usize,
                (bounds.height() * scale).ceil().max(1.0) as usize,
            );
            let our_map = coverage(&entry.ours, bounds, pixels);
            let their_map = coverage(&entry.theirs, bounds, pixels);
            let mut marked = 0usize;
            let mut total = 0.0_f64;
            for (a, b) in our_map.iter().zip(&their_map) {
                let difference = (a - b).abs();
                total += difference;
                if difference >= MARK {
                    marked += 1;
                }
            }
            panels.push(Panel {
                name: entry.name.clone(),
                ppem,
                pixels,
                our_map,
                their_map,
                marked,
                total,
            });
        }
    }

    let svg = render(&panels);
    let out = workspace_path("target/lemmata-rendering.svg");
    fs::write(&out, svg).expect("write svg");
    eprintln!("wrote {}", out.display());
    for panel in &panels {
        eprintln!(
            "  {} at {:.0}px: {} pixels, {:.2}% coverage summed, {} pixels differ by {:.0}% or more",
            panel.name,
            panel.ppem,
            panel.pixels.0 * panel.pixels.1,
            100.0 * panel.total / (panel.pixels.0 * panel.pixels.1) as f64,
            panel.marked,
            100.0 * MARK
        );
    }
}

struct Panel {
    name: String,
    ppem: f64,
    pixels: (usize, usize),
    our_map: Vec<f64>,
    their_map: Vec<f64>,
    marked: usize,
    total: f64,
}

fn render(panels: &[Panel]) -> String {
    let mut svg = String::new();
    let mut y = 60.0_f64;
    let mut width = 0.0_f64;
    for panel in panels {
        width = width.max(panel.pixels.0 as f64 + 40.0);
    }
    let height: f64 = panels
        .iter()
        .map(|panel| panel.pixels.1 as f64 + 44.0)
        .sum::<f64>()
        + 60.0;
    let _ = write!(
        svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0}" height="{height:.0}" viewBox="0 0 {width:.0} {height:.0}">
<rect width="100%" height="100%" fill="#ffffff"/>
<style>
  text {{ font: 12px ui-sans-serif, system-ui, sans-serif; fill: #333; }}
  text.title {{ font-size: 16px; font-weight: 600; fill: #111; }}
</style>
<text class="title" x="16" y="28">The same glyphs rasterized: gray is Iosevka, red is where our coverage differs by 5% or more</text>
<text x="16" y="46">Top: 32 px. Bottom of each pair: 128 px, magnified so the difference is visible.</text>
"##
    );
    for panel in panels {
        let _ = write!(
            svg,
            r#"<text x="16" y="{:.1}">{} at {:.0}px &#8212; {} of {} pixels differ by 5% or more ({:.2}% of the area summed)</text>"#,
            y,
            panel.name,
            panel.ppem,
            panel.marked,
            panel.pixels.0 * panel.pixels.1,
            100.0 * panel.total / (panel.pixels.0 * panel.pixels.1) as f64
        );
        y += 8.0;
        let _ = write!(
            svg,
            r#"<g transform="translate(20,{:.1})">"#,
            y + panel.pixels.1 as f64
        );
        for row in 0..panel.pixels.1 {
            for column in 0..panel.pixels.0 {
                let index = row * panel.pixels.0 + column;
                let theirs = panel.their_map[index];
                let ours = panel.our_map[index];
                if theirs > 0.0 {
                    let level = (255.0 - 190.0 * theirs) as u8;
                    let _ = write!(
                        svg,
                        r#"<rect x="{column}" y="{row}" width="1" height="1" fill="rgb({level},{level},{level})"/>"#,
                    );
                }
                let difference = (ours - theirs).abs();
                if difference >= MARK {
                    let _ = write!(
                        svg,
                        r##"<rect x="{column}" y="{row}" width="1" height="1" fill="#e53935" fill-opacity="{:.2}"/>"##,
                        difference.min(1.0)
                    );
                }
            }
        }
        svg.push_str("</g>");
        y += panel.pixels.1 as f64 + 36.0;
    }
    svg.push_str("</svg>\n");
    svg
}
