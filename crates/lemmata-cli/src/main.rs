//! The `lemmata` command line tool.
//!
//! For now it builds a font from a glyph capture. The plan's argument parser
//! and the rest of the commands come later.

use std::path::PathBuf;
use std::process::ExitCode;

use lemmata_build::font::{GlyphData, Metrics, Names};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("lemmata: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Rounds a captured advance to the units the font table stores.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn advance_to_u16(advance: f64) -> u16 {
    advance.round().clamp(0.0, f64::from(u16::MAX)) as u16
}

fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("build") => build(&args[1..]),
        Some("help" | "--help" | "-h") | None => {
            println!("{USAGE}");
            Ok(())
        }
        Some(other) => Err(format!("unknown command {other:?}\n{USAGE}")),
    }
}

const USAGE: &str = "\
Usage: lemmata build [--glyphs <capture>] [--out <font.ttf>] [--family <name>]

  --glyphs  glyph capture to read (default target/glyphs.bin)
  --out     font to write (default target/Lemmata-Regular.ttf)
  --family  family name for the output font (default Lemmata)";

fn build(args: &[String]) -> Result<(), String> {
    let mut glyphs_path = PathBuf::from("target/glyphs.bin");
    let mut out_path = PathBuf::from("target/Lemmata-Regular.ttf");
    let mut names = Names::default();

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("{flag} needs a value"))?;
        match flag {
            "--glyphs" => glyphs_path = PathBuf::from(value),
            "--out" => out_path = PathBuf::from(value),
            "--family" => names.family.clone_from(value),
            other => return Err(format!("unknown option {other:?}\n{USAGE}")),
        }
        index += 2;
    }

    let data = std::fs::read(&glyphs_path)
        .map_err(|error| format!("reading {}: {error}", glyphs_path.display()))?;
    let captured = lemmata_capture::load(&data)?;

    let glyphs: Vec<GlyphData> = captured
        .iter()
        .map(|glyph| GlyphData {
            name: glyph.name.clone(),
            unicodes: glyph.unicodes.clone(),
            advance: advance_to_u16(glyph.advance),
            contours: lemmata_capture::contours(glyph),
        })
        .collect();

    let bytes = lemmata_build::font::build(&glyphs, &Metrics::default(), &names)?;
    std::fs::write(&out_path, &bytes)
        .map_err(|error| format!("writing {}: {error}", out_path.display()))?;
    println!(
        "{}: {} glyphs, {} bytes",
        out_path.display(),
        glyphs.len() + 1,
        bytes.len()
    );

    Ok(())
}
