//! Fairization: corner classification, splitting at extrema, and straight-run
//! reduction.
//!
//! Ported from typo-geom's `bez-tool/fairize` and the pieces it uses
//! (`Bez3Slice`, `splitAtExtrema`, `CombinedArc`, the quadratic solver and the
//! 16-point Gauss-Legendre tables). `StrokeGeometry` and `SimplifyGeometry`
//! apply this before stroke expansion and quadratic conversion.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::float_cmp,
    clippy::imprecise_flops,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::similar_names,
    clippy::suboptimal_flops,
    clippy::suspicious_operation_groupings,
    clippy::too_many_arguments,
    clippy::unreadable_literal
)]

use kurbo::{Point, Vec2};

use crate::fit::CurveArc;
use crate::math;
use crate::stroke::Bez3;

const EPSILON: f64 = 1e-12;
const MACHINE_EPSILON: f64 = 1.12e-16;
const CURVE_TIME_EPSILON: f64 = 1e-8;
const GEOMETRIC_EPSILON: f64 = 1e-7;

/// How two adjacent curves meet at a point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CornerType {
    Smooth,
    Corner,
    Extrema,
    Hetero,
}

/// One piece of a contour, with the corner types on either side.
#[derive(Clone, Copy, Debug)]
struct Slice {
    cubic: Bez3,
    corner_before: CornerType,
    corner_after: CornerType,
}

/// One fairized arc: a straight run, or a chain of cubics.
#[derive(Clone, Debug)]
pub enum FairArc {
    Straight { a: Point, b: Point },
    Combined(CombinedArc),
}

impl FairArc {
    #[must_use]
    pub fn eval(&self, t: f64) -> Point {
        match self {
            Self::Straight { a, b } => Point::new(mix(a.x, b.x, t), mix(a.y, b.y, t)),
            Self::Combined(combined) => combined.eval(t),
        }
    }

    #[must_use]
    pub fn derivative(&self, t: f64) -> Vec2 {
        match self {
            Self::Straight { a, b } => Vec2::new(b.x - a.x, b.y - a.y),
            Self::Combined(combined) => combined.derivative(t),
        }
    }
}

impl CurveArc for FairArc {
    fn eval(&self, t: f64) -> Point {
        FairArc::eval(self, t)
    }

    fn derivative(&self, t: f64) -> Vec2 {
        FairArc::derivative(self, t)
    }
}

/// A chain of cubics, parameterized by normalized arc length.
#[derive(Clone, Debug)]
pub struct CombinedArc {
    segments: Vec<Bez3>,
    stops: Vec<f64>,
    end_adjustments: Vec<Vec2>,
    total_length: f64,
}

