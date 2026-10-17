//! Boolean operations on closed paths.
//!
//! Linesweeper does the sweep-line work on kurbo paths. Iosevka's typo-geom does
//! the same job on an integer grid, so results from the two agree to within that
//! grid's resolution rather than exactly.

#![allow(clippy::cast_precision_loss, clippy::suboptimal_flops)]

use kurbo::BezPath;
use linesweeper::{BinaryOp, Error as SweepError, FillRule, binary_op};

use crate::stroke::Bez3;

#[derive(Debug)]
pub enum Error {
    Infinity,
    NaN,
    NonClosed,
}

impl From<SweepError> for Error {
    fn from(error: SweepError) -> Self {
        match error {
            SweepError::Infinity => Self::Infinity,
            SweepError::NaN => Self::NaN,
            SweepError::NonClosedPath(_) => Self::NonClosed,
        }
    }
}

/// Drops contours that enclose no area.
///
/// A zero-area operand contributes nothing to a boolean, and feeding one to the
/// sweep can leave a zero-width seam in the result where the other engine drops
/// it. Glyph definitions produce them: a difference whose right side collapses,
/// for instance.
#[must_use]
pub fn drop_degenerate(shape: &[Vec<Bez3>]) -> Vec<Vec<Bez3>> {
    use kurbo::Shape;
    shape
        .iter()
        .filter(|contour| {
            if contour.len() < 2 {
                return false;
            }
            let paths = crate::stroke::shape_to_paths(std::slice::from_ref(*contour));
            let area: f64 = paths.iter().map(BezPath::area).sum();
            area.abs() > DEGENERATE_AREA
        })
        .cloned()
        .collect()
}

/// Contours below this area, in square font units, count as degenerate.
const DEGENERATE_AREA: f64 = 1e-6;

/// Two points within this distance count as the same point.
const POINT_EPSILON: f64 = 1e-9;
/// Arcs whose control points all sit within this distance of the start are
/// numerical debris from the sweep, not geometry. Upstream's rebuild never
/// emits them.
const DEGENERATE_EXTENT: f64 = 1e-7;
/// An arc whose ends are this close is a spike candidate. The threshold is
/// looser than `POINT_EPSILON` because a detour out and back can return a hair
/// away from where it left, and the chord is then too short to give a stable
/// direction.
const SPIKE_ENDPOINT_EPSILON: f64 = 1e-6;

/// Removes the degenerate arcs a sweep can leave behind.
///
/// Two kinds turn up. A zero-length arc has all four control points in one
/// place, and a collinear excursion runs out along a line and back without
/// enclosing anything. Neither changes the filled region, but both show up in
/// the boundary, and typo-geom's rebuild never emits them because it only
/// places arcs it can find in its segment table.
#[must_use]
pub fn clean_shape(shape: &[Vec<Bez3>]) -> Vec<Vec<Bez3>> {
    shape
        .iter()
        .map(|contour| {
            let mut cleaned: Vec<Bez3> = Vec::with_capacity(contour.len());
            for arc in contour {
                if extent(arc) <= DEGENERATE_EXTENT {
                    continue;
                }
                // An arc that comes back to where it started without enclosing
                // anything is a spike: a detour the sweep left behind.
                if (arc.a - arc.d).hypot() <= SPIKE_ENDPOINT_EPSILON
                    && closed_area(arc).abs() <= DEGENERATE_AREA
                {
                    continue;
                }
                let arc = if is_collinear_excursion(arc) {
                    straight(arc.a, arc.d)
                } else {
                    *arc
                };
                // A zero-length arc can also appear as a repeated point with
                // stray controls; drop it once it has no extent left.
                if let Some(previous) = cleaned.last()
                    && close(previous.d, arc.a)
                    && close(arc.a, arc.d)
                {
                    continue;
                }
                cleaned.push(arc);
            }
            cleaned
        })
        .filter(|contour| contour.len() > 1)
        .collect()
}

/// How far the arc's control points reach from its start.
fn extent(arc: &Bez3) -> f64 {
    (arc.b - arc.a)
        .hypot()
        .max((arc.c - arc.a).hypot())
        .max((arc.d - arc.a).hypot())
}

/// Signed area enclosed by the arc and the chord back to its start.
fn closed_area(arc: &Bez3) -> f64 {
    const STEPS: usize = 16;
    let mut area = 0.0;
    let mut previous = arc.eval(0.0);
    let first = previous;
    for step in 1..=STEPS {
        let point = arc.eval(step as f64 / STEPS as f64);
        area += previous.x * point.y - point.x * previous.y;
        previous = point;
    }
    area += previous.x * first.y - first.x * previous.y;
    area / 2.0
}

