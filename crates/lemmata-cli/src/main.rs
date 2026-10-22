//! The `lemmata` command line tool.
//!
//! The arguments, `--help`, `--version` and shell completions come from
//! `figue`, so a command is a struct and a flag is a field. `build` builds one
//! face, or every face a family document names when `--family` is set.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use facet::Facet;
use figue as args;
use lemmata_build::font::{self, Names};
use lemmata_capture::Glyph;
use lemmata_plan::{CoveragePolicy, FamilyDocument, OutputFormat, Plan, Spacing};
use lemmata_ptl::param::Shape;
use lemmata_source::Coverage;
use serde::Serialize;

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
    /// Build one face, or every face a family document names.
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

    /// Font to write, or the output directory in family mode.
    #[facet(args::named, default)]
    out: Option<String>,

    /// Family name for a single output font.
    #[facet(args::named, default = "Lemmata")]
    name: String,

    /// Family document to build instead of one face.
    #[facet(args::named, default)]
    family: Option<String>,

    /// Comma-separated instance ids to build in family mode.
    #[facet(args::named, default)]
    only: Option<String>,

    /// JSON plan for the font's metrics.
    #[facet(args::named, default)]
    plan: Option<String>,

    /// Unicode ranges to keep, as `U+XXXX` or `U+XXXX-U+YYYY`, comma separated.
    #[facet(args::named, default)]
    subset: Option<String>,

    /// Unicode ranges the host capture should hold. Defaults to ASCII.
    #[facet(args::named, default)]
    codepoints: Option<String>,

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
    let result = match cli.command {
        Command::Build(build) => build_font(build),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("lemmata: {message}");
            ExitCode::FAILURE
        }
    }
}

/// The Unicode ranges an argument names.
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

/// Every code point in a set of ranges, sorted and deduplicated.
fn expand_ranges(ranges: &[(u32, u32)]) -> Result<Vec<u32>, String> {
    let mut codes = Vec::new();
    for (low, high) in ranges {
        if low > high {
            return Err(format!("U+{low:04X}-U+{high:04X} is reversed"));
        }
        codes.extend(*low..=*high);
    }
    codes.sort_unstable();
    codes.dedup();

    Ok(codes)
}

fn build_font(build: Build) -> Result<(), String> {
    let Build {
        glyphs,
        out,
        name,
        family,
        only,
        plan,
        subset,
        codepoints,
        weight,
        width,
        slope,
        serifs,
        spacing,
    } = build;

    if let Some(family) = family {
        let single_face_option = name != "Lemmata"
            || glyphs != "target/glyphs.bin"
            || plan.is_some()
            || codepoints.is_some()
            || weight.is_some()
            || width.is_some()
            || slope.is_some()
            || serifs.is_some()
            || spacing.is_some();
        if single_face_option {
            return Err(
                "face options apply to one face; put them in the family document".to_string(),
            );
        }
        return build_family(&family, out.as_deref(), only.as_deref(), subset.as_deref());
    }
    if only.is_some() {
        return Err("--only applies to a family document".to_string());
    }
    let out = out.unwrap_or_else(|| "target/Lemmata-Regular.ttf".to_string());

    let mut plan = read_plan(plan.as_deref())?;
    let shaped = weight.is_some()
        || width.is_some()
        || slope.is_some()
        || serifs.is_some()
        || spacing.is_some();
    if let Some(weight) = weight {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let weight = weight.round().clamp(0.0, f64::from(u16::MAX)) as u16;
        plan.weight_class = weight;
    }
    if let Some(width) = width {
        plan.width = width;
        plan.width_class = lemmata_plan::width_class(width);
    }
    if let Some(slope) = slope {
        plan.italic_angle = slope;
    }
    if let Some(serifs) = serifs {
        plan.serifs = serifs;
    }
    if let Some(spacing) = spacing {
        plan.spacing = Spacing::from_name(&spacing)?;
    }

    let glyphs_path = if shaped {
        let shape = lemmata_source::shape_of(&plan);
        let codes = match codepoints {
            Some(text) => expand_ranges(&parse_ranges(&text)?)?,
            None => (0x20..=0x7E).collect(),
        };
        host_capture(&shape, &codes)?
    } else {
        PathBuf::from(glyphs)
    };

    let data = fs::read(&glyphs_path)
        .map_err(|error| format!("reading {}: {error}", glyphs_path.display()))?;
    let captured = lemmata_capture::load(&data)?;
    // The capture records the slant in each glyph's gizmo. A supplied capture
    // is the truth about the shape, so the plan follows it.
    plan.italic_angle = capture_angle(&captured, plan.italic_angle);
    let names = single_names(&name, &plan);
    let result = build_face(&plan, &data, &names, subset.as_deref())?;

    let out_path = PathBuf::from(&out);
    let compressed = out_path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("woff2"));
    let bytes = if compressed {
        font::woff2(&result.bytes)?
    } else {
        result.bytes
    };
    write_file(&out_path, &bytes)?;
    println!(
        "{}: {} glyphs, {} bytes, {} ({} ported, {} captured)",
        out_path.display(),
        result.glyphs,
        bytes.len(),
        names.subfamily.to_lowercase(),
        result.coverage.ported,
        result.coverage.captured
    );

    Ok(())
}

