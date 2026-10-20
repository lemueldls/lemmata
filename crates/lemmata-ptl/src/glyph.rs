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

    /// `includeGlyph` at an explicit shift, which is how a mark is placed.
    pub fn include_glyph_at(&mut self, other: &Self, shift: Vec2) {
        self.include_glyph_impl(other, shift);
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

// `mark_shift` would be const if iterator methods were.
#[allow(clippy::missing_const_for_fn)]
/// `combineMarks`: the translation that lands a mark on a base.
///
/// The first anchor class the two share decides it: the base's anchor minus the
/// mark's. `None` when they share none, which upstream leaves where it is.
#[must_use]
pub fn mark_shift(base: &Glyph, mark: &Glyph) -> Option<Vec2> {
    for (name, mark_anchor) in &mark.mark_anchors {
        if let Some((_, base_anchor)) = base.base_anchors.iter().find(|(other, _)| other == name) {
            return Some(*base_anchor - *mark_anchor);
        }
    }

    None
}

/// A base with its marks placed on it.
///
/// This is the placement half of an accented composition: the base is included,
/// then each mark at the shift its anchors name. Which parts a composition has,
/// and which mark replaces which, is the definition's business and lives in the
/// glyph source.
#[derive(Clone, Debug)]
pub struct Composite {
    pub base: Glyph,
    pub marks: Vec<Glyph>,
}

impl Composite {
    #[must_use]
    pub const fn new(base: Glyph, marks: Vec<Glyph>) -> Self {
        Self { base, marks }
    }
}

impl Component for Composite {
    fn apply_to_glyph(&self, glyph: &mut Glyph, copy_anchors: bool, copy_width: bool) {
        glyph.include_glyph(&self.base, copy_anchors, copy_width);
        for mark in &self.marks {
            let shift = mark_shift(glyph, mark).unwrap_or_default();
            glyph.include_glyph_at(mark, shift);
        }
    }
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

#[cfg(test)]
mod compose_tests {
    use kurbo::Vec2;

    use super::{Composite, Glyph, mark_shift};

    fn anchored(base: &[(&str, Vec2)], mark: &[(&str, Vec2)]) -> (Glyph, Glyph) {
        let mut a = Glyph::new();
        a.base_anchors = base.iter().map(|(k, v)| ((*k).to_string(), *v)).collect();
        let mut b = Glyph::new();
        b.mark_anchors = mark.iter().map(|(k, v)| ((*k).to_string(), *v)).collect();

        (a, b)
    }

    #[test]
    fn a_shared_anchor_names_the_shift() {
        let (base, mark) = anchored(
            &[("above", Vec2::new(100.0, 500.0))],
            &[("above", Vec2::new(100.0, 700.0))],
        );
        assert_eq!(mark_shift(&base, &mark), Some(Vec2::new(0.0, -200.0)));
    }

    #[test]
    fn no_shared_anchor_leaves_the_mark_alone() {
        let (base, mark) = anchored(
            &[("below", Vec2::new(0.0, 0.0))],
            &[("above", Vec2::new(0.0, 700.0))],
        );
        assert_eq!(mark_shift(&base, &mark), None);
    }

    #[test]
    fn a_composite_places_each_mark() {
        let (base, mark) = anchored(
            &[("above", Vec2::new(250.0, 520.0))],
            &[("above", Vec2::new(250.0, 700.0))],
        );
        let composite = Composite::new(base, vec![mark]);
        let mut glyph = Glyph::new();
        glyph.include(&composite, true, true);

        let lemmata_geom::Geometry::Combine(parts) = &glyph.geometry else {
            panic!("a composite is a combine");
        };
        assert_eq!(parts.len(), 2);
        let lemmata_geom::Geometry::Transformed { tf, .. } = &parts[1] else {
            panic!("the mark is placed by a transform");
        };
        assert_eq!(tf.as_coeffs()[5], -180.0);
    }
}
