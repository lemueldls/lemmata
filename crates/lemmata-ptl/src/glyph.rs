//! The composition layer: how a glyph definition's shapes become one tree.
//!
//! Ported from `packages/glyph/src/glyph.mjs` and `font-kits/boole-kit.mjs`.
//! A [`Glyph`] starts with an empty combine and accumulates geometry as the
//! definition includes shapes, transforms them and combines them with boolean
//! operators. [`GlyphProc`] and [`ForkGlyphProc`] are the deferred forms, which
//! a definition returns instead of mutating a glyph directly.
//!
//! The store and the relation tables are not ported: they name and order
//! glyphs and build layout relations, neither of which reaches the outlines.
//! Anchors are carried but unused, since they feed the layout tables.

use kurbo::{Affine, Vec2};
use lemmata_geom::Geometry;
use linesweeper::BinaryOp;

/// Something a glyph can include: a shape, a proc, or another glyph.
pub trait Component {
    fn apply_to_glyph(&self, glyph: &mut Glyph, copy_anchors: bool, copy_width: bool);
}

/// A glyph under construction.
#[derive(Clone, Debug)]
pub struct Glyph {
    pub identifier: Option<String>,
    pub geometry: Geometry,
    pub gizmo: Affine,
    pub advance_width: f64,
    /// The tag every included geometry is wrapped in, when set.
    pub ctx_tag: Option<String>,
    /// Base anchors, by name. Carried for the layout tables.
    pub base_anchors: Vec<(String, Vec2)>,
    /// Mark anchors, by name. Carried for the layout tables.
    pub mark_anchors: Vec<(String, Vec2)>,
}

impl Default for Glyph {
    fn default() -> Self {
        Self::new()
    }
}

impl Glyph {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            identifier: None,
            geometry: Geometry::Combine(Vec::new()),
            gizmo: Affine::IDENTITY,
            advance_width: 500.0,
            ctx_tag: None,
            base_anchors: Vec::new(),
            mark_anchors: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_identifier(identifier: impl Into<String>) -> Self {
        Self {
            identifier: Some(identifier.into()),
            ..Self::new()
        }
    }

    /// `include`: hand this glyph to a component.
    pub fn include(&mut self, component: &impl Component, copy_anchors: bool, copy_width: bool) {
        component.apply_to_glyph(self, copy_anchors, copy_width);
    }

    /// `includeGeometry`: append a shape, tagged when a context tag is set.
    pub fn include_geometry(&mut self, geometry: Geometry) {
        // The tag's value is not modelled, so the wrapper is all that survives.
        let geometry = if self.ctx_tag.is_some() {
            Geometry::Tagged(Box::new(geometry))
        } else {
            geometry
        };
        self.geometry = combine_with(self.geometry.clone(), geometry);
    }

    /// `includeGlyph`: place another glyph, by reference when it is named.
    pub fn include_glyph(&mut self, other: &Self, copy_anchors: bool, copy_width: bool) {
        // Marks shift the placement, and the port carries no anchors yet, so the
        // shift is zero.
        let shift = Vec2::new(0.0, 0.0);
        self.include_glyph_impl(other, shift);
        if copy_anchors {
            self.base_anchors.extend(other.base_anchors.iter().cloned());
            self.mark_anchors.extend(other.mark_anchors.iter().cloned());
        }

        if copy_width && other.advance_width >= 0.0 {
            self.advance_width = other.advance_width;
        }
    }

    fn include_glyph_impl(&mut self, other: &Self, shift: Vec2) {
        // A named glyph becomes a reference upstream, which resolves to the same
        // translated geometry. The port places it directly.
        let _ = &other.identifier;
        let placed = Geometry::Transformed {
            tf: Affine::translate(shift),
            geom: Box::new(other.geometry.clone()),
        };
        self.include_geometry(placed);
    }

    /// `applyTransform`.
    pub fn apply_transform(&mut self, tf: Affine) {
        self.geometry = Geometry::Transformed {
            tf,
            geom: Box::new(self.geometry.clone()),
        };
    }

    pub const fn set_width(&mut self, width: f64) {
        self.advance_width = width;
    }

    pub fn clear_geometry(&mut self) {
        self.geometry = Geometry::Combine(Vec::new());
    }

    /// `ejectTagged`: drop geometry carrying this tag.
    pub fn eject_tagged(&mut self, tag: &str) {
        self.geometry = filter_tag(&self.geometry, tag);
    }
    // `tag` reaches `filter_tag` for parity with upstream's signature; the value
    // is not modelled.
}

/// `ejectTagged`: drop the geometry carrying a tag, keeping the rest.
#[must_use]
pub fn eject_tagged(geometry: &Geometry, tag: &str) -> Geometry {
    filter_tag(geometry, tag)
}

/// `combineWith`.
#[must_use]
pub fn combine_with(geometry: Geometry, other: Geometry) -> Geometry {
    match geometry {
        Geometry::Combine(mut parts) => {
            parts.push(other);
            Geometry::Combine(parts)
        }
        first => Geometry::Combine(vec![first, other]),
    }
}

