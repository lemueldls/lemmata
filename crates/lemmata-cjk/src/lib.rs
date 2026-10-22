//! CJK glyphs from a built font, as the builder's glyph data.
//!
//! The source is a font rather than a definition: Sarasa Gothic and Source Han
//! Sans are composites of other projects, and their CJK glyphs are already
//! drawn. This crate reads the outlines the source holds, scales them into the
//! Latin cell's frame, and hands them to the same table writer as the ported
//! glyphs.
//!
//! Only TrueType outlines are read. The writer is `glyf`, and a source that
//! holds CFF or CFF2 would need a cubic-to-quadratic step that does not exist
//! here yet.
//!
//! r[impl cjk.source]
//! r[impl cjk.width]
//! r[impl cjk.ranges]

use std::collections::BTreeSet;

use lemmata_build::font::GlyphData;
use lemmata_geom::{QuadifyKind, QuadifyPoint};
use read_fonts::{FontRef, TableProvider};
use skrifa::{
    MetadataProvider,
    instance::{LocationRef, Size},
    outline::{DrawSettings, OutlinePen},
};

/// The CJK ranges a Chinese face usually carries.
///
/// Extension B is left out: its 42,000 glyphs plus the rest would pass the
/// 65,535 glyphs a `maxp` table can count. A caller that wants it can name the
/// range, and the source has to hold it.
pub const CN_RANGES: &[(u32, u32)] = &[
    (0x2e80, 0x2eff),
    (0x3000, 0x303f),
    (0x31c0, 0x31ef),
    (0x3400, 0x4dbf),
    (0x4e00, 0x9fff),
    (0xff00, 0xffef),
];

/// One CJK glyph set, read from a source font.
///
/// `target_cell` is the Latin advance, and `scale` is how many Latin cells one
/// full-width CJK glyph should occupy. The source's em is the source's CJK
/// cell, which is what makes Sarasa Mono 2:1 at Iosevka's 500: a 1000-unit
/// ideograph fills two cells.
///
/// # Errors
///
/// Returns a message when the source is not a TrueType font, a glyph uses cubic
/// outlines, or the requested ranges would exceed the glyph count `maxp` can
/// hold.
pub fn glyphs(
    source: &[u8],
    ranges: &[(u32, u32)],
    target_cell: f64,
    scale: f64,
    units_per_em: f64,
) -> Result<Vec<GlyphData>, String> {
    let font = FontRef::new(source).map_err(|error| format!("reading the CJK source: {error}"))?;
    let head = font
        .head()
        .map_err(|error| format!("reading the CJK source's head: {error}"))?;
    let source_upem = f64::from(head.units_per_em());
    if source_upem <= 0.0 {
        return Err("the CJK source has no units per em".to_string());
    }
    let hmtx = font
        .hmtx()
        .map_err(|error| format!("reading the CJK source's hmtx: {error}"))?;
    let charmap = font.charmap();
    let outlines = font.outline_glyphs();
    let x_scale = target_cell * scale / source_upem;
    let y_scale = units_per_em / source_upem;
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for &(low, high) in ranges {
        if low > high {
            return Err(format!("U+{low:04X}-U+{high:04X} is reversed"));
        }
        for code in low..=high {
            if !seen.insert(code) {
                continue;
            }
            let Some(glyph_id) = charmap.map(code) else {
                continue;
            };
            let Some(glyph) = outlines.get(glyph_id) else {
                continue;
            };
            let mut pen = Collector::new(x_scale, y_scale);
            glyph
                .draw(
                    DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                    &mut pen,
                )
                .map_err(|error| format!("drawing U+{code:04X}: {error}"))?;
            if pen.cubic {
                return Err(format!(
                    "U+{code:04X} uses cubic outlines; the source must be TrueType"
                ));
            }
            let contours = pen.finish();
            if contours.is_empty() {
                continue;
            }
            let source_advance = f64::from(hmtx.advance(glyph_id).unwrap_or(0));
            let advance = (source_advance * x_scale).round();
            if !(0.0..=f64::from(u16::MAX)).contains(&advance) {
                return Err(format!("U+{code:04X} has an advance that does not fit"));
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let advance = advance as u16;
            out.push(GlyphData {
                name: format!("cjk/{code:04X}"),
                unicodes: vec![code],
                advance,
                contours,
            });
        }
    }
    if out.len() > 65_000 {
        return Err(format!(
            "the CJK ranges select {} glyphs; maxp holds 65,535",
            out.len()
        ));
    }

    Ok(out)
}

/// Records a glyph's outline as the builder's quadratic points.
struct Collector {
    contours: Vec<Vec<QuadifyPoint>>,
    current: Vec<QuadifyPoint>,
    x_scale: f64,
    y_scale: f64,
    cubic: bool,
}

impl Collector {
    const fn new(x_scale: f64, y_scale: f64) -> Self {
        Self {
            contours: Vec::new(),
            current: Vec::new(),
            x_scale,
            y_scale,
            cubic: false,
        }
    }

    fn point(&self, kind: QuadifyKind, x: f32, y: f32) -> QuadifyPoint {
        QuadifyPoint {
            kind,
            x: f64::from(x) * self.x_scale,
            y: f64::from(y) * self.y_scale,
        }
    }

    fn flush(&mut self) {
        if self.current.len() > 1 {
            self.contours.push(std::mem::take(&mut self.current));
        } else {
            self.current.clear();
        }
    }

    fn finish(mut self) -> Vec<Vec<QuadifyPoint>> {
        self.flush();

        self.contours
    }
}

impl OutlinePen for Collector {
    fn move_to(&mut self, x: f32, y: f32) {
        self.flush();
        self.current.push(self.point(QuadifyKind::Corner, x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.current.push(self.point(QuadifyKind::Corner, x, y));
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.current
            .push(self.point(QuadifyKind::Quadratic, cx, cy));
        self.current.push(self.point(QuadifyKind::Corner, x, y));
    }

    fn curve_to(&mut self, _cx0: f32, _cy0: f32, _cx1: f32, _cy1: f32, _x: f32, _y: f32) {
        self.cubic = true;
    }

    fn close(&mut self) {
        self.flush();
    }
}
