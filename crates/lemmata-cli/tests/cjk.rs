//! Bundles a few Sarasa CN glyphs and checks the 2:1 grid.
//!
//! Skips when the source font is not present. Sarasa is a release download, not
//! a repository file.

use std::path::{Path, PathBuf};

use lemmata_build::font::{GlyphData, Metrics, Names, build};
use lemmata_geom::{QuadifyKind, QuadifyPoint};
use read_fonts::{FontRef, TableProvider};

fn source_path() -> PathBuf {
    std::env::var_os("LEMMATA_CJK_SOURCE").map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("target/cjk/SarasaMonoSC-Regular.ttf")
        },
        PathBuf::from,
    )
}

fn latin_a() -> GlyphData {
    let corner = |x, y| {
        QuadifyPoint {
            kind: QuadifyKind::Corner,
            x,
            y,
        }
    };
    GlyphData {
        name: "a".to_string(),
        unicodes: vec![0x61],
        advance: 600,
        contours: vec![vec![
            corner(80.0, 0.0),
            corner(80.0, 520.0),
            corner(520.0, 520.0),
            corner(520.0, 0.0),
        ]],
    }
}

#[test]
fn sarasa_cn_is_two_cells_wide() {
    let path = source_path();
    let Ok(source) = std::fs::read(&path) else {
        eprintln!("skipping: no CJK source at {}", path.display());
        return;
    };
    let cjk = lemmata_cjk::glyphs(&source, &[(0x4e00, 0x4e0f)], 600.0, 2.0, 1000.0)
        .expect("read the CJK source");
    assert_eq!(cjk.len(), 16, "U+4E00 through U+4E0F");
    for glyph in &cjk {
        assert_eq!(glyph.advance, 1200, "{}", glyph.name);
    }

    let mut glyphs = vec![latin_a()];
    glyphs.extend(cjk);
    let bytes =
        build(&glyphs, &Metrics::default(), &Names::default(), &[]).expect("build the mixed font");
    let font = FontRef::new(&bytes).expect("reopen");
    let cmap = font.cmap().expect("cmap");
    let hmtx = font.hmtx().expect("hmtx");
    let advance = |ch: char| {
        let id = cmap.map_codepoint(u32::from(ch)).expect("mapped");
        hmtx.advance(id).expect("advance")
    };
    assert_eq!(advance('a'), 600);
    assert_eq!(advance('一'), 1200);

    let blob: harfrust::font::Blob = bytes.into();
    let font = harfrust::Font::new(blob, 0).expect("shape");
    let shaper = harfrust::ShaperFont::new(&font);
    let mut buffer = harfrust::Buffer::new();
    buffer.push_str("a一a");
    buffer.guess_segment_properties();
    harfrust::shape(&shaper, &mut buffer, harfrust::ShapeOptions::default()).expect("shape");
    let total: i64 = buffer
        .glyph_positions()
        .iter()
        .map(|position| i64::from(position.x_advance))
        .sum();
    assert_eq!(buffer.glyph_infos().len(), 3);
    assert_eq!(total, 2400, "a cell, two cells, a cell");
}
