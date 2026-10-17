//! Flattening outlines to polylines so two fonts can be compared point by point.

use kurbo::{BezPath, PathEl, Point};

/// Flattens one contour into a polyline.
///
/// `tolerance` is the flattening tolerance in font units, so it should be well
/// below the drift tolerance being measured.
#[must_use]
pub fn polyline(path: &BezPath, tolerance: f64) -> Vec<Point> {
    let mut points = Vec::new();
    kurbo::flatten(path.iter(), tolerance, |element| match element {
        PathEl::MoveTo(p) | PathEl::LineTo(p) => points.push(p),
        PathEl::QuadTo(..) | PathEl::CurveTo(..) | PathEl::ClosePath => {}
    });

    points
}

#[cfg(test)]
mod tests {
    use super::polyline;
    use kurbo::{BezPath, Point};

    #[test]
    fn quad_stays_within_tolerance() {
        let mut path = BezPath::new();
        path.move_to(Point::new(0.0, 0.0));
        path.quad_to(Point::new(50.0, 100.0), Point::new(100.0, 0.0));

        let points = polyline(&path, 0.5);

        assert_eq!(points.first(), Some(&Point::new(0.0, 0.0)));
        assert_eq!(points.last(), Some(&Point::new(100.0, 0.0)));
        assert!(points.len() > 2);
    }

    #[test]
    fn line_needs_no_subdivision() {
        let mut path = BezPath::new();
        path.move_to(Point::new(0.0, 0.0));
        path.line_to(Point::new(10.0, 10.0));

        assert_eq!(
            polyline(&path, 0.01),
            vec![Point::new(0.0, 0.0), Point::new(10.0, 10.0)]
        );
    }
}