fn close(a: kurbo::Point, b: kurbo::Point) -> bool {
    (a - b).hypot() <= POINT_EPSILON
}

/// True when every control point sits on the line through the endpoints but the
/// curve leaves the span between them on the way.
fn is_collinear_excursion(arc: &Bez3) -> bool {
    let along = arc.d - arc.a;
    let length = along.hypot();
    if length <= SPIKE_ENDPOINT_EPSILON {
        return false;
    }
    let direction = along / length;
    let project = |p: kurbo::Point| (p - arc.a).dot(direction);
    let on_line = |p: kurbo::Point| ((p - arc.a) - direction * project(p)).hypot() <= POINT_EPSILON;
    if !(on_line(arc.b) && on_line(arc.c)) {
        return false;
    }
    let span = [project(arc.b), project(arc.c)];
    span.iter()
        .any(|value| *value < -POINT_EPSILON || *value > length + POINT_EPSILON)
}

fn straight(a: kurbo::Point, d: kurbo::Point) -> Bez3 {
    Bez3::new(a, a + (d - a) / 3.0, a + (d - a) * (2.0 / 3.0), d)
}

/// Applies a boolean operation to two shapes, both filled by the non-zero rule.
pub fn boolean(a: &BezPath, b: &BezPath, op: BinaryOp) -> Result<Vec<BezPath>, Error> {
    let contours = binary_op(a, b, FillRule::NonZero, op)?;
    Ok(contours
        .into_contours()
        // typo-geom's clipper leaves outer contours clockwise; linesweeper
        // leaves them counter-clockwise. Downstream unions mix these contours
        // with ones from other constructors under the non-zero rule, so the
        // orientation has to match or overlapping regions cancel.
        .map(|contour| contour.path.reverse_subpaths())
        .collect())
}

/// Resolves self-intersections and overlapping contours into disjoint contours.
pub fn remove_overlap(shape: &BezPath) -> Result<Vec<BezPath>, Error> {
    boolean(shape, &BezPath::new(), BinaryOp::Union)
}

/// Union of two shapes.
pub fn union(a: &BezPath, b: &BezPath) -> Result<Vec<BezPath>, Error> {
    boolean(a, b, BinaryOp::Union)
}

/// Intersection of two shapes.
pub fn intersection(a: &BezPath, b: &BezPath) -> Result<Vec<BezPath>, Error> {
    boolean(a, b, BinaryOp::Intersection)
}

#[cfg(test)]
mod tests {
    use super::{intersection, remove_overlap, union};
    use kurbo::{BezPath, Rect, Shape};

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> BezPath {
        Rect::new(x0, y0, x1, y1).to_path(0.1)
    }

    #[test]
    fn disjoint_squares_stay_separate() {
        let contours = union(
            &rect(0.0, 0.0, 100.0, 100.0),
            &rect(200.0, 0.0, 300.0, 100.0),
        )
        .expect("union");
        assert_eq!(contours.len(), 2);
    }

    #[test]
    fn overlapping_squares_merge() {
        let contours = union(
            &rect(0.0, 0.0, 100.0, 100.0),
            &rect(50.0, 0.0, 150.0, 100.0),
        )
        .expect("union");
        assert_eq!(contours.len(), 1);
        let area: f64 = contours.iter().map(BezPath::area).sum();
        assert!((area.abs() - 15_000.0).abs() < 1.0, "area {area}");
        assert!(area < 0.0, "outer contours come out clockwise");
    }

    #[test]
    fn intersection_is_the_overlap() {
        let contours = intersection(
            &rect(0.0, 0.0, 100.0, 100.0),
            &rect(50.0, 0.0, 150.0, 100.0),
        )
        .expect("intersection");
        assert_eq!(contours.len(), 1);
        let area: f64 = contours.iter().map(BezPath::area).sum();
        assert!((area.abs() - 5_000.0).abs() < 1.0, "area {area}");
    }

    #[test]
    fn remove_overlap_keeps_a_single_square() {
        let contours = remove_overlap(&rect(0.0, 0.0, 100.0, 100.0)).expect("remove overlap");
        assert_eq!(contours.len(), 1);
        let area: f64 = contours.iter().map(BezPath::area).sum();
        assert!((area.abs() - 10_000.0).abs() < 1.0, "area {area}");
    }

    #[test]
    fn remove_overlap_folds_two_squares_in_one_path() {
        let mut shape = rect(0.0, 0.0, 100.0, 100.0);
        shape.extend(rect(50.0, 0.0, 150.0, 100.0).iter());
        let contours = remove_overlap(&shape).expect("remove overlap");
        assert_eq!(contours.len(), 1);
        let area: f64 = contours.iter().map(BezPath::area).sum();
        assert!((area.abs() - 15_000.0).abs() < 1.0, "area {area}");
    }
}
