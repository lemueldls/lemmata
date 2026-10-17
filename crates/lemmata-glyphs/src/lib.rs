//! Glyph data captured from a pinned Iosevka build, and the layer glyph
//! definitions are written in.
//!
//! The capture writes, for each glyph, its name, unicodes, advance width, glyph
//! gizmo and the geometry tree its definitions produced. Feeding the tree back
//! through [`lemmata_geom::Geometry`] reproduces the outline with our own
//! geometry code, which is how the seam is meant to work: upstream's glyph
//! definitions, our primitives.
//!
//! The modules below are the layer the definitions themselves are written in.
//! [`control`], [`spiro_kit`] and [`metrics`] are the shape side: control knots,
//! knot pairs, the flattener that resolves interpolators, the collector that
//! turns controls into bi-knots, the knot builders, and the metrics every
//! definition reads. [`glyph`] is the composition side: a [`glyph::Glyph`]
//! accumulates shapes into the same geometry tree upstream builds. [`util`] is
//! the numeric vocabulary the definitions are written with. `tests/spiro_kit.rs`
//! and `tests/glyph_compose.rs` compare those layers against upstream's own,
//! and `tests/metrics.rs` compares all 105 metrics against the ones the build
//! derived.

pub mod control;
pub mod glyph;
pub mod letter_model;
pub mod metrics;
pub mod spiro_kit;
pub mod store;
pub mod util;

use kurbo::{Affine, Vec2};
use lemmata_geom::{Bez3, BiKnot, Geometry, KnotType, MonoKnot, PenKnot};
use linesweeper::BinaryOp;

/// One captured glyph.
#[derive(Clone, Debug)]
pub struct Glyph {
    pub name: String,
    pub unicodes: Vec<u32>,
    pub advance: f64,
    pub gizmo: Affine,
    pub geometry: Geometry,
}

/// Reads a capture file.
///
/// # Errors
///
/// Returns an error message when the buffer is truncated or holds an unknown tag.
pub fn load(data: &[u8]) -> Result<Vec<Glyph>, String> {
    let mut reader = Reader::new(data);
    let count = reader.u32()?;
    let mut glyphs = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let name = reader.string()?;
        let advance = reader.f64()?;
        let unicode_count = reader.u32()?;
        let mut unicodes = Vec::with_capacity(unicode_count as usize);
        for _ in 0..unicode_count {
            unicodes.push(reader.u32()?);
        }
        let gizmo = reader.transform()?;
        let geometry = reader.node(0)?;
        glyphs.push(Glyph {
            name,
            unicodes,
            advance,
            gizmo,
            geometry,
        });
    }
    Ok(glyphs)
}

/// The outline of a captured glyph, as quadratic contours.
#[must_use]
pub fn contours(glyph: &Glyph) -> Vec<Vec<lemmata_geom::QuadifyPoint>> {
    Geometry::Simplify {
        gizmo: glyph.gizmo,
        geom: Box::new(glyph.geometry.clone()),
    }
    .to_contours()
}

