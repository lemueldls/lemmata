//! Quadratic conversion for TrueType output.
//!
//! Ports `quadify.mjs` and the `Quadify.auto` curve fitter it calls from
//! typo-geom. The sink turns cubic contours into quadratics, then cleans them up:
//! coordinate alignment, occurrent-point removal, colinear removal, integer
//! rounding, and a start-point rotation that keeps deltas small.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::similar_names,
    clippy::float_cmp,
    clippy::suboptimal_flops
)]

use kurbo::Point;

use crate::stroke::Bez3;

/// Occurrent precision, from `curve-util.mjs`.
const OCCURRENT_PRECISION: f64 = 1.0 / 16.0;
/// Colinearity tolerance, from `curve-util.mjs`.
const GEOMETRY_PRECISION: f64 = 0.25;
/// Error the autofitter allows, from `QuadifySink.arcTo`.
const ALLOW_ERROR: f64 = 1.0;
/// Segment cap the autofitter tries, from `QuadifySink.arcTo`.
const MAX_SEGMENTS: usize = 8;

/// Whether a contour point is a corner or a quadratic off-point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuadifyKind {
    Corner,
    Quadratic,
}

/// One point of a quadratified contour.
#[derive(Clone, Copy, Debug)]
pub struct QuadifyPoint {
    pub kind: QuadifyKind,
    pub x: f64,
    pub y: f64,
}

impl QuadifyPoint {
    const fn corner(x: f64, y: f64) -> Self {
        Self {
            kind: QuadifyKind::Corner,
            x,
            y,
        }
    }

    const fn quadratic(x: f64, y: f64) -> Self {
        Self {
            kind: QuadifyKind::Quadratic,
            x,
            y,
        }
    }

    fn mix(kind: QuadifyKind, a: &Self, b: &Self) -> Self {
        Self {
            kind,
            x: a.x + (b.x - a.x) * 0.5,
            y: a.y + (b.y - a.y) * 0.5,
        }
    }
}

/// Sink that collects quadratic contours.
#[derive(Debug, Default)]
pub struct QuadifySink {
    contours: Vec<Vec<QuadifyPoint>>,
    last_contour: Vec<QuadifyPoint>,
}

impl QuadifySink {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn into_contours(self) -> Vec<Vec<QuadifyPoint>> {
        self.contours
    }

    pub const fn begin_shape(&mut self) {}

    pub fn end_shape(&mut self) {
        let mut contour = std::mem::take(&mut self.last_contour);
        if contour.len() <= 2 {
            return;
        }

        contour = align_hv_knots(contour);
        drop_duplicate_first_last(&mut contour);
        cleanup_occurrent_knots1(&mut contour);
        contour = cleanup_occurrent_knots2(&contour);
        cleanup_occurrent_knots1(&mut contour);
        remove_colinear_arc(&mut contour);
        remove_colinear_corners(&mut contour);
        cleanup_occurrent_knots1(&mut contour);
        for point in &mut contour {
            point.x = js_round(point.x);
            point.y = js_round(point.y);
        }
        if contour.len() <= 2 {
            return;
        }

        rotate_to_optimized_start(&mut contour, &self.contours);
        self.contours.push(contour);
    }

    pub fn move_to(&mut self, x: f64, y: f64) {
        self.end_shape();
        self.line_to(x, y);
    }

    pub fn line_to(&mut self, x: f64, y: f64) {
        self.last_contour.push(QuadifyPoint::corner(x, y));
    }

    pub fn arc_to(&mut self, arc: &Bez3) {
        for point in auto_quadify(arc, ALLOW_ERROR, MAX_SEGMENTS) {
            self.last_contour
                .push(QuadifyPoint::quadratic(point.x, point.y));
        }
        self.line_to(arc.d.x, arc.d.y);
    }
}

/// Turns cubic contours into quadratic contours.
#[must_use]
pub fn quadify(shape: &[Vec<Bez3>]) -> Vec<Vec<QuadifyPoint>> {
    let mut sink = QuadifySink::new();
    sink.begin_shape();
    for contour in shape {
        let Some(first) = contour.first() else {
            continue;
        };
        sink.move_to(first.a.x, first.a.y);
        let mut current = first.a;
        for arc in contour {
            if !points_close(arc.a, current, GEOMETRY_PRECISION) {
                sink.line_to(arc.a.x, arc.a.y);
            }
            sink.arc_to(arc);
            current = arc.d;
        }
    }
    sink.end_shape();
    sink.into_contours()
}

