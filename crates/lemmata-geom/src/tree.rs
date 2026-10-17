//! The geometry tree: the classes in `index.mjs` that compose the primitives.
//!
//! `SpiroGeometry`, `DiSpiroGeometry`, `SpiroPenGeometry`, `StrokeGeometry`,
//! `RemoveHolesGeometry`, `SimplifyGeometry`, `CombineGeometry`,
//! `BooleanGeometry`, `TransformedGeometry`, `TaggedGeometry`,
//! `RadicalGeometry`, `ContourSetGeometry` and `InvalidGeometry`, with the
//! `toBezArcs` and `toContours` entry points glyph code uses.

#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::needless_range_loop,
    clippy::similar_names,
    clippy::suboptimal_flops
)]

use kurbo::{Affine, BezPath, Vec2};
use linesweeper::BinaryOp;

use crate::boolean::remove_overlap;
use crate::expand::{BiKnot, MonoKnot, di_spiro_expand};
use crate::fairize::{FairArc, fairize};
use crate::outline::{self, transform_bez3};
use crate::pen::{PenKnot, pen_spiro_paths};
use crate::quadify::{QuadifyPoint, QuadifySink};
use crate::spiro::KnotType;
use crate::stroke::{Bez3, boolean_shapes, paths_to_shape, shape_to_paths, stroke_arcs};

/// Geometry that is not empty.
pub const NON_EMPTY: u8 = 0x01;
/// Geometry that contains non-simple contours.
pub const NON_SIMPLE: u8 = 0x02;
/// Geometry that contains broken contours.
pub const BROKEN: u8 = 0x04;
/// Geometry whose complexity is unknown.
pub const UNKNOWN: u8 = 0xff;

/// Fit tolerance, matching `GEOMETRY_PRECISION` in `curve-util.mjs`.
const GEOMETRY_PRECISION: f64 = 0.25;

/// A node of the geometry tree.
#[derive(Clone, Debug)]
pub enum Geometry {
    /// Already-cubic contours.
    Contours(Vec<Vec<Bez3>>),
    /// A spiro skeleton.
    Spiro {
        gizmo: Affine,
        knots: Vec<MonoKnot>,
        closed: bool,
    },
    /// A contrast-expanded skeleton.
    DiSpiro {
        gizmo: Affine,
        contrast: f64,
        closed: bool,
        knots: Vec<BiKnot>,
    },
    /// A pen profile swept along a skeleton.
    SpiroPen {
        gizmo: Affine,
        pen_profile: Vec<Vec2>,
        closed: bool,
        knots: Vec<PenKnot>,
    },
    /// Offset every contour of `geom`.
    Stroke {
        gizmo: Affine,
        radius: f64,
        contrast: f64,
        inside: bool,
        geom: Box<Geometry>,
    },
    /// Resolve overlaps in `geom`.
    RemoveHoles { gizmo: Affine, geom: Box<Geometry> },
    /// Overlap resolution and quadratic conversion.
    Simplify { gizmo: Affine, geom: Box<Geometry> },
    /// Concatenate the parts' arcs.
    Combine(Vec<Geometry>),
    /// A boolean operation across the operands.
    Boolean {
        op: BinaryOp,
        operands: Vec<Geometry>,
    },
    /// Apply a transform.
    Transformed { tf: Affine, geom: Box<Geometry> },
    /// A tag that does not affect the geometry.
    Tagged(Box<Geometry>),
    /// A reference to another geometry that does not affect the geometry.
    Radical(Box<Geometry>),
    /// Geometry that could not be built.
    Invalid,
}

