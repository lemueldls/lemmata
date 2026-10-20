//! The `lemmata` command line tool.
//!
//! The arguments, `--help`, `--version` and shell completions come from
//! `figue`, so a command is a struct and a flag is a field. `build` is the only
//! command so far.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use facet::Facet;
use figue as args;
use lemmata_build::font::{GlyphData, Names};
use lemmata_ptl::param::Shape;

/// The command line.
#[derive(Debug, Facet)]
struct Cli {
    /// Standard `--help`, `--version` and shell completion flags.
    #[facet(flatten)]
    builtins: args::FigueBuiltins,

    /// The command to run.
    #[facet(args::subcommand)]
    command: Command,
}

/// What the tool can do.
#[derive(Debug, Facet)]
#[repr(u8)]
enum Command {
    /// Build a font from a glyph capture and a plan.
    Build(Build),
}

/// Build a font.
///
/// A shape option asks upstream's glyph code to draw a shape the repository
/// does not hold, so it needs `IOSEVKA_BUILD_DIR` pointing at a scratch copy of
/// `vendor/Iosevka` with `npm install` run in it.
#[derive(Debug, Facet)]
struct Build {
    /// Glyph capture to read.
    #[facet(args::named, default = "target/glyphs.bin")]
    glyphs: String,

    /// Font to write; a `.woff2` extension compresses it.
    #[facet(args::named, default = "target/Lemmata-Regular.ttf")]
    out: String,

    /// Family name for the output font.
    #[facet(args::named, default = "Lemmata")]
    family: String,

    /// JSON plan for the font's metrics.
    #[facet(args::named, default)]
    plan: Option<String>,

    /// Unicode ranges to keep, as `U+XXXX` or `U+XXXX-U+YYYY`, comma separated.
    #[facet(args::named, default)]
    subset: Option<String>,

    /// Weight to draw, in hundreds. Names a shape upstream builds.
    #[facet(args::named, default)]
    weight: Option<f64>,

    /// Width to draw, in font units.
    #[facet(args::named, default)]
    width: Option<f64>,

    /// Slope to draw, in degrees.
    #[facet(args::named, default)]
    slope: Option<f64>,

    /// Serif style to draw.
    #[facet(args::named, default)]
    serifs: Option<String>,

    /// Spacing to draw.
    #[facet(args::named, default)]
    spacing: Option<String>,
}

fn main() -> ExitCode {
    // `unwrap` prints help, version or completions and exits for those, and
    // prints a diagnostic and exits non-zero for a parse error.
    let config = figue::builder::<Cli>()
        .expect("the argument schema builds")
        .cli(|cli| cli.args(std::env::args().skip(1)))
        .help(|help| {
            help.program_name("lemmata")
                .version(env!("CARGO_PKG_VERSION"))
                .description("A programmatic typeface factory.")
        })
        .build();
    let cli = figue::Driver::new(config).run().unwrap();

    run(cli)
}

fn run(cli: Cli) -> ExitCode {
    match cli.command {
        Command::Build(build) => match build_font(build) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("lemmata: {message}");
                ExitCode::FAILURE
            }
        },
    }
}

/// Rounds a captured advance to the units the font table stores.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn advance_to_u16(advance: f64) -> u16 {
    advance.round().clamp(0.0, f64::from(u16::MAX)) as u16
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

fn build_font(build: Build) -> Result<(), String> {
    let Build {
        glyphs,
        out,
        family,
        plan,
        subset,
        weight,
        width,
        slope,
        serifs,
        spacing,
    } = build;

    let mut names = Names {
        family,
        ..Names::default()
    };
    let mut glyphs_path = PathBuf::from(glyphs);
    let out_path = PathBuf::from(out);
    let mut plan_path = plan.unwrap_or_default();

    // A shape names outlines the repository does not hold, so upstream's glyph
    // code runs for it. The capture is cached by shape, because a build is tens
    // of seconds and the shape is the whole of its identity.
    let shaped = weight.is_some()
        || width.is_some()
        || slope.is_some()
        || serifs.is_some()
        || spacing.is_some();
    let mut shape = Shape::default();
    if let Some(value) = weight {
        shape.weight = value;
    }
    if let Some(value) = width {
        shape.width = value;
    }
    if let Some(value) = slope {
        shape.slope_angle = value;
    }
    if let Some(value) = serifs {
        shape.serifs = Some(value);
    }
    if let Some(value) = spacing {
        shape.spacing = Some(value);
    }
    if shaped {
        let name = lemmata_host::plan_name(&shape);
        let cached = PathBuf::from("target/host").join(format!("{name}.bin"));
        if !cached.exists() {
            let build_dir = std::env::var("IOSEVKA_BUILD_DIR").map_err(|_| {
                "--weight, --width, --slope, --serifs and --spacing need \
                 IOSEVKA_BUILD_DIR, a scratch copy of vendor/Iosevka with \
                 `npm install` run in it"
                    .to_string()
            })?;
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

    if !plan_path.is_empty() {
        plan_path = std::fs::read_to_string(&plan_path)
            .map_err(|error| format!("reading {plan_path}: {error}"))?;
    }

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
    // whatever the plan said.
    let plan = lemmata_plan::from_json(&plan_path)?;
    let metrics = {
        let mut metrics = plan.to_font_metrics();
        metrics.italic_angle = italic_angle;
        metrics
    };
    // The style is one thing, so the subfamily follows the angle rather than a
    // flag that could disagree with it.
    names.subfamily = if italic_angle == 0.0 {
        "Regular".to_string()
    } else {
        "Italic".to_string()
    };

    // The ligature rules and the glyphs they draw are part of the font source,
    // so both front ends add them. A rule naming a glyph the subset removed is
    // dropped by the build.
    let ligatures = lemmata_build::ligatures::add(&mut glyphs);
    let bytes =
        lemmata_build::font::build_with(&glyphs, &metrics, &names, &plan.variants, ligatures)?;
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
