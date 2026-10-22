//! The design suite: what a plan can ask for.
//!
//! `r[plan.combinations]` is the one that needed the glyph source. Weight, width
//! and slope change every outline, so a plan that names them needs a drawing for
//! each, and the host seam is what draws them.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use read_fonts::{FontRef, TableProvider, tables::glyf::Glyph as ReadGlyph, types::GlyphId};

fn workspace(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// The ink's horizontal extent of a glyph, in font units.
fn ink_width(font: &FontRef, id: GlyphId) -> Option<i32> {
    let loca = font.loca(None).ok()?;
    let glyf = font.glyf().ok()?;
    let Ok(Some(glyph)) = loca.get_glyf(id, &glyf) else {
        return None;
    };
    let ReadGlyph::Simple(simple) = glyph else {
        return None;
    };
    let (mut low, mut high) = (i32::MAX, i32::MIN);
    for point in simple.points() {
        low = low.min(i32::from(point.x));
        high = high.max(i32::from(point.x));
    }

    Some(high - low)
}

/// The width of the `m`'s ink in a font, which is what tells a bold drawing from
/// a regular one: the stem is thicker and the letter comes out wider.
fn m_ink(path: &Path) -> Option<i32> {
    let bytes = std::fs::read(path).ok()?;
    let font = FontRef::new(&bytes).ok()?;
    let id = font.cmap().ok()?.map_codepoint(u32::from('m'))?;

    ink_width(&font, id)
}

/// `r[plan.combinations]`: a plan that names a weight gets outlines for it.
///
/// Skipped unless `LEMMATA_HOST_BUILD` is set, because the first run starts an
/// external build. The second shape is cached, so it costs nothing.
#[test]
fn a_shape_gets_its_own_drawing() {
    if std::env::var_os("LEMMATA_HOST_BUILD").is_none() {
        eprintln!("skipping: set LEMMATA_HOST_BUILD to draw a shape");
        return;
    }
    let bold = workspace("target/reference/RefB-Bold.ttf");
    if !bold.exists() {
        eprintln!("skipping: no {}", bold.display());
        return;
    }

    let out = workspace("target/design-bold.ttf");
    let built = Command::new(env!("CARGO_BIN_EXE_lemmata"))
        .current_dir(workspace("."))
        .args(["build", "--weight", "700", "--serifs", "slab", "--out"])
        .arg(&out)
        .status()
        .expect("run the CLI");
    assert!(built.success(), "the CLI could not draw the bold shape");

    let ours = m_ink(&out).expect("our bold m");
    let upstream = m_ink(&bold).expect("upstream's bold m");
    assert!(
        (ours - upstream).abs() <= 2,
        "our bold m is {ours} and upstream's is {upstream}"
    );

    // And the shape is not a label on the same drawing: the captured regular is
    // 398 units of m against the bold one's 482.
    let regular = m_ink(&workspace("target/parity-regen.ttf")).unwrap_or(0);
    assert!(
        ours > regular + 40,
        "the bold m is {ours} against a regular {regular}, which is not a bolder drawing"
    );
    eprintln!("design: the bold plan draws an m of {ours} against {regular}");
}