/// The autofitter from `Quadify.auto`: off-points for the fewest segments that
/// fit within `allow_error`.
#[must_use]
pub fn auto_quadify(arc: &Bez3, allow_error: f64, max_segments: usize) -> Vec<Point> {
    let mut results: Option<Vec<Point>> = None;
    for segments in 1..=max_segments {
        let off_points = quadify_curve(arc, segments);
        if off_points.is_empty() {
            continue;
        }
        if error_fits_in(arc, &off_points, allow_error * allow_error) {
            return off_points;
        }
        results = Some(off_points);
    }
    results.unwrap_or_default()
}

fn quadify_curve(arc: &Bez3, segments: usize) -> Vec<Point> {
    let mut knots = Vec::with_capacity(2 * segments);
    let mut z_before = arc.eval(0.0);
    let mut d_before = arc.derivative(0.0);
    let scale = 1.0 / (4.0 * segments as f64);
    for k in 0..segments {
        let t_after = (k + 1) as f64 / segments as f64;
        let z_after = arc.eval(t_after);
        let d_after = arc.derivative(t_after);
        knots.push(Point::new(
            z_before.x + scale * d_before.x,
            z_before.y + scale * d_before.y,
        ));
        knots.push(Point::new(
            z_after.x - scale * d_after.x,
            z_after.y - scale * d_after.y,
        ));
        z_before = z_after;
        d_before = d_after;
    }
    knots
}

fn error_fits_in(arc: &Bez3, off_points: &[Point], square_error: f64) -> bool {
    let n = off_points.len();
    let mut z_before = arc.eval(0.0);
    let mut d_before = arc.derivative(0.0);
    for j in 0..n {
        let z_off = off_points[j];
        let t_after = (j + 1) as f64 / n as f64;
        let z_after = arc.eval(t_after);
        let d_after = arc.derivative(t_after);

        if j % 2 == 1 {
            let z_off_prev = off_points[j - 1];
            let z_before_qu = Point::new(
                z_off.x + (z_off_prev.x - z_off.x) * 0.5,
                z_off.y + (z_off_prev.y - z_off.y) * 0.5,
            );
            let d_before_qu = (
                (z_off.x - z_off_prev.x) * n as f64,
                (z_off.y - z_off_prev.y) * n as f64,
            );

            if square_distance(z_before, z_before_qu) > square_error {
                return false;
            }
            let limit = 4.0 * (n * n) as f64 * square_error;
            if square_distance_vec(d_before, d_before_qu) > limit {
                return false;
            }
        }

        z_before = z_after;
        d_before = d_after;
    }
    true
}

fn align_hv_knots(contour: Vec<QuadifyPoint>) -> Vec<QuadifyPoint> {
    let mut result = contour;
    let len = result.len();
    let mut align_x = CoordinateAligner::new(&result, Axis::X);
    let mut align_y = CoordinateAligner::new(&result, Axis::Y);

    for i in 0..len {
        let next = (i + 1) % len;
        let current = result[i];
        let following = result[next];
        if current.kind == QuadifyKind::Quadratic && following.kind == QuadifyKind::Corner {
            align_x.try_align(&result, i, next);
            align_y.try_align(&result, i, next);
        } else {
            align_x.try_align(&result, next, i);
            align_y.try_align(&result, next, i);
        }
    }

    align_x.apply(&mut result);
    align_y.apply(&mut result);
    result
}

fn drop_duplicate_first_last(contour: &mut Vec<QuadifyPoint>) {
    while contour.len() > 1 {
        let first = contour[0];
        let last = contour[contour.len() - 1];
        if first.kind == QuadifyKind::Corner
            && last.kind == QuadifyKind::Corner
            && occurrent(first, last)
        {
            contour.pop();
        } else {
            break;
        }
    }
}