/// The result of building one face, before it is put in a container.
struct FaceResult {
    bytes: Vec<u8>,
    glyphs: usize,
    coverage: Coverage,
}

fn build_face(
    plan: &Plan,
    capture: &[u8],
    names: &Names,
    subset: Option<&str>,
) -> Result<FaceResult, String> {
    let captured = lemmata_capture::load(capture)?;
    let (mut glyphs, coverage) = lemmata_source::glyphs_reported(plan, &captured);
    glyphs.extend(lemmata_source::ligature_glyphs(plan));

    // The restriction runs on the input rather than on the built font, so the
    // MATH table is assembled over the glyphs that are left and its ids stay
    // true. Subsetting a built font would have to renumber them, and the
    // subsetter does not close over MATH.
    if let Some(ranges) = subset {
        font::retain_ranges(&mut glyphs, &parse_ranges(ranges)?);
        if glyphs.is_empty() {
            return Err(format!("--subset {ranges} leaves no glyphs"));
        }
    }

    let metrics = plan.to_font_metrics();
    let ligatures = lemmata_build::ligatures::add(&mut glyphs, plan.width);
    let drawn = lemmata_source::texture_forms(plan, &captured);
    let bytes = font::build_all(&glyphs, &metrics, names, &plan.variants, ligatures, &drawn)?;
    let total = glyphs.len() + 1 + font::derived(&glyphs, &metrics).len();

    Ok(FaceResult {
        bytes,
        glyphs: total,
        coverage,
    })
}

/// The names one face gets from its weight and slope.
fn single_names(family: &str, plan: &Plan) -> Names {
    let subfamily = plan.style_subfamily();

    Names {
        family: family.to_string(),
        subfamily: subfamily.clone(),
        typographic_family: Some(family.to_string()),
        typographic_subfamily: Some(subfamily.clone()),
        postscript_name: Some(format!(
            "{}-{}",
            family.replace(' ', ""),
            subfamily.replace(' ', "")
        )),
        ..Names::default()
    }
}

fn read_plan(path: Option<&str>) -> Result<Plan, String> {
    let Some(path) = path else {
        return Ok(Plan::default());
    };
    let text = fs::read_to_string(path).map_err(|error| format!("reading {path}: {error}"))?;

    lemmata_plan::from_json(&text)
}

fn capture_angle(captured: &[Glyph], fallback: f64) -> f64 {
    let Some(glyph) = captured.first() else {
        return fallback;
    };
    let coefficients = glyph.gizmo.as_coeffs();

    lemmata_geom::math::atan2(coefficients[2], coefficients[0]).to_degrees()
}

fn host_capture(shape: &Shape, codepoints: &[u32]) -> Result<PathBuf, String> {
    let mut codes = codepoints.to_vec();
    codes.sort_unstable();
    codes.dedup();
    let key = lemmata_host::capture_key(shape, &codes);
    let cached = PathBuf::from("target/host").join(format!("{key}.bin"));
    if cached.exists() {
        if let Some(recorded) = lemmata_host::read_shape(&cached) {
            if !lemmata_host::shape_matches(&recorded, shape, &codes) {
                return Err(format!(
                    "{} records a different shape; remove it and build again",
                    cached.display()
                ));
            }
        } else {
            lemmata_host::write_shape(&cached, shape, &codes)?;
        }
        return Ok(cached);
    }

    let build_dir = std::env::var("IOSEVKA_BUILD_DIR").map_err(|_| {
        "this shape needs IOSEVKA_BUILD_DIR, a scratch copy of vendor/Iosevka \
         with `npm install` run in it"
            .to_string()
    })?;
    let data = lemmata_host::capture(shape, &codes, Path::new(&build_dir))?;
    if let Some(parent) = cached.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("creating {}: {error}", parent.display()))?;
    }
    fs::write(&cached, &data).map_err(|error| format!("writing {}: {error}", cached.display()))?;
    lemmata_host::write_shape(&cached, shape, &codes)?;

    Ok(cached)
}

