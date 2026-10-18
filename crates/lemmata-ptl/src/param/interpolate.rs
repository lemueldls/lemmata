//! `monotonic-interpolate`, from `@iosevka/util`.
//!
//! A monotone cubic Hermite interpolant, Fritsch and Carlson. It is what the
//! parameter data's `blend` blocks are read through: a hive can give a value
//! per grade, and the style's grade selects one by interpolating between them.
//!
//! `xs`, `ys` and the names built from them are upstream's, and the arithmetic
//! is written to match it: fused multiply-add rounds differently, and the
//! rightmost-point test is an exact equality on purpose.
#![allow(clippy::similar_names, clippy::suboptimal_flops, clippy::float_cmp)]

/// Builds the interpolant for a set of points.
///
/// The points are sorted by `x` first, and a single point returns a constant.
///
/// # Panics
///
/// Panics when the two slices differ in length, which is a programming error
/// rather than an input one.
pub fn monotonic_interpolate(xs: &[f64], ys: &[f64]) -> Box<dyn Fn(f64) -> f64> {
    assert_eq!(xs.len(), ys.len(), "need an equal count of xs and ys");
    let length = xs.len();
    if length == 0 {
        return Box::new(|_| 0.0);
    }
    if length == 1 {
        let only = ys[0];
        return Box::new(move |_| only);
    }

    let mut order: Vec<usize> = (0..length).collect();
    order.sort_by(|a, b| {
        xs[*a]
            .partial_cmp(&xs[*b])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let sorted_xs: Vec<f64> = order.iter().map(|i| xs[*i]).collect();
    let sorted_ys: Vec<f64> = order.iter().map(|i| ys[*i]).collect();

    let mut dxs = Vec::with_capacity(length - 1);
    let mut ms = Vec::with_capacity(length - 1);
    for i in 0..length - 1 {
        let dx = sorted_xs[i + 1] - sorted_xs[i];
        let dy = sorted_ys[i + 1] - sorted_ys[i];
        dxs.push(dx);
        ms.push(dy / dx);
    }

    let mut c1s = vec![ms[0]];
    for i in 0..dxs.len() - 1 {
        let (m, m_next) = (ms[i], ms[i + 1]);
        if m * m_next <= 0.0 {
            c1s.push(0.0);
        } else {
            let (dx, dx_next) = (dxs[i], dxs[i + 1]);
            let common = dx + dx_next;
            c1s.push((3.0 * common) / ((common + dx_next) / m + (common + dx) / m_next));
        }
    }
    c1s.push(ms[ms.len() - 1]);

    let mut c2s = Vec::with_capacity(c1s.len() - 1);
    let mut c3s = Vec::with_capacity(c1s.len() - 1);
    for i in 0..c1s.len() - 1 {
        let c1 = c1s[i];
        let m = ms[i];
        let inv_dx = 1.0 / dxs[i];
        let common = c1 + c1s[i + 1] - m - m;
        c2s.push((m - c1 - common) * inv_dx);
        c3s.push(common * inv_dx * inv_dx);
    }

    Box::new(move |x: f64| {
        // The rightmost point gives an exact result.
        let last = sorted_xs.len() - 1;
        if x == sorted_xs[last] {
            return sorted_ys[last];
        }
        let mut low = 0usize;
        let mut high = c3s.len() - 1;
        while low <= high {
            let mid = low.midpoint(high);
            let here = sorted_xs[mid];
            if here < x {
                low = mid + 1;
            } else if here > x {
                if mid == 0 {
                    break;
                }
                high = mid - 1;
            } else {
                return sorted_ys[mid];
            }
        }
        let diff = x - sorted_xs[high];
        let diff_sq = diff * diff;
        sorted_ys[high] + c1s[high] * diff + c2s[high] * diff_sq + c3s[high] * diff * diff_sq
    })
}