impl Geometry {
    /// The `toBezArcs` entry point.
    #[must_use]
    pub fn to_bez_arcs(&self) -> Vec<Vec<Bez3>> {
        match self {
            Self::Contours(contours) => contours.clone(),
            Self::Spiro {
                gizmo,
                knots,
                closed,
            } => outline::spiro_to_bez_arcs(knots, *closed, *gizmo),
            Self::DiSpiro {
                gizmo,
                contrast,
                closed,
                knots,
            } => di_spiro_arcs(*gizmo, *contrast, *closed, knots),
            Self::SpiroPen {
                gizmo,
                pen_profile,
                closed,
                knots,
            } => union_all_contours(&paths_to_shape(&pen_spiro_paths(
                *gizmo,
                pen_profile.len(),
                *closed,
                knots,
            ))),
            Self::Stroke {
                gizmo,
                radius,
                contrast,
                inside,
                geom,
            } => {
                let inner = transformed(*gizmo, geom).to_bez_arcs();
                let arcs = resolve_overlaps(&inner);
                let fairized = fairize(&arcs);
                let out = stroke_arcs(&fairized, *radius, *contrast, *inside);

                transform_shape(&out, *gizmo)
            }
            Self::RemoveHoles { gizmo, geom } => {
                let inner = transformed(*gizmo, geom).to_bez_arcs();
                let arcs = union_all_contours(&resolve_overlaps(&inner));

                transform_shape(&arcs, *gizmo)
            }
            Self::Simplify { gizmo, geom } => simplify_arcs(*gizmo, geom),
            Self::Combine(parts) => parts.iter().flat_map(Self::to_bez_arcs).collect(),
            Self::Boolean { op, operands } => {
                let mut result: Vec<Vec<Bez3>> = Vec::new();
                for (index, operand) in operands.iter().enumerate() {
                    let arcs = operand.to_bez_arcs();
                    result = if index == 0 {
                        arcs
                    } else {
                        boolean_shapes(&result, &arcs, *op)
                    };
                }

                result
            }
            Self::Transformed { tf, geom } => transform_shape(&geom.to_bez_arcs(), *tf),
            Self::Tagged(geom) | Self::Radical(geom) => geom.to_bez_arcs(),
            Self::Invalid => Vec::new(),
        }
    }

    /// The `toContours` entry point. Fairize, then quadratify.
    #[must_use]
    pub fn to_contours(&self) -> Vec<Vec<QuadifyPoint>> {
        let arcs = self.to_bez_arcs();
        let fairized = fairize(&arcs);

        quadify_fair(&fairized)
    }

    /// Upstream's complexity flags, used to decide whether to resolve overlaps.
    #[must_use]
    pub fn measure_complexity(&self) -> u8 {
        match self {
            Self::Contours(contours) => {
                let mut complexity = if contours.is_empty() { 0 } else { NON_EMPTY };
                for contour in contours {
                    for arc in contour {
                        for point in [arc.a, arc.b, arc.c, arc.d] {
                            if !point.x.is_finite() || !point.y.is_finite() {
                                complexity |= BROKEN;
                            }
                        }
                    }
                }

                complexity
            }
            Self::Spiro { knots, .. } => {
                let mut complexity = NON_EMPTY | NON_SIMPLE;
                for knot in knots {
                    if !knot.x.is_finite() || !knot.y.is_finite() {
                        complexity |= BROKEN;
                    }
                }

                complexity
            }
            Self::DiSpiro { .. } | Self::SpiroPen { .. } => NON_EMPTY | NON_SIMPLE,
            Self::Stroke { geom, .. } | Self::RemoveHoles { geom, .. } => {
                geom.measure_complexity() | NON_SIMPLE
            }
            Self::Simplify { geom, .. } => geom.measure_complexity(),
            Self::Combine(parts) => parts
                .iter()
                .fold(0, |acc, part| acc | part.measure_complexity()),
            Self::Boolean { operands, .. } => operands
                .iter()
                .fold(NON_SIMPLE, |acc, part| acc | part.measure_complexity()),
            Self::Transformed { tf, geom } => {
                let flags = if is_positive(*tf) { 0 } else { NON_SIMPLE };
                flags | geom.measure_complexity()
            }
            Self::Tagged(geom) | Self::Radical(geom) => geom.measure_complexity(),
            Self::Invalid => UNKNOWN,
        }
    }
}

fn transformed(gizmo: Affine, geom: &Geometry) -> Geometry {
    Geometry::Transformed {
        tf: gizmo.inverse(),
        geom: Box::new(geom.clone()),
    }
}

fn simplify_arcs(gizmo: Affine, geom: &Geometry) -> Vec<Vec<Bez3>> {
    let mut arcs = geom.to_bez_arcs();
    let needs_transform = !is_translate(gizmo);
    if needs_transform {
        arcs = transform_shape(&arcs, gizmo.inverse());
    }
    if geom.measure_complexity() & NON_SIMPLE != 0 {
        arcs = resolve_overlaps(&arcs);
    }
    if needs_transform {
        arcs = transform_shape(&arcs, gizmo);
    }

    arcs
}