fn cleanup_occurrent_knots1(contour: &mut Vec<QuadifyPoint>) {
    let mut drops = vec![false; contour.len()];
    for i in 0..contour.len() {
        let post = (i + 1) % contour.len();
        if post > 0
            && contour[i].kind == QuadifyKind::Corner
            && contour[post].kind == QuadifyKind::Corner
            && occurrent(contour[i], contour[post])
        {
            drops[post] = true;
        }
    }
    drop_by(contour, &drops);
}

fn cleanup_occurrent_knots2(contour: &[QuadifyPoint]) -> Vec<QuadifyPoint> {
    let mut insert_after = vec![false; contour.len()];
    for i in 0..contour.len() {
        if contour[i].kind != QuadifyKind::Quadratic {
            continue;
        }
        let pre = (i + contour.len() - 1) % contour.len();
        let post = (i + 1) % contour.len();
        if occurrent(contour[pre], contour[i]) && contour[post].kind == QuadifyKind::Quadratic {
            insert_after[i] = true;
        }
        if occurrent(contour[i], contour[post]) && contour[pre].kind == QuadifyKind::Quadratic {
            insert_after[pre] = true;
        }
    }

    let mut result = Vec::with_capacity(contour.len());
    for i in 0..contour.len() {
        result.push(contour[i]);
        if insert_after[i] {
            let post = (i + 1) % contour.len();
            result.push(QuadifyPoint::mix(
                QuadifyKind::Corner,
                &contour[i],
                &contour[post],
            ));
        }
    }
    result
}

