//! Extraction checks against reference fonts in `target/reference`.
//!
//! Run with `cargo test -p lemmata-oracle`. The tests skip when the
//! directory is missing, which is the case until a reference build has been
//! copied in. Set `LEMMATA_REFERENCE_DIR` to point somewhere else.

use std::{
    fs,
    path::{Path, PathBuf},
};

use lemmata_oracle::{extract::outlines, flatten::polyline};

fn reference_dir() -> PathBuf {
    std::env::var_os("LEMMATA_REFERENCE_DIR").map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("target/reference")
        },
        PathBuf::from,
    )
}

fn reference_fonts() -> Vec<PathBuf> {
    let dir = reference_dir();
    let Ok(entries) = fs::read_dir(&dir) else {
        eprintln!("skipping: no reference fonts in {}", dir.display());
        return Vec::new();
    };
    let mut fonts: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ttf"))
        .collect();
    fonts.sort();

    fonts
}

#[test]
fn every_reference_font_extracts() {
    for path in reference_fonts() {
        let data = fs::read(&path).expect("read reference font");
        let glyphs = outlines(&data).expect("extract outlines");
        assert!(!glyphs.is_empty(), "{} has no outlines", path.display());
        eprintln!("{}: {} outlines", path.display(), glyphs.len());
    }
}

#[test]
fn extraction_is_deterministic() {
    for path in reference_fonts() {
        let data = fs::read(&path).expect("read reference font");
        let first = outlines(&data).expect("first pass");
        let second = outlines(&data).expect("second pass");
        assert_eq!(first.len(), second.len(), "{}", path.display());

        for (a, b) in first.iter().zip(&second) {
            assert_eq!(a.glyph_id, b.glyph_id);
            let flat_a: Vec<f64> = a
                .contours
                .iter()
                .flat_map(|contour| polyline(contour, 0.1))
                .flat_map(|p| [p.x, p.y])
                .collect();
            let flat_b: Vec<f64> = b
                .contours
                .iter()
                .flat_map(|contour| polyline(contour, 0.1))
                .flat_map(|p| [p.x, p.y])
                .collect();
            assert_eq!(flat_a, flat_b, "{} glyph {}", path.display(), a.glyph_id);
        }
    }
}