struct Reader<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    const fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self.position + count;
        if end > self.data.len() {
            return Err("truncated capture".to_string());
        }
        let slice = &self.data[self.position..end];
        self.position = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, String> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn f64(&mut self) -> Result<f64, String> {
        let bytes = self.take(8)?;
        Ok(f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn string(&mut self) -> Result<String, String> {
        let length = self.u32()? as usize;
        let bytes = self.take(length)?;
        String::from_utf8(bytes.to_vec()).map_err(|error| error.to_string())
    }

    fn transform(&mut self) -> Result<Affine, String> {
        let xx = self.f64()?;
        let xy = self.f64()?;
        let yx = self.f64()?;
        let yy = self.f64()?;
        let tx = self.f64()?;
        let ty = self.f64()?;
        Ok(Affine::new([xx, yx, xy, yy, tx, ty]))
    }

    fn node(&mut self, depth: usize) -> Result<Geometry, String> {
        if depth > 64 {
            return Err("geometry too deep".to_string());
        }
        match self.u8()? {
            0 => Ok(Geometry::Invalid),
            1 => {
                let contour_count = self.u32()?;
                let mut contours = Vec::with_capacity(contour_count as usize);
                for _ in 0..contour_count {
                    let arc_count = self.u32()?;
                    let mut contour = Vec::with_capacity(arc_count as usize);
                    for _ in 0..arc_count {
                        let a = self.point()?;
                        let b = self.point()?;
                        let c = self.point()?;
                        let d = self.point()?;
                        contour.push(Bez3::new(a, b, c, d));
                    }
                    contours.push(contour);
                }
                Ok(Geometry::Contours(contours))
            }
            2 => {
                let gizmo = self.transform()?;
                let closed = self.u8()? != 0;
                let knot_count = self.u32()?;
                let mut knots = Vec::with_capacity(knot_count as usize);
                for _ in 0..knot_count {
                    let ty = knot_type(&self.string()?);
                    let x = self.f64()?;
                    let y = self.f64()?;
                    let unimportant = self.u8()? != 0;
                    knots.push(MonoKnot::new(ty, x, y, unimportant));
                }
                Ok(Geometry::Spiro {
                    gizmo,
                    knots,
                    closed,
                })
            }
            3 => {
                let gizmo = self.transform()?;
                let contrast = self.f64()?;
                let closed = self.u8()? != 0;
                let knot_count = self.u32()?;
                let mut knots = Vec::with_capacity(knot_count as usize);
                for _ in 0..knot_count {
                    let ty = knot_type(&self.string()?);
                    let x = self.f64()?;
                    let y = self.f64()?;
                    let d1 = self.f64()?;
                    let d2 = self.f64()?;
                    let unimportant = self.u8()? != 0;
                    let has_normal = self.u8()? != 0;
                    let proposed_normal = if has_normal {
                        Some(Vec2::new(self.f64()?, self.f64()?))
                    } else {
                        None
                    };
                    let mut knot = BiKnot::new(ty, x, y, d1, d2);
                    knot.unimportant = unimportant;
                    knot.proposed_normal = proposed_normal;
                    knots.push(knot);
                }
                Ok(Geometry::DiSpiro {
                    gizmo,
                    contrast,
                    closed,
                    knots,
                })
            }
            4 => {
                let gizmo = self.transform()?;
                let closed = self.u8()? != 0;
                let profile_count = self.u32()?;
                let mut pen_profile = Vec::with_capacity(profile_count as usize);
                for _ in 0..profile_count {
                    pen_profile.push(Vec2::new(self.f64()?, self.f64()?));
                }
                let knot_count = self.u32()?;
                let mut knots = Vec::with_capacity(knot_count as usize);
                for _ in 0..knot_count {
                    let ty = knot_type(&self.string()?);
                    let x = self.f64()?;
                    let y = self.f64()?;
                    let profile = if self.u8()? != 0 {
                        let count = self.u32()?;
                        let mut points = Vec::with_capacity(count as usize);
                        for _ in 0..count {
                            points.push(Vec2::new(self.f64()?, self.f64()?));
                        }
                        Some(points)
                    } else {
                        None
                    };
                    knots.push(PenKnot::new(ty, x, y, profile));
                }
                Ok(Geometry::SpiroPen {
                    gizmo,
                    pen_profile,
                    closed,
                    knots,
                })
            }
            5 => {
                let gizmo = self.transform()?;
                let radius = self.f64()?;
                let contrast = self.f64()?;
                let inside = self.u8()? != 0;
                let geom = Box::new(self.node(depth + 1)?);
                Ok(Geometry::Stroke {
                    gizmo,
                    radius,
                    contrast,
                    inside,
                    geom,
                })
            }
            6 => {
                let gizmo = self.transform()?;
                let geom = Box::new(self.node(depth + 1)?);
                Ok(Geometry::RemoveHoles { gizmo, geom })
            }
            7 => {
                let gizmo = self.transform()?;
                let geom = Box::new(self.node(depth + 1)?);
                Ok(Geometry::Simplify { gizmo, geom })
            }
            8 => {
                let part_count = self.u32()?;
                let mut parts = Vec::with_capacity(part_count as usize);
                for _ in 0..part_count {
                    parts.push(self.node(depth + 1)?);
                }
                Ok(Geometry::Combine(parts))
            }
            9 => {
                let operator = self.string()?;
                let operand_count = self.u32()?;
                let mut operands = Vec::with_capacity(operand_count as usize);
                for _ in 0..operand_count {
                    operands.push(self.node(depth + 1)?);
                }
                Ok(Geometry::Boolean {
                    op: match operator.as_str() {
                        "intersection" => BinaryOp::Intersection,
                        "difference" => BinaryOp::Difference,
                        "xor" => BinaryOp::Xor,
                        _ => BinaryOp::Union,
                    },
                    operands,
                })
            }
            10 => {
                let tf = self.transform()?;
                let geom = Box::new(self.node(depth + 1)?);
                Ok(Geometry::Transformed { tf, geom })
            }
            11 => Ok(Geometry::Tagged(Box::new(self.node(depth + 1)?))),
            12 => Ok(Geometry::Radical(Box::new(self.node(depth + 1)?))),
            13 => self.node(depth + 1),
            tag => Err(format!("unknown node tag {tag}")),
        }
    }

    fn point(&mut self) -> Result<kurbo::Point, String> {
        Ok(kurbo::Point::new(self.f64()?, self.f64()?))
    }
}

fn knot_type(name: &str) -> KnotType {
    match name {
        "open" => KnotType::Open,
        "open_end" => KnotType::OpenEnd,
        "left" => KnotType::Left,
        "right" => KnotType::Right,
        "g2" => KnotType::G2,
        "g4" => KnotType::G4,
        _ => KnotType::Corner,
    }
}