/// `filterTag`: drop tagged geometry, keeping the rest.
///
/// The tag's value is not modelled, so this drops every tagged node rather than
/// the ones carrying one particular tag. Definitions that eject a tag use one at
/// a time, which this handles; mixing several would not be distinguished.
fn filter_tag(geometry: &Geometry, tag: &str) -> Geometry {
    let _ = tag;
    match geometry {
        Geometry::Tagged(_) => Geometry::Invalid,
        Geometry::Combine(parts) => {
            Geometry::Combine(parts.iter().map(|part| filter_tag(part, tag)).collect())
        }
        Geometry::Transformed { tf, geom } => Geometry::Transformed {
            tf: *tf,
            geom: Box::new(filter_tag(geom, tag)),
        },
        other => other.clone(),
    }
}

impl Component for Glyph {
    fn apply_to_glyph(&self, glyph: &mut Glyph, copy_anchors: bool, copy_width: bool) {
        glyph.include_glyph(self, copy_anchors, copy_width);
    }
}

/// A proc that is applied when the glyph is built.
#[derive(Clone)]
pub struct GlyphProc {
    operations: Vec<Operation>,
}

/// One step of a proc.
#[derive(Clone)]
pub enum Operation {
    IncludeGeometry(Geometry),
    IncludeProc(Box<GlyphProc>),
    IncludeGlyph(Box<Glyph>),
    ApplyTransform(Affine),
    SetWidth(f64),
    ClearGeometry,
    /// Set the context tag every later inclusion is wrapped in.
    CtxTag(String),
    EjectTagged(String),
    Boole {
        operator: BinaryOp,
        operands: Vec<Glyph>,
    },
}

impl GlyphProc {
    #[must_use]
    pub const fn new(operations: Vec<Operation>) -> Self {
        Self { operations }
    }

    #[must_use]
    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }
}

impl Component for GlyphProc {
    fn apply_to_glyph(&self, glyph: &mut Glyph, copy_anchors: bool, copy_width: bool) {
        for operation in &self.operations {
            match operation {
                Operation::IncludeGeometry(geometry) => {
                    glyph.include_geometry(geometry.clone());
                }
                Operation::IncludeProc(proc) => {
                    glyph.include(proc.as_ref(), copy_anchors, copy_width);
                }
                Operation::IncludeGlyph(other) => {
                    glyph.include(other.as_ref(), copy_anchors, copy_width);
                }
                Operation::ApplyTransform(tf) => glyph.apply_transform(*tf),
                Operation::SetWidth(width) => glyph.set_width(*width),
                Operation::ClearGeometry => glyph.clear_geometry(),
                Operation::CtxTag(tag) => glyph.ctx_tag = Some(tag.clone()),
                Operation::EjectTagged(tag) => glyph.eject_tagged(tag),
                Operation::Boole { operator, operands } => {
                    let forward = glyph.gizmo;
                    let backward = forward.inverse();
                    let placed: Vec<Geometry> = operands
                        .iter()
                        .map(|operand| {
                            let mut temp = operand.clone();
                            temp.gizmo = forward;

                            Geometry::Transformed {
                                tf: backward,
                                geom: Box::new(temp.geometry),
                            }
                        })
                        .collect();
                    glyph.include_geometry(Geometry::Transformed {
                        tf: forward,
                        geom: Box::new(Geometry::Boolean {
                            op: *operator,
                            operands: placed,
                        }),
                    });
                }
            }
        }
    }
}

/// `ForkGlyphProc`: start from one glyph, then add another.
#[derive(Clone)]
pub struct ForkGlyphProc {
    from: Box<Glyph>,
    component: Option<Box<GlyphProc>>,
}

impl ForkGlyphProc {
    #[must_use]
    pub fn new(from: Glyph, component: Option<GlyphProc>) -> Self {
        Self {
            from: Box::new(from),
            component: component.map(Box::new),
        }
    }
}

impl Component for ForkGlyphProc {
    fn apply_to_glyph(&self, glyph: &mut Glyph, _copy_anchors: bool, _copy_width: bool) {
        glyph.include(self.from.as_ref(), true, true);
        if let Some(component) = &self.component {
            glyph.include(component.as_ref(), true, true);
        }
    }
}

/// `union`: a boolean union of the operands.
#[must_use]
pub const fn union(operands: Vec<Geometry>) -> Geometry {
    Geometry::Boolean {
        op: BinaryOp::Union,
        operands,
    }
}

/// `intersection`: a boolean intersection of the operands.
#[must_use]
pub const fn intersection(operands: Vec<Geometry>) -> Geometry {
    Geometry::Boolean {
        op: BinaryOp::Intersection,
        operands,
    }
}

/// `difference`: the first operand less the rest.
#[must_use]
pub const fn difference(operands: Vec<Geometry>) -> Geometry {
    Geometry::Boolean {
        op: BinaryOp::Difference,
        operands,
    }
}
