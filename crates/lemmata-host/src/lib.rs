//! The host seam: upstream's compiled glyph code, run for a plan.
//!
//! r[impl port.strategy]
//!
//! The glyph-source decision is to host upstream's code rather than port it, and
//! this is the half that turns parameters into outlines. The other half is
//! `lemmata-ptl::param`, a port of `@iosevka/param` checked against it over
//! twenty shapes.
//!
//! What crosses the boundary is upstream's build, and what it leaves behind is
//! the same capture the frozen ones were, so everything downstream is unchanged.
//! The plan is what makes it a seam rather than an artifact: weight, width and
//! slope used to be baked into whichever build happened to be pinned, and a
//! capture at another shape is now a call.
//!
//! It runs through `npm`, because the glyph code is PTL compiled to JavaScript
//! and that is the toolchain which compiles it. In a browser the engine comes
//! free and the same boundary is a bundle rather than a process.

use std::path::Path;
use std::process::Command;

use lemmata_ptl::param::Shape;

/// The name a shape's capture is built under.
///
/// The build keys its cache and its output directory by this, so two shapes need
/// two names and one shape has to keep its own.
#[must_use]
pub fn plan_name(shape: &Shape) -> String {
    let slope = if shape.slope_angle == 0.0 {
        "upright"
    } else {
        "italic"
    };

    format!(
        "Host{}x{}-{slope}",
        shape.weight.round() as i64,
        shape.width.round() as i64
    )
}

/// The upstream build plan for one shape, as TOML.
///
/// One weight, one width and one slope, so the build makes the single face the
/// capture is for rather than the family a plan usually names.
#[must_use]
pub fn plan_toml(name: &str, shape: &Shape) -> String {
    let serifs = shape.serifs.as_deref().unwrap_or("sans");
    let spacing = shape.spacing.as_deref().unwrap_or("normal");
    let weight = shape.weight.round() as i64;
    let width = shape.width.round() as i64;
    // The width class is what a font menu shows, and a hundred units a step is
    // the scale it is named on.
    let class = (width / 100).clamp(1, 9);
    let angle = shape.slope_angle;
    let (slope_shape, slope_menu, slope_css) = if angle == 0.0 {
        ("upright", "upright", "normal")
    } else {
        ("oblique", "italic", "italic")
    };

    format!(
        "\n# Written by `lemmata-host` for one shape. The plans above are the ones\n\
         # the captures in the repository were built from.\n\
         [buildPlans.{name}]\n\
         family = \"Lemmata Host {name}\"\n\
         spacing = \"{spacing}\"\n\
         serifs = \"{serifs}\"\n\
         exportGlyphNames = true\n\
         \n\
         [buildPlans.{name}.weights.Host]\n\
         shape = {weight}\n\
         menu = {weight}\n\
         css = {weight}\n\
         \n\
         [buildPlans.{name}.widths.Host]\n\
         shape = {width}\n\
         menu = {class}\n\
         css = \"normal\"\n\
         \n\
         [buildPlans.{name}.slopes.Host]\n\
         angle = {angle}\n\
         shape = \"{slope_shape}\"\n\
         menu = \"{slope_menu}\"\n\
         css = \"{slope_css}\"\n"
    )
}

/// Runs upstream's build for one shape and returns the capture it leaves.
///
/// `codepoints` is what the capture carries. The build is much larger than that,
/// and the dump stops it as soon as the glyph store exists.
///
/// # Errors
///
/// Returns a message when the plan cannot be written, the build fails, or the
/// dump leaves nothing behind.
pub fn capture(shape: &Shape, codepoints: &[u32], build_dir: &Path) -> Result<Vec<u8>, String> {
    let name = plan_name(shape);
    let plans = build_dir.join("private-build-plans.toml");
    let existing = std::fs::read_to_string(&plans)
        .map_err(|error| format!("reading {}: {error}", plans.display()))?;
    if !existing.contains(&format!("[buildPlans.{name}]")) {
        let mut next = existing;
        if !next.ends_with('\n') {
            next.push('\n');
        }
        next.push_str(&plan_toml(&name, shape));
        std::fs::write(&plans, next)
            .map_err(|error| format!("writing {}: {error}", plans.display()))?;
    }

    // The build caches by plan and a cached build does not re-run the dump, so
    // both the output directory and the capture go before the next run starts.
    let _ = std::fs::remove_dir_all(build_dir.join("dist").join(&name));
    let out = build_dir.join(format!("{name}.bin"));
    let _ = std::fs::remove_file(&out);

    let list = codepoints
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let status = Command::new("npm")
        .current_dir(build_dir)
        .args(["run", "build", "--", &format!("ttf-unhinted::{name}")])
        .env("LEMMATA_GLYPH_DUMP", &out)
        .env(
            "LEMMATA_GLYPH_WIDTH",
            (shape.width.round() as i64).to_string(),
        )
        .env("LEMMATA_GLYPH_CODEPOINTS", &list)
        .status()
        .map_err(|error| format!("running npm in {}: {error}", build_dir.display()))?;
    if !status.success() {
        return Err(format!("the build for {name} left {status}"));
    }

    std::fs::read(&out).map_err(|error| format!("reading {}: {error}", out.display()))
}