fn build_family(
    config: &str,
    out: Option<&str>,
    only: Option<&str>,
    subset: Option<&str>,
) -> Result<(), String> {
    // r[impl family.static]
    let config_path = PathBuf::from(config);
    let text = fs::read_to_string(&config_path)
        .map_err(|error| format!("reading {}: {error}", config_path.display()))?;
    let document = parse_family(&config_path, &text)?;
    let family = document.family;
    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    let resolved = family.resolve(config_dir)?;

    let out_dir = out.map_or_else(|| resolve_path(config_dir, &family.out), PathBuf::from);
    fs::create_dir_all(&out_dir)
        .map_err(|error| format!("creating {}: {error}", out_dir.display()))?;

    let base_capture = family
        .capture
        .as_ref()
        .map(|path| resolve_path(config_dir, path));
    let codepoints = if let Some(spec) = &family.codepoints {
        expand_ranges(&parse_ranges(spec)?)?
    } else if let Some(path) = &base_capture {
        capture_codepoints(path)?
    } else if let Some(path) = resolved
        .iter()
        .find_map(|instance| instance.capture.as_ref())
    {
        capture_codepoints(path)?
    } else {
        return Err(
            "the family needs `codepoints` or a `capture` to know what to draw".to_string(),
        );
    };
    if codepoints.is_empty() {
        return Err("the family's codepoint set is empty".to_string());
    }

    let only = match only {
        None => None,
        Some(text) => {
            let ids: BTreeSet<String> = text
                .split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(str::to_string)
                .collect();
            if ids.is_empty() {
                return Err("--only names no instances".to_string());
            }
            for id in &ids {
                if !resolved.iter().any(|instance| &instance.id == id) {
                    return Err(format!("--only names unknown instance {id}"));
                }
            }
            Some(ids)
        }
    };
    let formats = family.formats.clone();
    if formats.is_empty() {
        return Err("the family names no output formats".to_string());
    }
    if !formats
        .iter()
        .any(|format| matches!(format, OutputFormat::Ttf | OutputFormat::Woff2))
    {
        return Err("a family needs TTF or WOFF2 output".to_string());
    }

    let vendor = font::vendor_tag(&family.vendor)?;
    let mut instances = Vec::new();
    let mut css = String::new();
    let mut built = 0usize;

    for instance in &resolved {
        if only
            .as_ref()
            .is_some_and(|only| !only.contains(&instance.id))
        {
            continue;
        }
        built += 1;
        let shape = lemmata_source::shape_of(&instance.plan);
        let (capture_path, mut capture_source) = if let Some(path) = &instance.capture {
            (path.clone(), "file")
        } else {
            (host_capture(&shape, &codepoints)?, "host")
        };
        if let Some(recorded) = lemmata_host::read_shape(&capture_path) {
            if !lemmata_host::shape_matches(&recorded, &shape, &codepoints) {
                return Err(format!(
                    "{}: {} records a different shape",
                    instance.id,
                    capture_path.display()
                ));
            }
        } else if capture_source == "file" {
            // A frozen capture has no sidecar. It is still usable, but the
            // report says the shape was not checked.
            capture_source = "file-unverified";
        }

        let data = fs::read(&capture_path)
            .map_err(|error| format!("reading {}: {error}", capture_path.display()))?;
        let names = Names {
            family: instance.legacy_family.clone(),
            subfamily: instance.legacy_subfamily.clone(),
            typographic_family: Some(instance.typographic_family.clone()),
            typographic_subfamily: Some(instance.typographic_subfamily.clone()),
            postscript_name: Some(instance.postscript_name.clone()),
            version: family.version.clone(),
            vendor,
        };
        let result = build_face(&instance.plan, &data, &names, subset)?;
        match family.policy {
            CoveragePolicy::Strict
                if result.coverage.captured > 0 || result.coverage.missing > 0 =>
            {
                return Err(format!(
                    "{}: {} glyphs came from the capture and {} are missing; the family is strict",
                    instance.id, result.coverage.captured, result.coverage.missing
                ));
            }
            CoveragePolicy::PortOnly if result.coverage.captured > 0 => {
                return Err(format!(
                    "{}: {} glyphs came from the capture; the family is port-only",
                    instance.id, result.coverage.captured
                ));
            }
            _ => {}
        }

        let stem = safe_stem(&instance.postscript_name)?;
        let ttf = format!("{stem}.ttf");
        let woff2 = format!("{stem}.woff2");
        let mut files = BTreeMap::new();
        if formats.contains(&OutputFormat::Ttf) {
            write_file(&out_dir.join(&ttf), &result.bytes)?;
            files.insert("ttf".to_string(), ttf.clone());
        }
        if formats.contains(&OutputFormat::Woff2) {
            let compressed = font::woff2(&result.bytes)?;
            write_file(&out_dir.join(&woff2), &compressed)?;
            files.insert("woff2".to_string(), woff2.clone());
        }
        if formats.contains(&OutputFormat::Css) {
            css.push_str(&css_face(
                &instance.plan,
                &instance.typographic_family,
                &stem,
                &files,
            ));
        }

        let shown = if formats.contains(&OutputFormat::Ttf) {
            out_dir.join(&ttf)
        } else {
            out_dir.join(&woff2)
        };
        println!(
            "{}: {} glyphs, {} bytes, {} ({} ported, {} captured, {} missing)",
            shown.display(),
            result.glyphs,
            result.bytes.len(),
            instance.legacy_subfamily.to_lowercase(),
            result.coverage.ported,
            result.coverage.captured,
            result.coverage.missing
        );
        instances.push(InstanceManifest {
            id: instance.id.clone(),
            family: instance.legacy_family.clone(),
            subfamily: instance.legacy_subfamily.clone(),
            typographic_family: instance.typographic_family.clone(),
            typographic_subfamily: instance.typographic_subfamily.clone(),
            postscript_name: instance.postscript_name.clone(),
            weight: instance.plan.weight_class,
            width: instance.plan.width,
            slope: instance.plan.italic_angle,
            serifs: instance.plan.serifs.clone(),
            spacing: instance.plan.spacing.name().to_string(),
            files,
            coverage: CoverageManifest {
                ported: result.coverage.ported,
                captured: result.coverage.captured,
                missing: result.coverage.missing,
                capture: capture_source.to_string(),
            },
        });
    }

    if built == 0 {
        return Err("--only matched no instances".to_string());
    }

    let manifest = FamilyManifest {
        name: family.name.clone(),
        version: family.version.clone(),
        vendor: family.vendor.clone(),
        instances,
    };
    let manifest = serde_json::to_string_pretty(&manifest)
        .map_err(|error| format!("lemmata.json: {error}"))?;
    write_file(&out_dir.join("lemmata.json"), manifest.as_bytes())?;
    if formats.contains(&OutputFormat::Css) {
        let header = format!("/* {} {} */\n", family.name, family.version);
        write_file(
            &out_dir.join("lemmata.css"),
            format!("{header}{css}").as_bytes(),
        )?;
    }

    Ok(())
}

