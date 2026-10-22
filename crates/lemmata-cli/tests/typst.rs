//! The Typst harness: `r[math.typst]`.
//!
//! Compiles `corpus/equations.typ` against the built font and fails when a
//! renderer falls back to a font of its own, which is what a missing glyph or a
//! missing table looks like from the outside.
//!
//! Skips when Typst is not installed, like the tests that skip without a
//! capture. The two details that make it work are the font path, and setting
//! the font on `math.equation` rather than on `text`, because a text font does
//! not reach math mode.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use lemmata_build::font::{GlyphData, Metrics, Names};
use lemmata_capture::Glyph;

fn workspace_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// Rounds a captured advance to the units the font table stores.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn advance_to_u16(advance: f64) -> u16 {
    advance.round().clamp(0.0, f64::from(u16::MAX)) as u16
}

fn glyph_data(captured: &[Glyph]) -> Vec<GlyphData> {
    captured
        .iter()
        .map(|glyph| {
            GlyphData {
                name: glyph.name.clone(),
                unicodes: glyph.unicodes.clone(),
                advance: advance_to_u16(glyph.advance),
                contours: lemmata_capture::contours(glyph),
            }
        })
        .collect()
}

/// Builds one face from a capture, into `directory`.
fn build_face(capture: &Path, directory: &Path, name: &str) -> Option<()> {
    let data = std::fs::read(capture).ok()?;
    let captured = lemmata_capture::load(&data).expect("decode the capture");
    let coefficients = captured
        .first()
        .map_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0], |glyph| {
            glyph.gizmo.as_coeffs()
        });
    let italic_angle = coefficients[2].atan2(coefficients[0]).to_degrees();
    let metrics = Metrics {
        italic_angle,
        ..Metrics::default()
    };
    let names = Names {
        subfamily: if italic_angle == 0.0 {
            "Regular".to_string()
        } else {
            "Italic".to_string()
        },
        ..Names::default()
    };
    let bytes = lemmata_build::font::build(&glyph_data(&captured), &metrics, &names, &[])
        .expect("build the face");
    std::fs::write(directory.join(name), bytes).expect("write the face");

    Some(())
}

/// The fonts a PDF embeds, by name.
fn embedded_fonts(pdf: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(pdf);
    let mut names = Vec::new();
    for part in text.split("/BaseFont").skip(1) {
        let name = part
            .trim_start_matches(|c: char| c.is_whitespace() || c == '/')
            .split(|c: char| c.is_whitespace() || c == '/')
            .next()
            .unwrap_or_default();
        // The subset prefix is six letters and a plus.
        let name = name.split_once('+').map_or(name, |(_, rest)| rest);
        if !name.is_empty() && !names.contains(&name.to_string()) {
            names.push(name.to_string());
        }
    }
    names
}

#[test]
fn equations_render_without_falling_back() {
    if Command::new("typst").arg("--version").output().is_err() {
        eprintln!("skipping: typst is not installed");
        return;
    }

    let directory = workspace_path("target/typst-harness");
    std::fs::create_dir_all(&directory).expect("create the harness directory");
    if build_face(
        &workspace_path("target/math.bin"),
        &directory,
        "Lemmata-Regular.ttf",
    )
    .is_none()
    {
        eprintln!("skipping: no math capture");
        return;
    }
    build_face(
        &workspace_path("target/math-italic.bin"),
        &directory,
        "Lemmata-Italic.ttf",
    );

    // A cache of its own, so a stale font list cannot decide the result.
    let cache = directory.join("cache");
    let output = Command::new("typst")
        .arg("compile")
        .arg("--font-path")
        .arg(&directory)
        .arg(workspace_path("corpus/equations.typ"))
        .arg(directory.join("equations.pdf"))
        .env("XDG_CACHE_HOME", &cache)
        .output()
        .expect("run typst");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "typst failed:\n{stderr}");
    assert!(
        !stderr.contains("unknown font family"),
        "typst did not find the family:\n{stderr}"
    );

    let pdf = std::fs::read(directory.join("equations.pdf")).expect("read the PDF");
    let fonts = embedded_fonts(&pdf);
    assert!(!fonts.is_empty(), "the PDF embeds no font at all");
    for font in &fonts {
        assert!(
            font.starts_with("Lemmata"),
            "the PDF fell back to {font}, so the corpus has a glyph or a table we do not serve"
        );
    }
    // Every equation in the corpus, so a corpus that stops being read is
    // visible in the count.
    let equations = std::fs::read_to_string(workspace_path("corpus/equations.typ"))
        .expect("read the corpus")
        .lines()
        .filter(|line| line.starts_with("$ ") && line.ends_with(" $"))
        .count();
    assert!(
        equations >= 8,
        "the corpus has shrunk to {equations} equations"
    );
    eprintln!("typst: {equations} equations, fonts {fonts:?}");
}
