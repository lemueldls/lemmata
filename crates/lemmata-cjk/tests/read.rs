//! Reads a small TrueType source and checks the 2:1 scale.

use lemmata_build::font::{GlyphData, Metrics, Names, build};
use lemmata_geom::{QuadifyKind, QuadifyPoint};

fn rectangle(width: f64, height: f64) -> Vec<Vec<QuadifyPoint>> {
    let corner = |x, y| {
        QuadifyPoint {
            kind: QuadifyKind::Corner,
            x,
            y,
        }
    };
    vec![vec![
        corner(0.0, 0.0),
        corner(0.0, height),
        corner(width, height),
        corner(width, 0.0),
    ]]
}

#[test]
fn a_source_at_two_cells_scales_to_the_latin_cell() {
    // r[verify cjk.width]
    // r[verify cjk.source]
    // r[verify cjk.ranges]
    // A source with one 1000-unit ideograph, the shape Sarasa's CJK glyphs
    // have before the merge.
    let source_glyphs = vec![GlyphData {
        name: "source/one".to_string(),
        unicodes: vec![0x4e00],
        advance: 1000,
        contours: rectangle(500.0, 700.0),
    }];
    let source = build(&source_glyphs, &Metrics::default(), &Names::default(), &[])
        .expect("build the source");

    let glyphs = lemmata_cjk::glyphs(&source, &[(0x4e00, 0x4e00)], 600.0, 2.0, 1000.0)
        .expect("read the source");
    assert_eq!(glyphs.len(), 1);
    assert_eq!(glyphs[0].advance, 1200, "a full-width glyph is two cells");
    assert_eq!(glyphs[0].unicodes, vec![0x4e00]);
    let right = glyphs[0]
        .contours
        .iter()
        .flatten()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!((right - 600.0).abs() < 1e-9, "ink scaled to 1.2: {right}");
}

#[test]
fn a_range_with_no_mapped_glyph_is_empty() {
    let source_glyphs = vec![GlyphData {
        name: "source/one".to_string(),
        unicodes: vec![0x4e00],
        advance: 1000,
        contours: rectangle(500.0, 700.0),
    }];
    let source = build(&source_glyphs, &Metrics::default(), &Names::default(), &[])
        .expect("build the source");

    let glyphs = lemmata_cjk::glyphs(&source, &[(0x4e01, 0x4e02)], 600.0, 2.0, 1000.0)
        .expect("read the source");
    assert!(glyphs.is_empty());
}
