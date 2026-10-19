//! The host seam's gate: a shape the repository has no capture for, checked
//! against the font upstream built from the same plan.
//!
//! `r[port.strategy]` says the oracle gates drift under either strategy. The
//! frozen captures gate the replay; this gates the host, which is the half that
//! decides whether a capture is the plan it claims to be.
//!
//! Skipped unless `LEMMATA_HOST_BUILD` is set and the scratch build and the
//! reference font are both there, because it starts an external build.

// A captured advance is a whole number of font units, so rounding it into the
// field the font stores is the conversion rather than a truncation.
#![allow(clippy::cast_possible_truncation)]

use std::path::{Path, PathBuf};

use lemmata_build::font::{GlyphData, Metrics, Names};
use lemmata_ptl::param::Shape;
use read_fonts::tables::glyf::Glyph as ReadGlyph;
use read_fonts::types::GlyphId;
use read_fonts::{FontRef, TableProvider};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The ink's horizontal extent of a glyph, in font units.
fn ink_width(font: &FontRef, id: GlyphId) -> Option<i32> {
    let loca = font.loca(None).ok()?;
    let glyf = font.glyf().ok()?;
    let Ok(Some(glyph)) = loca.get_glyf(id, &glyf) else {
        return None;
    };
    let ReadGlyph::Simple(simple) = glyph else {
        // Upstream composes a dotted letter; ours is one contour.
        return None;
    };
    let (mut low, mut high) = (i32::MAX, i32::MIN);
    for point in simple.points() {
        low = low.min(i32::from(point.x));
        high = high.max(i32::from(point.x));
    }

    Some(high - low)
}

#[test]
fn a_capture_is_the_plan_it_claims_to_be() {
    if std::env::var_os("LEMMATA_HOST_BUILD").is_none() {
        eprintln!("skipping: set LEMMATA_HOST_BUILD to run the host");
        return;
    }
    let build = PathBuf::from(
        std::env::var("IOSEVKA_BUILD_DIR").unwrap_or_else(|_| "/tmp/opencode/iosevka-build".into()),
    );
    if !build.join("verdafile.mjs").exists() {
        eprintln!("skipping: no scratch build at {}", build.display());
        return;
    }
    let reference = root().join("target/reference/RefB-Bold.ttf");
    let Ok(upstream_bytes) = std::fs::read(&reference) else {
        eprintln!("skipping: no {}", reference.display());
        return;
    };
    let upstream = FontRef::new(&upstream_bytes).expect("the reference parses");

    // RefB: slab serifs, bold, the same width the reference plan builds.
    let shape = Shape {
        serifs: Some("slab".to_string()),
        spacing: Some("normal".to_string()),
        weight: 700.0,
        width: 500.0,
        ..Shape::default()
    };
    let codepoints: Vec<u32> = (u32::from('a')..=u32::from('z')).collect();
    let capture = lemmata_host::capture(&shape, &codepoints, &build).expect("the host runs");
    let captured = lemmata_capture::load(&capture).expect("the capture decodes");
    assert!(!captured.is_empty(), "the capture carries nothing");

    let glyphs: Vec<GlyphData> = captured
        .iter()
        .map(|glyph| GlyphData {
            name: glyph.name.clone(),
            unicodes: glyph.unicodes.clone(),
            advance: glyph.advance.round().clamp(0.0, f64::from(u16::MAX)) as u16,
            contours: lemmata_capture::contours(glyph),
        })
        .collect();
    let bytes = lemmata_build::font::build(&glyphs, &Metrics::default(), &Names::default(), &[])
        .expect("our font builds");
    let ours = FontRef::new(&bytes).expect("our font reopens");

    // Every glyph the two fonts share has to sit in the same ink. A capture of
    // the wrong shape is not a small error: the bold `m` is 84 units wider than
    // the regular one, so a tolerance of two units tells the plans apart.
    let (our_cmap, up_cmap) = (ours.cmap().unwrap(), upstream.cmap().unwrap());
    let mut compared = 0usize;
    for code in &codepoints {
        let (Some(mine), Some(theirs)) =
            (our_cmap.map_codepoint(*code), up_cmap.map_codepoint(*code))
        else {
            continue;
        };
        let (Some(a), Some(b)) = (ink_width(&ours, mine), ink_width(&upstream, theirs)) else {
            continue;
        };
        assert!(
            (a - b).abs() <= 2,
            "{}: ours is {a} wide and upstream's is {b}",
            char::from_u32(*code).unwrap_or('?')
        );
        compared += 1;
    }

    assert!(compared >= 20, "only {compared} glyphs compared");
    eprintln!("host: {compared} glyphs at the bold plan match upstream");
}