impl CombinedArc {
    /// A run of cubics treated as one arc, parameterized by length.
    #[must_use]
    pub fn new(arcs: Vec<Bez3>) -> Self {
        let mut segments = Vec::with_capacity(arcs.len());
        for arc in arcs {
            if bez3_length(&arc) > 0.0 {
                segments.push(arc);
            }
        }

        let mut stops = vec![0.0; segments.len()];
        let mut total_length = 0.0;
        for (index, segment) in segments.iter().enumerate() {
            stops[index] = total_length;
            total_length += bez3_length(segment);
        }
        for stop in &mut stops {
            *stop /= total_length;
        }

        let mut end_adjustments = vec![Vec2::ZERO; segments.len()];
        for i in 0..segments.len() {
            if i + 1 < segments.len() {
                let a = segments[i].eval(1.0);
                let b = segments[i + 1].eval(0.0);
                end_adjustments[i] = Vec2::new(b.x - a.x, b.y - a.y);
            }
        }

        Self {
            segments,
            stops,
            end_adjustments,
            total_length,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty() || self.total_length == 0.0
    }

    #[must_use]
    pub fn segments(&self) -> &[Bez3] {
        &self.segments
    }

    #[must_use]
    pub fn eval(&self, t: f64) -> Point {
        let (j, t_relative) = self.locate(t);
        let base = self.segments[j].eval(t_relative);
        let adjustment = self.end_adjustments[j];

        Point::new(
            base.x + t_relative * adjustment.x,
            base.y + t_relative * adjustment.y,
        )
    }

    #[must_use]
    pub fn derivative(&self, t: f64) -> Vec2 {
        let (j, t_relative) = self.locate(t);
        let scale = self.scale_at(j);
        let d = self.segments[j].derivative(t_relative);
        let adjustment = self.end_adjustments[j];

        Vec2::new((d.x + adjustment.x) * scale, (d.y + adjustment.y) * scale)
    }

    fn scale_at(&self, index: usize) -> f64 {
        let t_before = self.stops[index];
        let t_next = if index < self.stops.len() - 1 {
            self.stops[index + 1]
        } else {
            1.0
        };

        1.0 / (t_next - t_before)
    }

    fn locate(&self, t: f64) -> (usize, f64) {
        let index = segment_search(&self.stops, t);
        let t_before = self.stops[index];
        let t_next = if index < self.stops.len() - 1 {
            self.stops[index + 1]
        } else {
            1.0
        };
        let mut t_relative = (t - t_before) / (t_next - t_before);
        if t <= t_before {
            t_relative = 0.0;
        } else if t >= t_next {
            t_relative = 1.0;
        }

        (index, t_relative)
    }

    fn reduce_if_straight(self) -> FairArc {
        if self.segments.is_empty() {
            return FairArc::Combined(self);
        }
        let z0 = self.segments[0].a;
        let z1 = self.segments[self.segments.len() - 1].d;
        for segment in &self.segments {
            if !number_close(0.0, point_line_dist(z0, z1, segment.a))
                || !number_close(0.0, point_line_dist(z0, z1, segment.b))
                || !number_close(0.0, point_line_dist(z0, z1, segment.c))
                || !number_close(0.0, point_line_dist(z0, z1, segment.d))
            {
                return FairArc::Combined(self);
            }
        }

        FairArc::Straight { a: z0, b: z1 }
    }
}

/// Fairizes every contour of a shape.
#[must_use]
pub fn fairize(shape: &[Vec<Bez3>]) -> Vec<Vec<FairArc>> {
    let mut results = Vec::new();
    for contour in shape {
        if contour.is_empty() {
            continue;
        }
        let slices: Vec<Slice> = contour
            .iter()
            .map(|arc| Slice {
                cubic: *arc,
                corner_before: CornerType::Corner,
                corner_after: CornerType::Corner,
            })
            .collect();
        results.push(fairize_contour(slices));
    }

    results
}

fn fairize_contour(mut contour: Vec<Slice>) -> Vec<FairArc> {
    filter_degenerates(&mut contour);
    let split = canonical_start(mark_corners_and_split(&mut contour));

    let mut results = Vec::new();
    let mut front = 0;
    while front < split.len() {
        let mut rear = front;
        let mut stop_found = false;
        while rear < split.len() {
            if is_stop(split[rear].corner_after) {
                stop_found = true;
                break;
            }
            rear += 1;
        }
        let end = if stop_found { rear + 1 } else { split.len() };
        let combined = CombinedArc::new(split[front..end].iter().map(|s| s.cubic).collect());
        if !combined.is_empty() {
            results.push(combined.reduce_if_straight());
        }
        front = end;
    }

    results
}

fn is_stop(corner: CornerType) -> bool {
    corner != CornerType::Smooth
}

fn filter_degenerates(contour: &mut Vec<Slice>) {
    let mut keep = Vec::with_capacity(contour.len());
    for slice in contour.iter() {
        let straight = bez3_is_straight(&slice.cubic);
        let degenerate =
            straight && is_close_point(slice.cubic.d, slice.cubic.a, GEOMETRIC_EPSILON);

        if !degenerate {
            let cubic = if straight {
                force_straight(&slice.cubic)
            } else {
                slice.cubic
            };
            keep.push(Slice {
                cubic,
                corner_before: slice.corner_before,
                corner_after: slice.corner_after,
            });
        }
    }
    *contour = keep;
}

fn mark_corners_and_split(contour: &mut [Slice]) -> Vec<Slice> {
    let len = contour.len();
    for j in 0..len {
        let before = (j + len - 1) % len;
        let corner = classify(&contour[before], &contour[j]);
        contour[before].corner_after = corner;
        contour[j].corner_before = corner;
    }

    let mut sink = Vec::new();
    for slice in contour.iter() {
        split_at_extrema(slice, &mut sink);
    }

    sink
}

fn classify(before: &Slice, after: &Slice) -> CornerType {
    let z1 = after.cubic.a;
    let z0 = before.cubic.c;
    let z2 = after.cubic.b;
    let z11 = before.cubic.d;

    if !is_close_point(z1, z11, GEOMETRIC_EPSILON) {
        return CornerType::Corner;
    }

    let hetero = bez3_is_straight(&before.cubic) != bez3_is_straight(&after.cubic);
    let almost_linear = point_line_dist(z0, z2, z1) < GEOMETRIC_EPSILON;
    let in_between = (z0.x - z1.x) * (z2.x - z1.x) + (z0.y - z1.y) * (z2.y - z1.y) < 0.0;

    if hetero {
        CornerType::Hetero
    } else if almost_linear && in_between {
        let is_extrema = !bez3_is_straight(&before.cubic)
            && !bez3_is_straight(&after.cubic)
            && ((z0.x - z2.x).abs() < GEOMETRIC_EPSILON || (z0.y - z2.y).abs() < GEOMETRIC_EPSILON);
        if is_extrema {
            CornerType::Extrema
        } else {
            CornerType::Smooth
        }
    } else {
        CornerType::Corner
    }
}

fn canonical_start(mut contour: Vec<Slice>) -> Vec<Slice> {
    let mut start: Option<Point> = None;
    let mut start_index = 0;
    for (j, slice) in contour.iter().enumerate() {
        if is_stop(slice.corner_before) {
            let a = slice.cubic.a;
            let better = match start {
                None => true,
                Some(current) => a.y < current.y || (a.y == current.y && a.x < current.x),
            };

            if better {
                start = Some(a);
                start_index = j;
            }
        }
    }
    if start.is_some() {
        contour.rotate_left(start_index);
    }

    contour
}

fn split_at_extrema(arc: &Slice, sink: &mut Vec<Slice>) {
    let mut root_sink = RootSink::new();
    push_extrema(
        arc.cubic.a.x,
        arc.cubic.b.x,
        arc.cubic.c.x,
        arc.cubic.d.x,
        &mut root_sink,
    );
    push_extrema(
        arc.cubic.a.y,
        arc.cubic.b.y,
        arc.cubic.c.y,
        arc.cubic.d.y,
        &mut root_sink,
    );
    let mut roots = root_sink.roots;
    roots.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    if roots.len() > 1 {
        let mut j = 1;
        for i in 1..roots.len() {
            if roots[i] - roots[j - 1] >= CURVE_TIME_EPSILON {
                roots[j] = roots[i];
                j += 1;
            }
        }
        roots.truncate(j);
    }

    if !roots.is_empty() && roots[0] < CURVE_TIME_EPSILON {
        roots[0] = 0.0;
    } else {
        roots.insert(0, 0.0);
    }
    if let Some(last) = roots.last_mut()
        && *last > 1.0 - CURVE_TIME_EPSILON
    {
        *last = 1.0;
    } else {
        roots.push(1.0);
    }

    for k in 1..roots.len() {
        sink.push(Slice {
            cubic: split_ratio(&arc.cubic, roots[k - 1], roots[k]),
            corner_before: if k == 1 {
                arc.corner_before
            } else {
                CornerType::Extrema
            },
            corner_after: if k == roots.len() - 1 {
                arc.corner_after
            } else {
                CornerType::Extrema
            },
        });
    }
}

fn push_extrema(v0: f64, v1: f64, v2: f64, v3: f64, sink: &mut RootSink) {
    let a = 3.0 * (-v0 + 3.0 * v1 - 3.0 * v2 + v3);
    let b = 6.0 * (v0 - 2.0 * v1 + v2);
    let c = 3.0 * (v1 - v0);
    solve_quadratic(a, b, c, sink);
}

/// The root sink typo-geom uses while splitting at extrema.
///
/// `degenerated` sets the counter to -1, and the next accepted root is written
/// to a negative index, which a JavaScript array ignores while still advancing
/// the counter. A cubic whose extrema quadratic is degenerate on one axis
/// therefore loses one root on the other. That is upstream behaviour, and it is
/// reproduced here on purpose.
#[derive(Default)]
struct RootSink {
    roots: Vec<f64>,
    root_count: isize,
}

impl RootSink {
    fn new() -> Self {
        Self::default()
    }

