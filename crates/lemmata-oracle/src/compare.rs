//! Measuring how far two flattened outlines are apart.
//!
//! r[impl drift.outline]

use kurbo::Point;

/// Largest distance from a point of `a` to the nearest point of `b`.
///
/// Returns infinity when `a` has points and `b` has none, so a contour that
/// vanished in one font cannot pass silently.
#[must_use]
pub fn directed_deviation(a: &[Point], b: &[Point]) -> f64 {
    let mut worst: f64 = 0.0;
    for p in a {
        let mut nearest = f64::INFINITY;
        for q in b {
            let distance = (*p - *q).hypot();
            if distance < nearest {
                nearest = distance;
            }
        }

        if nearest > worst {
            worst = nearest;
        }
    }

    worst
}

/// Both directions of [`directed_deviation`], so an extra point counts too.
#[must_use]
pub fn max_deviation(a: &[Point], b: &[Point]) -> f64 {
    directed_deviation(a, b).max(directed_deviation(b, a))
}

#[cfg(test)]
mod tests {
    use super::{directed_deviation, max_deviation};
    use kurbo::Point;

    #[test]
    fn identical_polylines_have_no_deviation() {
        let line: Vec<Point> = (0..10).map(|i| Point::new(f64::from(i), 0.0)).collect();
        assert_eq!(max_deviation(&line, &line), 0.0);
    }

    #[test]
    fn a_shift_shows_up_in_one_direction_only() {
        let a = vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)];
        let b = vec![Point::new(0.0, 3.0), Point::new(10.0, 3.0)];

        assert_eq!(directed_deviation(&a, &b), 3.0);
        assert_eq!(max_deviation(&a, &b), 3.0);
    }

    #[test]
    fn an_empty_side_is_infinite() {
        let a = vec![Point::new(0.0, 0.0)];
        assert!(max_deviation(&a, &[]).is_infinite());
        assert_eq!(max_deviation(&[], &[]), 0.0);
    }
}
