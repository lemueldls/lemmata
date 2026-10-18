//! The `lemmata` command line tool.
//!
//! For now it builds a font from a glyph capture. The plan's argument parser
//! and the rest of the commands come later.

use std::path::PathBuf;
use std::process::ExitCode;

use lemmata_build::font::{GlyphData, Names};

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
  --out     font to write; a .woff2 extension compresses it (default
            target/Lemmata-Regular.ttf)
  --family  family name for the output font (default Lemmata)
  --plan    JSON plan for the font's metrics (default: the reference plan)";

fn build(args: &[String]) -> Result<(), String> {
    let mut glyphs_path = PathBuf::from("target/glyphs.bin");
    let mut out_path = PathBuf::from("target/Lemmata-Regular.ttf");
    let mut names = Names::default();
    let mut plan_path = String::new();

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
            "--plan" => plan_path.clone_from(value),
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

    let plan = if plan_path.is_empty() {
        String::new()
    } else {
        std::fs::read_to_string(&plan_path)
            .map_err(|error| format!("reading {plan_path}: {error}"))?
    };

    // The capture records the slant in each glyph's gizmo, so the face's angle
    // comes from the source rather than from a flag.
    let coefficients = captured
        .first()
        .map_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0], |glyph| {
            glyph.gizmo.as_coeffs()
        });
    // Through `lemmata_geom::math` for the same reason the rest of the pipeline
    // is: the host and wasm32 have different `atan2` in the last bit, and this
    // value reaches the `post` table.
    let italic_angle = lemmata_geom::math::atan2(coefficients[2], coefficients[0]).to_degrees();
    // The plan supplies the font's metrics. The capture records its slant in
    // each glyph's gizmo, so the angle is taken from the source and overwrites
    // whatever the plan said; once the glyph source responds to a plan, the plan
    // is the one that decides.
    let metrics = {
        let mut metrics = lemmata_plan::from_json(&plan)?.to_font_metrics();
        metrics.italic_angle = italic_angle;
        metrics
    };
    // The style is one thing, so the subfamily follows the angle rather than a
    // flag that could disagree with it.
    let names = Names {
        subfamily: if italic_angle == 0.0 {
            "Regular".to_string()
        } else {
            "Italic".to_string()
        },
        ..names
    };

    let bytes = lemmata_build::font::build(&glyphs, &metrics, &names)?;
    // The extension decides the container. A `.woff2` is the same font
    // compressed, which is what a page serves and what a subsetter reads.
    let compressed = out_path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("woff2"));
    let bytes = if compressed {
        lemmata_build::font::woff2(&bytes)?
    } else {
        bytes
    };
    std::fs::write(&out_path, &bytes)
        .map_err(|error| format!("writing {}: {error}", out_path.display()))?;
    // The font carries `.notdef`, the captured glyphs, and the derived ones:
    // the size variants of the stretchy glyphs and their assembly pieces.
    // Reporting only the captured count was misleading enough to send me
    // looking for the wrong thing once.
    let total = glyphs.len() + 1 + lemmata_build::font::derived(&glyphs, &metrics).len();
    println!(
        "{}: {} glyphs, {} bytes, {}",
        out_path.display(),
        total,
        bytes.len(),
        names.subfamily.to_lowercase()
    );

    Ok(())
}