    fn add_root(&mut self, x: f64) -> bool {
        let mut addable = x > 0.0 && x < 1.0;
        let count = self.root_count.max(0) as usize;
        for j in 0..count {
            if self.roots[j] == x {
                addable = false;
            }
        }
        if !addable {
            return false;
        }
        if self.root_count < 0 {
            self.root_count = 0;
            return true;
        }
        let index = self.root_count as usize;
        if index >= self.roots.len() {
            self.roots.push(x.clamp(0.0, 1.0));
        } else {
            self.roots[index] = x.clamp(0.0, 1.0);
        }
        self.root_count += 1;

        true
    }

    const fn degenerated(&mut self) {
        self.root_count = -1;
    }
}

fn solve_quadratic(a: f64, b: f64, c: f64, sink: &mut RootSink) {
    let mut a = a;
    let mut b = b;
    let mut c = c;
    let mut x1 = f64::INFINITY;
    let mut x2 = f64::INFINITY;

    if a.abs() < EPSILON {
        if b.abs() < EPSILON {
            sink.degenerated();
        }
        x1 = -c / b;
    } else {
        b *= -0.5;
        let mut d = discriminant(a, b, c);
        if d != 0.0 && d.abs() < MACHINE_EPSILON {
            let f = normalization_factor(a.abs(), b.abs(), c.abs());
            if f != 0.0 {
                a *= f;
                b *= f;
                c *= f;
                d = discriminant(a, b, c);
            }
        }

        if d >= -MACHINE_EPSILON {
            let q = if d < 0.0 { 0.0 } else { d.sqrt() };
            let r = b + if b < 0.0 { -q } else { q };

            if r == 0.0 {
                x1 = c / a;
                x2 = -x1;
            } else {
                x1 = r / a;
                x2 = c / r;
            }
        }
    }

    if x1.is_finite() {
        sink.add_root(x1);
    }

    if x2.is_finite() {
        sink.add_root(x2);
    }
}

fn discriminant(a: f64, b: f64, c: f64) -> f64 {
    let mut d = b * b - a * c;
    let e = b * b + a * c;
    if d.abs() * 3.0 < e {
        let ad = split_f64(a);
        let bd = split_f64(b);
        let cd = split_f64(c);
        let p = b * b;
        let dp = bd.0 * bd.0 - p + 2.0 * bd.0 * bd.1 + bd.1 * bd.1;
        let q = a * c;
        let dq = ad.0 * cd.0 - q + ad.0 * cd.1 + ad.1 * cd.0 + ad.1 * cd.1;
        d = p - q + (dp - dq);
    }

    d
}

fn split_f64(v: f64) -> (f64, f64) {
    let x = v * 134_217_729.0;
    let y = v - x;
    let hi = y + x;
    let lo = v - hi;

    (hi, lo)
}

fn normalization_factor(a: f64, b: f64, c: f64) -> f64 {
    let norm = a.max(b).max(c);
    if norm != 0.0 && (norm < 1e-8 || norm > 1e8) {
        2f64.powf(-js_round(norm.log2()))
    } else {
        0.0
    }
}

fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

fn segment_search(stops: &[f64], t: f64) -> usize {
    if t <= 0.0 || stops.len() == 1 {
        return 0;
    }
    if t >= stops[stops.len() - 1] {
        return stops.len() - 1;
    }
    let (mut l, mut r) = (0, stops.len());
    while l < r {
        let m = usize::midpoint(l, r);
        if stops[m] > t {
            r = m;
        } else {
            l = m + 1;
        }
    }

    r - 1
}

fn bez3_is_straight(cubic: &Bez3) -> bool {
    let p1 = cubic.a;
    let p2 = cubic.d;
    let h1 = Vec2::new(cubic.b.x - cubic.a.x, cubic.b.y - cubic.a.y);
    let h2 = Vec2::new(cubic.c.x - cubic.d.x, cubic.c.y - cubic.d.y);
    if is_almost_zero(h1) || is_almost_zero(h2) {
        return true;
    }
    let v = Vec2::new(p2.x - p1.x, p2.y - p1.y);
    if is_almost_zero(v) {
        return false;
    }
    if number_close(0.0, point_line_dist(cubic.a, cubic.d, cubic.b))
        && number_close(0.0, point_line_dist(cubic.a, cubic.d, cubic.c))
    {
        let div = v.dot(v);
        let s1 = v.dot(h1) / div;
        let s2 = v.dot(h2) / div;
        return (0.0..=1.0).contains(&s1) && (-1.0..=0.0).contains(&s2);
    }

    false
}

fn force_straight(cubic: &Bez3) -> Bez3 {
    Bez3::new(
        cubic.a,
        mix_point(cubic.a, cubic.d, 1.0 / 3.0),
        mix_point(cubic.a, cubic.d, 2.0 / 3.0),
        cubic.d,
    )
}

fn split_ratio(cubic: &Bez3, t1: f64, t2: f64) -> Bez3 {
    let mut v = *cubic;
    if t1 > 0.0 {
        v = split_at(&v, t1).1;
    }
    if t2 < 1.0 {
        v = split_at(&v, (t2 - t1) / (1.0 - t1)).0;
    }

    v
}

fn split_at(cubic: &Bez3, t: f64) -> (Bez3, Bez3) {
    let p3 = mix_point(cubic.a, cubic.b, t);
    let p4 = mix_point(cubic.b, cubic.c, t);
    let p5 = mix_point(cubic.c, cubic.d, t);
    let p6 = mix_point(p3, p4, t);
    let p7 = mix_point(p4, p5, t);
    let p8 = mix_point(p6, p7, t);

    (
        Bez3::new(cubic.a, p3, p6, p8),
        Bez3::new(p8, p7, p5, cubic.d),
    )
}

fn bez3_length(cubic: &Bez3) -> f64 {
    if bez3_is_straight(cubic) {
        return (cubic.d - cubic.a).hypot();
    }
    gauss_legendre_16(|t| length_integrand(cubic, t), 0.0, 1.0)
}

fn length_integrand(cubic: &Bez3, t: f64) -> f64 {
    let ax = 9.0 * (cubic.b.x - cubic.c.x) + 3.0 * (cubic.d.x - cubic.a.x);
    let bx = 6.0 * (cubic.a.x + cubic.c.x) - 12.0 * cubic.b.x;
    let cx = 3.0 * (cubic.b.x - cubic.a.x);
    let ay = 9.0 * (cubic.b.y - cubic.c.y) + 3.0 * (cubic.d.y - cubic.a.y);
    let by = 6.0 * (cubic.a.y + cubic.c.y) - 12.0 * cubic.b.y;
    let cy = 3.0 * (cubic.b.y - cubic.a.y);

    math::hypot((ax * t + bx) * t + cx, (ay * t + by) * t + cy)
}

/// 16-point Gauss-Legendre on `a..b`. Only the 16-point table is ported, which
/// is the one the length integrator asks for.
fn gauss_legendre_16(f: impl Fn(f64) -> f64, a: f64, b: f64) -> f64 {
    const ABSCISSAS: [f64; 8] = [
        0.09501250983763744,
        0.2816035507792589,
        0.45801677765722737,
        0.6178762444026438,
        0.755404408355003,
        0.8656312023878318,
        0.9445750230732326,
        0.9894009349916499,
    ];
    const WEIGHTS: [f64; 8] = [
        0.1894506104550685,
        0.18260341504492358,
        0.16915651939500254,
        0.14959598881657674,
        0.12462897125553388,
        0.09515851168249279,
        0.062253523938647894,
        0.027152459411754096,
    ];

    let half = (b - a) * 0.5;
    let center = half + a;
    let mut sum = 0.0;
    for i in 0..8 {
        let offset = half * ABSCISSAS[i];
        sum += WEIGHTS[i] * (f(center + offset) + f(center - offset));
    }

    half * sum
}

fn number_close(x: f64, y: f64) -> bool {
    (x - y).abs() < EPSILON
}

fn is_close_point(a: Point, b: Point, tolerance: f64) -> bool {
    let dx = a.x - b.x;
    let dy = a.y - b.y;

    dx * dx + dy * dy <= tolerance * tolerance
}

fn is_almost_zero(v: Vec2) -> bool {
    v.x * v.x + v.y * v.y < EPSILON * EPSILON
}

fn point_line_dist(a: Point, b: Point, p: Point) -> f64 {
    signed_distance(a.x, a.y, b.x, b.y, p.x, p.y).abs()
}

fn signed_distance(px: f64, py: f64, vx: f64, vy: f64, x: f64, y: f64) -> f64 {
    let dx = vx - px;
    let dy = vy - py;

    if number_close(dx, 0.0) {
        if dy >= 0.0 { px - x } else { x - px }
    } else if number_close(dy, 0.0) {
        if dx >= 0.0 { y - py } else { py - y }
    } else {
        (dx * (y - py) - dy * (x - px)) / (dx * dx + dy * dy).sqrt()
    }
}

fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn mix_point(a: Point, b: Point, t: f64) -> Point {
    Point::new(mix(a.x, b.x, t), mix(a.y, b.y, t))
}
