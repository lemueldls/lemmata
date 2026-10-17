//! Pulling outlines out of a font with skrifa.

use kurbo::{BezPath, Point};
use read_fonts::TableProvider;
use skrifa::outline::{DrawError, DrawSettings, OutlinePen};
use skrifa::prelude::{FontRef, GlyphId, LocationRef, MetadataProvider, Size};

/// One glyph, as cubic contours in font units.
#[derive(Clone, Debug)]
pub struct GlyphOutline {
    pub glyph_id: GlyphId,
    pub contours: Vec<BezPath>,
}

#[derive(Debug)]
pub enum Error {
    Font(read_fonts::ReadError),
    Draw(DrawError),
    Missing(GlyphId),
}

impl From<read_fonts::ReadError> for Error {
    fn from(error: read_fonts::ReadError) -> Self {
        Self::Font(error)
    }
}

impl From<DrawError> for Error {
    fn from(error: DrawError) -> Self {
        Self::Draw(error)
    }
}

/// Number of glyphs in the font.
pub fn glyph_count(data: &[u8]) -> Result<u16, Error> {
    let font = read_fonts::FontRef::new(data)?;
    Ok(font.maxp()?.num_glyphs())
}

/// Outline for a single glyph.
pub fn outline(data: &[u8], glyph_id: GlyphId) -> Result<GlyphOutline, Error> {
    let font = FontRef::new(data)?;
    let glyph = font
        .outline_glyphs()
        .get(glyph_id)
        .ok_or(Error::Missing(glyph_id))?;
    let mut pen = PathPen::default();
    glyph.draw(
        DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
        &mut pen,
    )?;

    Ok(GlyphOutline {
        glyph_id,
        contours: pen.finish(),
    })
}

/// Outlines for every glyph in the font, in glyph ID order.
pub fn outlines(data: &[u8]) -> Result<Vec<GlyphOutline>, Error> {
    let count = glyph_count(data)?;
    let font = FontRef::new(data)?;
    let glyphs = font.outline_glyphs();
    let mut result = Vec::with_capacity(usize::from(count));
    for raw_id in 0..count {
        let glyph_id = GlyphId::new(u32::from(raw_id));
        if let Some(glyph) = glyphs.get(glyph_id) {
            let mut pen = PathPen::default();
            glyph.draw(
                DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                &mut pen,
            )?;
            result.push(GlyphOutline {
                glyph_id,
                contours: pen.finish(),
            });
        }
    }

    Ok(result)
}

#[derive(Default)]
struct PathPen {
    contours: Vec<BezPath>,
    current: Option<BezPath>,
}

impl PathPen {
    fn flush(&mut self) {
        if let Some(path) = self.current.take() {
            self.contours.push(path);
        }
    }

    fn finish(mut self) -> Vec<BezPath> {
        self.flush();
        self.contours
    }
}

impl OutlinePen for PathPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.flush();
        let mut path = BezPath::new();
        path.move_to(point(x, y));
        self.current = Some(path);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        if let Some(path) = self.current.as_mut() {
            path.line_to(point(x, y));
        }
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        if let Some(path) = self.current.as_mut() {
            path.quad_to(point(cx0, cy0), point(x, y));
        }
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        if let Some(path) = self.current.as_mut() {
            path.curve_to(point(cx0, cy0), point(cx1, cy1), point(x, y));
        }
    }

    fn close(&mut self) {
        if let Some(path) = self.current.as_mut() {
            path.close_path();
        }
    }
}

fn point(x: f32, y: f32) -> Point {
    Point::new(f64::from(x), f64::from(y))
}
