//! Measures how far apart the native and the browser build of the same input
//! are, which is what `r[wasm.parity]` is about.
//!
//! Skips unless `NATIVE_FONT` and `BROWSER_FONT` are set, because one of the
//! two has to come out of a browser. The site's download path produces it; copy
//! it somewhere and point `BROWSER_FONT` at it.
//!
//! It asserts rather than reports, so running it is a gate. It fails today:
//! `g` comes out 10.57 font units apart, against the 5 units `r[drift.outline]`
//! is calibrated to.

use std::fs;

use kurbo::Point;
use lemmata_oracle::{flatten, outlines};

fn nearest(p: Point, polyline: &[Point]) -> f64 {
    polyline
        .iter()
        .map(|q| ((p.x - q.x).powi(2) + (p.y - q.y).powi(2)).sqrt())
        .fold(f64::INFINITY, f64::min)
}

fn deviation(a: &[Point], b: &[Point]) -> f64 {
    a.iter().map(|p| nearest(*p, b)).fold(0.0, f64::max)
}

#[test]
fn native_and_browser_agree_visually() {
    let Ok(native_path) = std::env::var("NATIVE_FONT") else {
        eprintln!("skipping: set NATIVE_FONT and BROWSER_FONT");
        return;
    };
    let browser_path = std::env::var("BROWSER_FONT").expect("BROWSER_FONT");
    let native = fs::read(&native_path).expect("read native");
    let browser = fs::read(&browser_path).expect("read browser");

    let ours = outlines(&native).expect("native outlines");
    let theirs = outlines(&browser).expect("browser outlines");
    assert_eq!(ours.len(), theirs.len(), "glyph count");

    let mut worst = (0.0_f64, 0usize);
    let mut differing = 0;
    let mut ranked: Vec<(f64, usize)> = Vec::new();
    for (a, b) in ours.iter().zip(&theirs) {
        let pa: Vec<Point> = a
            .contours
            .iter()
            .flat_map(|c| flatten::polyline(c, 0.05))
            .collect();
        let pb: Vec<Point> = b
            .contours
            .iter()
            .flat_map(|c| flatten::polyline(c, 0.05))
            .collect();
        if pa.len() != pb.len() {
            differing += 1;
        }
        let d = deviation(&pa, &pb).max(deviation(&pb, &pa));
        if d > worst.0 {
            worst = (d, a.glyph_id.to_u32() as usize);
        }
        if d > 0.5 {
            ranked.push((d, a.glyph_id.to_u32() as usize));
        }
    }
    ranked.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
    for (d, gid) in ranked.iter().take(6) {
        eprintln!("   gid {gid}: {d:.4}");
    }
    // The project's own tolerance for two outlines being the same shape.
    const BUDGET: f64 = 5.0;
    assert!(
        worst.0 <= BUDGET,
        "worst deviation {:.4} font units, over the {BUDGET} budget",
        worst.0
    );
    eprintln!(
        "native vs browser: {} glyphs, {} differ in polyline length, worst deviation {:.6} font units (gid {})",
        ours.len(),
        differing,
        worst.0,
        worst.1
    );
}