fn parse_family(path: &Path, text: &str) -> Result<FamilyDocument, String> {
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        lemmata_plan::family_from_json(text)
    } else {
        toml::from_str(text).map_err(|error| error.to_string())
    }
}

fn resolve_path(base: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

fn capture_codepoints(path: &Path) -> Result<Vec<u32>, String> {
    let data = fs::read(path).map_err(|error| format!("reading {}: {error}", path.display()))?;
    let captured = lemmata_capture::load(&data)?;
    let mut codes: Vec<u32> = captured
        .iter()
        .flat_map(|glyph| glyph.unicodes.iter().copied())
        .collect();
    codes.sort_unstable();
    codes.dedup();

    Ok(codes)
}

fn safe_stem(name: &str) -> Result<String, String> {
    if name.is_empty()
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err(format!(
            "{name} is not a file-safe PostScript name; use letters, digits and hyphens"
        ));
    }

    Ok(name.to_string())
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("creating {}: {error}", parent.display()))?;
    }

    fs::write(path, bytes).map_err(|error| format!("writing {}: {error}", path.display()))
}

fn css_face(plan: &Plan, family: &str, stem: &str, files: &BTreeMap<String, String>) -> String {
    let mut sources = Vec::new();
    if files.contains_key("woff2") {
        sources.push(format!("url(\"./{stem}.woff2\") format(\"woff2\")"));
    }
    if files.contains_key("ttf") {
        sources.push(format!("url(\"./{stem}.ttf\") format(\"truetype\")"));
    }
    let style = if plan.italic_angle == 0.0 {
        "normal"
    } else {
        "italic"
    };
    let stretch = width_percent(plan.width_class);

    format!(
        "@font-face {{\n  font-family: \"{family}\";\n  src: {};\n  font-weight: {};\n  font-style: {style};\n  font-stretch: {stretch}%;\n  font-display: swap;\n}}\n",
        sources.join(",\n       "),
        plan.weight_class
    )
}

const fn width_percent(class: u16) -> &'static str {
    match class {
        1 => "50",
        2 => "62.5",
        3 => "75",
        4 => "87.5",
        5 => "100",
        6 => "112.5",
        7 => "125",
        8 => "150",
        _ => "200",
    }
}

#[derive(Serialize)]
struct FamilyManifest {
    name: String,
    version: String,
    vendor: String,
    instances: Vec<InstanceManifest>,
}

#[derive(Serialize)]
struct InstanceManifest {
    id: String,
    family: String,
    subfamily: String,
    typographic_family: String,
    typographic_subfamily: String,
    postscript_name: String,
    weight: u16,
    width: f64,
    slope: f64,
    serifs: String,
    spacing: String,
    files: BTreeMap<String, String>,
    coverage: CoverageManifest,
}

#[derive(Serialize)]
struct CoverageManifest {
    ported: usize,
    captured: usize,
    missing: usize,
    capture: String,
}