fn remove_colinear_corners(contour: &mut Vec<QuadifyPoint>) {
    loop {
        let mut found = false;
        for i in 0..contour.len() {
            let previous = contour[(i + contour.len() - 1) % contour.len()];
            let current = contour[i];
            let next = contour[(i + 1) % contour.len()];
            if previous.kind == QuadifyKind::Corner
                && next.kind == QuadifyKind::Corner
                && (points_hv_colinear(previous, current, next)
                    || points_colinear(previous, current, next))
            {
                contour.remove(i);
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
    }
}

fn remove_colinear_arc(contour: &mut Vec<QuadifyPoint>) {
    let mut front = 0;
    let mut should_remove = vec![false; contour.len()];
    let mut middle_points: Vec<QuadifyPoint> = Vec::new();

    for rear in 1..=contour.len() {
        let z_front = contour[front];
        let z_rear = contour[rear % contour.len()];
        if z_rear.kind == QuadifyKind::Corner {
            let mut all_colinear = true;
            for middle in &middle_points {
                if !points_hv_colinear(z_front, *middle, z_rear) {
                    all_colinear = false;
                }
            }
            if all_colinear {
                for i in (front + 1)..rear {
                    should_remove[i] = true;
                }
            }
            front = rear;
            middle_points.clear();
        } else {
            middle_points.push(z_rear);
        }
    }

    drop_by(contour, &should_remove);
}

fn rotate_to_optimized_start(contour: &mut Vec<QuadifyPoint>, contours: &[Vec<QuadifyPoint>]) {
    let (mut last_x, mut last_y) = (0.0, 0.0);
    if let Some(previous) = contours.last()
        && let Some(point) = previous.last()
    {
        last_x = point.x;
        last_y = point.y;
    }

    let mut min_id = 0;
    let min_x = contour[0].x;
    let min_y = contour[0].y;
    let mut min_cost = estimate_byte_cost_of_delta(contour[0], last_x, last_y);

    for i in 1..contour.len() {
        let point = contour[i];
        if point.kind != QuadifyKind::Corner {
            continue;
        }
        let cost = estimate_byte_cost_of_delta(point, last_x, last_y);
        if cost < min_cost
            || (cost == min_cost && point.y < min_y)
            || (cost == min_cost && point.y == min_y && point.x < min_x)
        {
            min_cost = cost;
            min_id = i;
        }
    }

    if min_id > 0 {
        let rotated: Vec<QuadifyPoint> = contour.drain(0..min_id).collect();
        contour.extend(rotated);
    }
}

fn estimate_byte_cost_of_delta(point: QuadifyPoint, last_x: f64, last_y: f64) -> i32 {
    let delta_x = point.x - last_x;
    let delta_y = point.y - last_y;
    let cost_x = if delta_x == 0.0 {
        0
    } else if delta_x.abs() < 128.0 {
        1
    } else {
        2
    };
    let cost_y = if delta_y == 0.0 {
        0
    } else if delta_y.abs() < 128.0 {
        1
    } else {
        2
    };
    cost_x + cost_y
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

/// Union-find for coordinate alignment.
struct CoordinateAligner {
    up: Vec<usize>,
    rank: Vec<f64>,
    axis: Axis,
}

impl CoordinateAligner {
    fn new(contour: &[QuadifyPoint], axis: Axis) -> Self {
        let mut up = Vec::with_capacity(contour.len());
        let mut rank = Vec::with_capacity(contour.len());
        for point in contour {
            let value = lens(*point, axis);
            up.push(up.len());
            rank.push((value - js_round(value)).abs());
        }
        Self { up, rank, axis }
    }

    fn find(&mut self, mut i: usize) -> usize {
        while self.up[i] != i {
            self.up[i] = self.up[self.up[i]];
            i = self.up[i];
        }
        i
    }

    fn try_align(&mut self, contour: &[QuadifyPoint], i: usize, j: usize) {
        let a = lens(contour[i], self.axis);
        let b = lens(contour[j], self.axis);
        if (a - b).abs() < OCCURRENT_PRECISION {
            self.align(i, j);
        }
    }

    fn align(&mut self, i: usize, j: usize) {
        let mut i = self.find(i);
        let mut j = self.find(j);
        if self.rank[i] > self.rank[j] {
            std::mem::swap(&mut i, &mut j);
        }
        self.up[j] = i;
    }

    fn apply(&mut self, contour: &mut [QuadifyPoint]) {
        for i in 0..contour.len() {
            let root = self.find(i);
            let value = lens(contour[root], self.axis);
            set_lens(&mut contour[i], self.axis, js_round(value));
        }
    }
}

const fn lens(point: QuadifyPoint, axis: Axis) -> f64 {
    match axis {
        Axis::X => point.x,
        Axis::Y => point.y,
    }
}

const fn set_lens(point: &mut QuadifyPoint, axis: Axis, value: f64) {
    match axis {
        Axis::X => point.x = value,
        Axis::Y => point.y = value,
    }
}

fn occurrent(a: QuadifyPoint, b: QuadifyPoint) -> bool {
    a.x == b.x && a.y == b.y
}

fn aligned(a: f64, b: f64, c: f64) -> bool {
    a == b && b == c
}

fn points_hv_colinear(previous: QuadifyPoint, current: QuadifyPoint, next: QuadifyPoint) -> bool {
    if aligned(previous.x, current.x, next.x) {
        return true;
    }
    aligned(previous.y, current.y, next.y)
}

fn in_between(a: f64, b: f64, c: f64) -> bool {
    (a <= b && b <= c) || (c <= b && b <= a)
}

fn points_colinear(previous: QuadifyPoint, current: QuadifyPoint, next: QuadifyPoint) -> bool {
    if !in_between(previous.x, current.x, next.x) {
        return false;
    }
    if !in_between(previous.y, current.y, next.y) {
        return false;
    }
    let dx = next.x - previous.x;
    let dy = next.y - previous.y;
    let t = (current.y - previous.y) * dx - (current.x - previous.x) * dy;
    t * t < GEOMETRY_PRECISION * GEOMETRY_PRECISION * (dx * dx + dy * dy)
}

fn drop_by(contour: &mut Vec<QuadifyPoint>, should_remove: &[bool]) {
    let mut n = 0;
    for i in 0..contour.len() {
        if !should_remove[i] {
            contour[n] = contour[i];
            n += 1;
        }
    }
    contour.truncate(n);
}

/// `Math.round`: half-way values go toward positive infinity, not away from zero.
fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

fn points_close(a: Point, b: Point, tolerance: f64) -> bool {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy < tolerance * tolerance
}

fn square_distance(a: Point, b: Point) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

fn square_distance_vec(a: kurbo::Vec2, b: (f64, f64)) -> f64 {
    let dx = a.x - b.0;
    let dy = a.y - b.1;
    dx * dx + dy * dy
}
