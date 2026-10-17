//! Boolean operations on closed paths.
//!
//! Linesweeper does the sweep-line work on kurbo paths. Iosevka's typo-geom does
//! the same job on an integer grid, so results from the two agree to within that
//! grid's resolution rather than exactly.

use kurbo::BezPath;
use linesweeper::{BinaryOp, Error as SweepError, FillRule, binary_op};

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
