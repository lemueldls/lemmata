//! The `lemmata` command line tool.
//!
//! For now it builds a font from a glyph capture. The plan's argument parser
//! and the rest of the commands come later.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use lemmata_build::font::{GlyphData, Names};
use lemmata_ptl::param::Shape;

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
  --subset  Unicode ranges to keep, as U+XXXX or U+XXXX-U+YYYY, comma
            separated (default: everything the capture holds)
  --family  family name for the output font (default Lemmata)
  --plan    JSON plan for the font's metrics (default: the reference plan)

  --weight  weight to draw, in hundreds (default: the captured one)
  --width   width to draw, in font units (default: the captured one)
  --slope   slope to draw, in degrees (default: the captured one)
  --serifs  serif style to draw (default: the captured one)
  --spacing spacing to draw (default: the captured one)

Naming any of the shape options asks for outlines nothing in the repository
holds, so upstream's glyph code runs for that shape and the capture is cached
under target/host. It needs the scratch build and IOSEVKA_BUILD_DIR.";

/// A number from a command line argument.
fn parse_number(value: &str, flag: &str) -> Result<f64, String> {
    value
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("{flag} wants a number, not {value:?}"))
}

/// The Unicode ranges a `--subset` argument names.
fn parse_ranges(text: &str) -> Result<Vec<(u32, u32)>, String> {
    let code_point = |text: &str| -> Result<u32, String> {
        let text = text.trim();
        let digits = text
            .strip_prefix("U+")
            .or_else(|| text.strip_prefix("u+"))
            .unwrap_or(text);
        u32::from_str_radix(digits, 16).map_err(|_| format!("{text} is not a code point"))
    };

    text.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.split_once('-').map_or_else(
                || code_point(part).map(|code| (code, code)),
                |(low, high)| Ok((code_point(low)?, code_point(high)?)),
            )
        })
        .collect()
}

fn build(args: &[String]) -> Result<(), String> {
    let mut glyphs_path = PathBuf::from("target/glyphs.bin");
    let mut out_path = PathBuf::from("target/Lemmata-Regular.ttf");
    let mut names = Names::default();
    let mut plan_path = String::new();
    let mut subset: Option<String> = None;
    let mut shape = Shape::default();
    let mut shaped = false;

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
            "--subset" => subset = Some(value.clone()),
            "--weight" => {
                shape.weight = parse_number(value, flag)?;
                shaped = true;
            }
            "--width" => {
                shape.width = parse_number(value, flag)?;
                shaped = true;
            }
            "--slope" => {
                shape.slope_angle = parse_number(value, flag)?;
                shaped = true;
            }
            "--serifs" => {
                shape.serifs = Some(value.clone());
                shaped = true;
            }
            "--spacing" => {
                shape.spacing = Some(value.clone());
                shaped = true;
            }
            other => return Err(format!("unknown option {other:?}\n{USAGE}")),
        }
        index += 2;
    }

    // A shape names outlines the repository does not hold, so upstream's glyph
    // code runs for it. The capture is cached by shape, because a build is tens
    // of seconds and the shape is the whole of its identity.
    if shaped {
        let name = lemmata_host::plan_name(&shape);
        let cached = PathBuf::from("target/host").join(format!("{name}.bin"));
        if !cached.exists() {
            let build_dir = std::env::var("IOSEVKA_BUILD_DIR")
                .unwrap_or_else(|_| "/tmp/opencode/iosevka-build".to_string());
            // The capture carries ASCII, which is what a naming plan asks for and
            // what a build of the whole set would spend minutes on.
            let codepoints: Vec<u32> = (0x20..=0x7E).collect();
            let capture = lemmata_host::capture(&shape, &codepoints, Path::new(&build_dir))?;
            if let Some(parent) = cached.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("creating {}: {error}", parent.display()))?;
            }
            std::fs::write(&cached, capture)
                .map_err(|error| format!("writing {}: {error}", cached.display()))?;
        }
        glyphs_path = cached;
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

    // The restriction runs on the input rather than on the built font, so the
    // MATH table is assembled over the glyphs that are left and its ids stay
    // true. Subsetting a built font would have to renumber them, and the
    // subsetter does not close over MATH.
    let mut glyphs = glyphs;
    if let Some(ranges) = &subset {
        lemmata_build::font::retain_ranges(&mut glyphs, &parse_ranges(ranges)?);
        if glyphs.is_empty() {
            return Err(format!("--subset {ranges} leaves no glyphs"));
        }
    }

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
    let plan = lemmata_plan::from_json(&plan)?;
    let metrics = {
        let mut metrics = plan.to_font_metrics();
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

    let bytes = lemmata_build::font::build(&glyphs, &metrics, &names, &plan.variants)?;
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