fn di_spiro_arcs(gizmo: Affine, contrast: f64, closed: bool, knots: &[BiKnot]) -> Vec<Vec<Bez3>> {
    let expanded = di_spiro_expand(gizmo, contrast, closed, knots);
    let mut lhs = expanded.lhs_untransformed;
    let mut rhs = expanded.rhs_untransformed;
    for knot in &mut rhs {
        knot.reverse_type();
    }
    rhs.reverse();

    if closed {
        let mut arcs = outline::spiro_to_bez_arcs(&lhs, true, gizmo);
        arcs.extend(outline::spiro_to_bez_arcs(&rhs, true, gizmo));

        arcs
    } else {
        if !lhs.is_empty() {
            let last = lhs.len() - 1;
            lhs[0].ty = KnotType::Corner;
            lhs[last].ty = KnotType::Corner;
        }
        if !rhs.is_empty() {
            let last = rhs.len() - 1;
            rhs[0].ty = KnotType::Corner;
            rhs[last].ty = KnotType::Corner;
        }
        let all: Vec<MonoKnot> = lhs.into_iter().chain(rhs).collect();

        outline::spiro_to_bez_arcs(&all, true, gizmo)
    }
}

/// Unions every contour into one region, filling holes. That is what
/// `RemoveHolesGeometry` does after resolving overlaps.
fn union_all_contours(shape: &[Vec<Bez3>]) -> Vec<Vec<Bez3>> {
    let mut contours = crate::boolean::drop_degenerate(shape).into_iter();
    let Some(first) = contours.next() else {
        return Vec::new();
    };
    let mut accumulated = vec![first];
    for contour in contours {
        accumulated = boolean_shapes(
            &accumulated,
            std::slice::from_ref(&contour),
            BinaryOp::Union,
        );
    }

    accumulated
}

fn resolve_overlaps(shape: &[Vec<Bez3>]) -> Vec<Vec<Bez3>> {
    let shape = crate::boolean::drop_degenerate(shape);
    let mut path = BezPath::new();
    for contour in shape_to_paths(&shape) {
        path.extend(contour.iter());
    }

    match remove_overlap(&path) {
        Ok(contours) => crate::boolean::annex(
            &crate::boolean::clean_shape(&paths_to_shape(&contours)),
            &shape,
        ),
        Err(_) => shape,
    }
}

fn transform_shape(shape: &[Vec<Bez3>], tf: Affine) -> Vec<Vec<Bez3>> {
    shape
        .iter()
        .map(|contour| {
            contour
                .iter()
                .map(|segment| transform_bez3(tf, *segment))
                .collect()
        })
        .collect()
}

fn is_translate(tf: Affine) -> bool {
    let c = tf.as_coeffs();
    c[0] == 1.0 && c[1] == 0.0 && c[2] == 0.0 && c[3] == 1.0
}

fn is_positive(tf: Affine) -> bool {
    let c = tf.as_coeffs();
    c[0] * c[3] - c[2] * c[1] > 0.0
}

/// `transferGenericShape(fairizeBezierShape(shape), sink, GEOMETRY_PRECISION)`.
///
/// Straight runs become lines; combined arcs go through the quadratic fitter.
#[must_use]
pub fn quadify_fair(shape: &[Vec<FairArc>]) -> Vec<Vec<QuadifyPoint>> {
    let mut sink = QuadifySink::new();
    sink.begin_shape();
    for contour in shape {
        let Some(first) = contour.first() else {
            continue;
        };
        let start = first.eval(0.0);
        let mut current = start;
        sink.move_to(start.x, start.y);

        for arc in contour {
            let a = arc.eval(0.0);
            let d = arc.eval(1.0);
            if !points_close(a, current, GEOMETRY_PRECISION) {
                sink.line_to(a.x, a.y);
                current = a;
            }

            if matches!(arc, FairArc::Straight { .. }) {
                if !points_close(d, current, GEOMETRY_PRECISION) {
                    sink.line_to(d.x, d.y);
                    current = d;
                }
            } else {
                sink.arc_to(arc, d.x, d.y);
                current = d;
            }
        }
    }
    sink.end_shape();

    sink.into_contours()
}

fn points_close(a: kurbo::Point, b: kurbo::Point, tolerance: f64) -> bool {
    let dx = a.x - b.x;
    let dy = a.y - b.y;

    dx * dx + dy * dy < tolerance * tolerance
}
