//! Spiro curve evaluation.
//!
//! Ported from spiro-js 3.0.1 (Apache-2.0 or MIT, Belleve Invis), which ports
//! Raph Levien's Spiro (Apache-2.0). Iteration counts and constants match the
//! JavaScript so both produce the same curves. Names keep the JavaScript
//! spelling where it helps line up the two files.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::manual_midpoint,
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::similar_names,
    clippy::suboptimal_flops,
    clippy::too_many_arguments,
    clippy::unreadable_literal
)]

use kurbo::{BezPath, Point, Vec2};

/// Largest number of pieces an arc is split into when cubic approximation
/// misses the bend threshold.
const MAX_STOPS: usize = 32;
/// Arcs flatter than this become a straight line instead of cubics.
const ARC_STRAIGHT_EPSILON: f64 = 1e-8;
/// Reciprocal of the finite-difference step for partial derivatives.
const RECIP_D: f64 = 2e6;
/// Steps per unit length in the curvature integral.
const INTEGRATION_STEPS: usize = 4;

/// Knot classification, same names as the JavaScript source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KnotType {
    #[default]
    Corner,
    Open,
    OpenEnd,
    Left,
    Right,
    G2,
    G4,
}

/// One control point of a spiro curve.
#[derive(Clone, Copy, Debug)]
pub struct Knot {
    pub x: f64,
    pub y: f64,
    pub ty: KnotType,
}

impl Knot {
    #[must_use]
    pub const fn new(x: f64, y: f64, ty: KnotType) -> Self {
        Self { x, y, ty }
    }
}

/// One solved segment between two knots.
#[derive(Clone, Copy, Debug, Default)]
struct SpiroSeg {
    x: f64,
    y: f64,
    ty: KnotType,
    bend_th: f64,
    ks: [f64; 4],
    seg_ch: f64,
    seg_th: f64,
}

/// Banded matrix row. `a` holds the band, `al` the elimination factors.
#[derive(Clone, Copy, Default)]
struct BandMat {
    a: [f64; 11],
    al: [f64; 5],
}

/// A curved span between two knots, described by four curvature moments.
#[derive(Clone, Debug)]
pub struct SpiroArc {
    ks: [f64; 4],
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub arc_length: f64,
    pub rot: f64,
    pub bend: f64,
    pub derive_x0: f64,
    pub derive_y0: f64,
    pub derive_x1: f64,
    pub derive_y1: f64,
}

impl SpiroArc {
    fn new(ks: [f64; 4], x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        let seg_ch = (x1 - x0).hypot(y1 - y0);
        let seg_th = (y1 - y0).atan2(x1 - x0);
        let xy = integrate_spiro(&ks);
        let ch = xy[0].hypot(xy[1]);
        let th = xy[1].atan2(xy[0]);
        let arc_length = seg_ch / ch;
        let rot = seg_th - th;
        let theta_left = rot + ks_theta(-0.5, -0.5, &ks);
        let theta_right = rot + ks_theta(0.5, 0.5, &ks);
        let derive_x0 = arc_length * theta_left.cos();
        let derive_y0 = arc_length * theta_left.sin();
        let derive_x1 = arc_length * theta_right.cos();
        let derive_y1 = arc_length * theta_right.sin();
        let bend = ks_bend(&ks);

        Self {
            ks,
            x0,
            y0,
            x1,
            y1,
            arc_length,
            rot,
            bend,
            derive_x0,
            derive_y0,
            derive_x1,
            derive_y1,
        }
    }

    /// Control points of the cubic that approximates this arc.
    #[must_use]
    pub fn cubic(&self) -> [Point; 4] {
        [
            Point::new(self.x0, self.y0),
            Point::new(
                self.x0 + self.derive_x0 / 3.0,
                self.y0 + self.derive_y0 / 3.0,
            ),
            Point::new(
                self.x1 - self.derive_x1 / 3.0,
                self.y1 - self.derive_y1 / 3.0,
            ),
            Point::new(self.x1, self.y1),
        ]
    }

    /// Point at parameter `at` in `0.0..=1.0`.
    #[must_use]
    pub fn eval(&self, at: f64) -> Point {
        let t = at - 0.5;
        let k_sub = divide_ks(-0.5, t, &self.ks);

        self.eval_impl(t, k_sub)
    }

    /// Tangent direction at parameter `at`.
    #[must_use]
    pub fn derivative(&self, at: f64) -> Vec2 {
        let t = at - 0.5;
        let theta = self.rot + ks_theta(t, t, &self.ks);

        Vec2::new(self.arc_length * theta.cos(), self.arc_length * theta.sin())
    }

    fn eval_impl(&self, t: f64, k_sub: [f64; 4]) -> Point {
        let th_sub = self.rot + ks_theta(-0.5, t, &self.ks);
        let cth = (t + 0.5) * self.arc_length * th_sub.cos();
        let sth = (t + 0.5) * self.arc_length * th_sub.sin();
        let xy_sub = integrate_spiro(&k_sub);

        Point::new(
            self.x0 + cth * xy_sub[0] - sth * xy_sub[1],
            self.y0 + cth * xy_sub[1] + sth * xy_sub[0],
        )
    }

    /// Splits this arc at parameter `at`, in the same parameterization the
    /// JavaScript source uses.
    #[must_use]
    pub fn subdivide(&self, at: f64) -> (Self, Self) {
        let t = at - 0.5;
        let k_sub = divide_ks(-0.5, t, &self.ks);
        let k_sub_rear = divide_ks(t, 0.5, &self.ks);
        let mid = self.eval_impl(t, k_sub);

        (
            Self::new(k_sub, self.x0, self.y0, mid.x, mid.y),
            Self::new(k_sub_rear, mid.x, mid.y, self.x1, self.y1),
        )
    }
}

/// Smooth path through the knots, as cubic curves.
///
/// `closed` joins the last knot back to the first. `delta` bounds the angular
/// bend of each emitted cubic, matching the `delta` argument of spiro-js.
#[must_use]
pub fn spiro_path(knots: &[Knot], closed: bool, delta: f64) -> BezPath {
    let mut path = BezPath::new();
    if knots.is_empty() {
        return path;
    }
    let segments = solve(knots, closed);
    let n_knots = knots.len();
    let n_segments = if segments[segments.len() - 1].ty == KnotType::OpenEnd {
        n_knots - 1
    } else {
        n_knots
    };
    for i in 0..n_segments {
        let (x0, y0) = (segments[i].x, segments[i].y);
        let (x1, y1) = (segments[i + 1].x, segments[i + 1].y);
        if i == 0 {
            path.move_to(Point::new(x0, y0));
        }
        let arc = SpiroArc::new(segments[i].ks, x0, y0, x1, y1);

        if arc.bend <= ARC_STRAIGHT_EPSILON {
            path.line_to(Point::new(x1, y1));
        } else {
            for part in find_subdivision(&arc, delta) {
                let [_, c1, c2, p3] = part.cubic();
                path.curve_to(c1, c2, p3);
            }
        }
    }

    path
}

/// Arcs of the solved curve, before cubic approximation.
#[must_use]
pub fn spiro_arcs(knots: &[Knot], closed: bool) -> Vec<SpiroArc> {
    let mut arcs = Vec::new();
    if knots.is_empty() {
        return arcs;
    }
    let segments = solve(knots, closed);
    let n_knots = knots.len();
    let n_segments = if segments[segments.len() - 1].ty == KnotType::OpenEnd {
        n_knots - 1
    } else {
        n_knots
    };
    for i in 0..n_segments {
        let (x0, y0) = (segments[i].x, segments[i].y);
        let (x1, y1) = (segments[i + 1].x, segments[i + 1].y);
        arcs.push(SpiroArc::new(segments[i].ks, x0, y0, x1, y1));
    }

    arcs
}

fn solve(knots: &[Knot], closed: bool) -> Vec<SpiroSeg> {
    let n = knots.len();
    let mut owned = knots.to_vec();

    if closed {
        run_spiro(&owned, n)
    } else {
        let old_start = owned[0].ty;
        let old_end = owned[n - 1].ty;
        owned[0].ty = KnotType::Open;
        owned[n - 1].ty = KnotType::OpenEnd;
        let segments = run_spiro(&owned, n);
        owned[n - 1].ty = old_end;
        owned[0].ty = old_start;

        segments
    }
}

fn run_spiro(src: &[Knot], n: usize) -> Vec<SpiroSeg> {
    let n_seg = if src[0].ty == KnotType::Open {
        n - 1
    } else {
        n
    };
    let mut segments = setup_path(src, n);
    if n_seg > 1 {
        solve_spiro(&mut segments, n_seg);
    }

    segments
}

fn count_vec(s: &[SpiroSeg], nseg: usize) -> usize {
    let mut n = 0;
    for i in 0..nseg {
        n += compute_jinc(s[i].ty, s[i + 1].ty);
    }

    n
}

fn find_subdivision(arc: &SpiroArc, delta: f64) -> Vec<SpiroArc> {
    'outer: for stops in 1..MAX_STOPS {
        let sink = uniform_subdivide(arc, stops);
        for part in &sink {
            if part.bend > delta {
                continue 'outer;
            }
        }
        return sink;
    }
    uniform_subdivide(arc, MAX_STOPS)
}

fn uniform_subdivide(arc: &SpiroArc, stops: usize) -> Vec<SpiroArc> {
    let mut sink = Vec::new();
    let mut current = arc.clone();
    let mut stops = stops;
    while stops > 1 {
        let (left, right) = current.subdivide(1.0 / stops as f64);
        sink.push(left);
        current = right;
        stops -= 1;
    }
    sink.push(current);

    sink
}

fn setup_path(src: &[Knot], n: usize) -> Vec<SpiroSeg> {
    let n_seg = if src[0].ty == KnotType::Open {
        n - 1
    } else {
        n
    };
    let mut r = vec![SpiroSeg::default(); n_seg + 1];
    for i in 0..n_seg {
        r[i] = SpiroSeg {
            x: src[i].x,
            y: src[i].y,
            ty: src[i].ty,
            ..SpiroSeg::default()
        };
    }
    if n_seg == 0 {
        return r;
    }
    r[n_seg] = SpiroSeg {
        x: src[n_seg % n].x,
        y: src[n_seg % n].y,
        ty: src[n_seg % n].ty,
        ..SpiroSeg::default()
    };
    for i in 0..n_seg {
        let dx = r[i + 1].x - r[i].x;
        let dy = r[i + 1].y - r[i].y;
        r[i].seg_ch = dx.hypot(dy);
        r[i].seg_th = dy.atan2(dx);
    }
    let mut i_last = n_seg - 1;
    for i in 0..n_seg {
        if matches!(
            r[i].ty,
            KnotType::Open | KnotType::OpenEnd | KnotType::Corner
        ) {
            r[i].bend_th = 0.0;
        } else {
            r[i].bend_th = mod_2pi(r[i].seg_th - r[i_last].seg_th);
        }
        i_last = i;
    }

    r
}

fn compute_ends(ks: &[f64; 4], ends: &mut [[f64; 4]; 2], seg_ch: f64) -> f64 {
    let xy = integrate_spiro(ks);
    let ch = xy[0].hypot(xy[1]);
    let th = xy[1].atan2(xy[0]);
    let l = ch / seg_ch;
    let th_even = 0.5 * ks[0] + (1.0 / 48.0) * ks[2];
    let th_odd = 0.125 * ks[1] + (1.0 / 384.0) * ks[3] - th;
    ends[0][0] = th_even - th_odd;
    ends[1][0] = th_even + th_odd;
    let k0_even = l * (ks[0] + 0.125 * ks[2]);
    let k0_odd = l * (0.5 * ks[1] + (1.0 / 48.0) * ks[3]);
    ends[0][1] = k0_even - k0_odd;
    ends[1][1] = k0_even + k0_odd;
    let l2 = l * l;
    let k1_even = l2 * (ks[1] + 0.125 * ks[3]);
    let k1_odd = l2 * 0.5 * ks[2];
    ends[0][2] = k1_even - k1_odd;
    ends[1][2] = k1_even + k1_odd;
    let l3 = l2 * l;
    let k2_even = l3 * ks[2];
    let k2_odd = l3 * 0.5 * ks[3];
    ends[0][3] = k2_even - k2_odd;
    ends[1][3] = k2_even + k2_odd;

    l
}

fn compute_pderivs(
    s: &SpiroSeg,
    ends: &mut [[f64; 4]; 2],
    derivs: &mut [[[f64; 4]; 2]; 4],
    jinc: usize,
) {
    let delta = 1.0 / RECIP_D;
    compute_ends(&s.ks, ends, s.seg_ch);

    for i in 0..jinc {
        let mut try_ks = s.ks;
        try_ks[i] += delta;
        let mut try_ends = [[0.0; 4]; 2];
        compute_ends(&try_ks, &mut try_ends, s.seg_ch);

        for k in 0..2 {
            for j in 0..4 {
                derivs[j][k][i] = RECIP_D * (try_ends[k][j] - ends[k][j]);
            }
        }
    }
}

fn mod_2pi(th: f64) -> f64 {
    let u = th / (2.0 * core::f64::consts::PI);
    2.0 * core::f64::consts::PI * (u - (u + 0.5).floor())
}

const fn compute_jinc(ty0: KnotType, ty1: KnotType) -> usize {
    use KnotType::{Corner, G2, G4, Left, Open, OpenEnd, Right};
    if matches!(ty0, G4) || matches!(ty1, G4) || matches!(ty0, Right) || matches!(ty1, Left) {
        4
    } else if matches!(ty0, G2) && matches!(ty1, G2) {
        2
    } else if (matches!(ty0, Open | Corner | Left) && matches!(ty1, G2))
        || (matches!(ty0, G2) && matches!(ty1, OpenEnd | Corner | Right))
    {
        1
    } else {
        0
    }
}

fn add_mat_line(
    m: &mut [BandMat],
    v: &mut [f64],
    derivs: &[f64; 4],
    x: f64,
    y: f64,
    j: usize,
    jj: i64,
    jinc: usize,
    nmat: usize,
) {
    if jj < 0 {
        return;
    }
    let jj = jj as usize;
    let j_i = j as i64;
    let jj_i = jj as i64;
    let nmat_i = nmat as i64;
    let mut joff = (j_i + 5 - jj_i + nmat_i) % nmat_i;
    if nmat < 6 {
        joff = j_i + 5 - jj_i;
    } else if nmat == 6 {
        joff = 2 + (j_i + 3 - jj_i + nmat_i) % nmat_i;
    }
    v[jj] += x;

    for k in 0..jinc {
        let index = joff + k as i64;
        // The JavaScript band row is a plain array, so an out-of-band write
        // lands on a slot that no reader looks at. Dropping it matches that.

        if (0..11).contains(&index) {
            m[jj].a[index as usize] += y * derivs[k];
        }
    }
}

fn spiro_iter(
    s: &mut [SpiroSeg],
    m: &mut [BandMat],
    perm: &mut [usize],
    v: &mut [f64],
    n: usize,
) -> f64 {
    let cyclic = !matches!(s[0].ty, KnotType::Open | KnotType::Corner);
    let nmat = count_vec(s, n);
    for i in 0..nmat {
        v[i] = 0.0;
        m[i] = BandMat::default();
    }
    let mut j: usize = 0;
    let mut jj: i64 = if s[0].ty == KnotType::G4 {
        nmat as i64 - 2
    } else if s[0].ty == KnotType::G2 {
        nmat as i64 - 1
    } else {
        0
    };
    for i in 0..n {
        let ty0 = s[i].ty;
        let ty1 = s[i + 1].ty;
        let jinc = compute_jinc(ty0, ty1);
        let th = s[i].bend_th;
        let mut ends = [[0.0; 4]; 2];
        let mut derivs = [[[0.0; 4]; 2]; 4];
        let mut jthl: i64 = -1;
        let mut jk0l: i64 = -1;
        let mut jk1l: i64 = -1;
        let mut jk2l: i64 = -1;
        let mut jthr: i64 = -1;
        let mut jk0r: i64 = -1;
        let mut jk1r: i64 = -1;
        let mut jk2r: i64 = -1;
        compute_pderivs(&s[i], &mut ends, &mut derivs, jinc);
        if matches!(
            ty0,
            KnotType::G4 | KnotType::G2 | KnotType::Left | KnotType::Right
        ) {
            jthl = jj;
            jj += 1;
            jj %= nmat as i64;
            jk0l = jj;
            jj += 1;
        }
        if ty0 == KnotType::G4 {
            jj %= nmat as i64;
            jk1l = jj;
            jj += 1;
            jk2l = jj;
            jj += 1;
        }
        if matches!(
            ty0,
            KnotType::Left | KnotType::Corner | KnotType::Open | KnotType::G2
        ) && jinc == 4
        {
            if ty0 != KnotType::G2 {
                jk1l = jj;
                jj += 1;
            }
            jk2l = jj;
            jj += 1;
        }
        if matches!(
            ty1,
            KnotType::Right | KnotType::Corner | KnotType::OpenEnd | KnotType::G2
        ) && jinc == 4
        {
            if ty1 != KnotType::G2 {
                jk1r = jj;
                jj += 1;
            }
            jk2r = jj;
            jj += 1;
        }
        if matches!(
            ty1,
            KnotType::G4 | KnotType::G2 | KnotType::Left | KnotType::Right
        ) {
            jthr = jj;
            jk0r = (jj + 1) % nmat as i64;
        }
        if ty1 == KnotType::G4 {
            jk1r = (jj + 2) % nmat as i64;
            jk2r = (jj + 3) % nmat as i64;
        }
        add_mat_line(
            m,
            v,
            &derivs[0][0],
            th - ends[0][0],
            1.0,
            j,
            jthl,
            jinc,
            nmat,
        );
        add_mat_line(m, v, &derivs[1][0], ends[0][1], -1.0, j, jk0l, jinc, nmat);
        add_mat_line(m, v, &derivs[2][0], ends[0][2], -1.0, j, jk1l, jinc, nmat);
        add_mat_line(m, v, &derivs[3][0], ends[0][3], -1.0, j, jk2l, jinc, nmat);
        add_mat_line(m, v, &derivs[0][1], -ends[1][0], 1.0, j, jthr, jinc, nmat);
        add_mat_line(m, v, &derivs[1][1], -ends[1][1], 1.0, j, jk0r, jinc, nmat);
        add_mat_line(m, v, &derivs[2][1], -ends[1][2], 1.0, j, jk1r, jinc, nmat);
        add_mat_line(m, v, &derivs[3][1], -ends[1][3], 1.0, j, jk2r, jinc, nmat);
        j += jinc;
    }
    let (n_invert, mut j_solve) = if cyclic {
        for i in 0..nmat {
            let row = m[i];
            m[i + nmat] = row;
            m[i + 2 * nmat] = row;
            let value = v[i];
            v[i + nmat] = value;
            v[i + 2 * nmat] = value;
        }
        (3 * nmat, nmat)
    } else {
        (nmat, 0)
    };
    bandec11(m, perm, n_invert);
    banbks11(m, perm, v, n_invert);
    let mut norm = 0.0;
    for i in 0..n {
        let jinc = compute_jinc(s[i].ty, s[i + 1].ty);
        for k in 0..jinc {
            let dk = v[j_solve];
            j_solve += 1;
            s[i].ks[k] += dk;
            norm += dk * dk;
        }
    }

    norm
}

fn bandec11(m: &mut [BandMat], perm: &mut [usize], n: usize) {
    for i in 0..5 {
        let mut j = 0;
        while j < i + 6 {
            m[i].a[j] = m[i].a[j + 5 - i];
            j += 1;
        }

        while j < 11 {
            m[i].a[j] = 0.0;
            j += 1;
        }
    }
    let mut l = 5;

    for k in 0..n {
        let mut pivot = k;
        let mut pivot_val = m[k].a[0];
        l = if l < n { l + 1 } else { n };
        for j in (k + 1)..l {
            if m[j].a[0].abs() > pivot_val.abs() {
                pivot_val = m[j].a[0];
                pivot = j;
            }
        }
        perm[k] = pivot;
        if pivot != k {
            for j in 0..11 {
                let tmp = m[k].a[j];
                m[k].a[j] = m[pivot].a[j];
                m[pivot].a[j] = tmp;
            }
        }
        if pivot_val.abs() < 1e-12 {
            pivot_val = 1e-12;
        }
        let pivot_scale = 1.0 / pivot_val;
        let pivot_row = m[k].a;

        for i in (k + 1)..l {
            let x = m[i].a[0] * pivot_scale;
            m[k].al[i - k - 1] = x;
            for j in 1..11 {
                m[i].a[j - 1] = m[i].a[j] - x * pivot_row[j];
            }
            m[i].a[10] = 0.0;
        }
    }
}

fn banbks11(m: &[BandMat], perm: &[usize], v: &mut [f64], n: usize) {
    let mut l = 5;
    for k in 0..n {
        let i = perm[k];
        if i != k {
            v.swap(k, i);
        }
        if l < n {
            l += 1;
        }

        for i in (k + 1)..l {
            v[i] -= m[k].al[i - k - 1] * v[k];
        }
    }
    let mut l = 1;

    for i in (0..n).rev() {
        let mut x = v[i];
        for k in 1..l {
            x -= m[i].a[k] * v[k + i];
        }
        v[i] = x / m[i].a[0];

        if l < 11 {
            l += 1;
        }
    }
}

fn solve_spiro(s: &mut [SpiroSeg], nseg: usize) {
    let nmat = count_vec(s, nseg);
    if nmat == 0 {
        return;
    }
    let mut n_alloc = nmat;
    if !matches!(s[0].ty, KnotType::Open | KnotType::Corner) {
        n_alloc *= 3;
    }
    if n_alloc < 5 {
        n_alloc = 5;
    }
    let mut m = vec![BandMat::default(); n_alloc];
    let mut v = vec![0.0; n_alloc];
    let mut perm = vec![0usize; n_alloc];

    for _ in 0..10 {
        let norm = spiro_iter(s, &mut m, &mut perm, &mut v, nseg);
        if norm < 1e-12 {
            break;
        }
    }
}

fn integrate_spiro(ks: &[f64; 4]) -> [f64; 2] {
    let th1 = ks[0];
    let th2 = 0.5 * ks[1];
    let th3 = (1.0 / 6.0) * ks[2];
    let th4 = (1.0 / 24.0) * ks[3];
    let ds = 1.0 / INTEGRATION_STEPS as f64;
    let ds2 = ds * ds;
    let ds3 = ds2 * ds;
    let k0 = ks[0] * ds;
    let k1 = ks[1] * ds;
    let k2 = ks[2] * ds;
    let k3 = ks[3] * ds;
    let mut s = 0.5 * ds - 0.5;
    let mut x = 0.0;
    let mut y = 0.0;
    for _ in 0..INTEGRATION_STEPS {
        let km0 = (((1.0 / 6.0) * k3 * s + 0.5 * k2) * s + k1) * s + k0;
        let km1 = ((0.5 * k3 * s + k2) * s + k1) * ds;
        let km2 = (k3 * s + k2) * ds2;
        let km3 = k3 * ds3;
        let t1_1 = km0;
        let t1_2 = 0.5 * km1;
        let t1_3 = (1.0 / 6.0) * km2;
        let t1_4 = (1.0 / 24.0) * km3;
        let t2_2 = t1_1 * t1_1;
        let t2_3 = 2.0 * (t1_1 * t1_2);
        let t2_4 = 2.0 * (t1_1 * t1_3) + t1_2 * t1_2;
        let t2_5 = 2.0 * (t1_1 * t1_4 + t1_2 * t1_3);
        let t2_6 = 2.0 * (t1_2 * t1_4) + t1_3 * t1_3;
        let t2_7 = 2.0 * (t1_3 * t1_4);
        let t2_8 = t1_4 * t1_4;
        let t3_4 = t2_2 * t1_2 + t2_3 * t1_1;
        let t3_6 = t2_2 * t1_4 + t2_3 * t1_3 + t2_4 * t1_2 + t2_5 * t1_1;
        let t3_8 = t2_4 * t1_4 + t2_5 * t1_3 + t2_6 * t1_2 + t2_7 * t1_1;
        let t3_10 = t2_6 * t1_4 + t2_7 * t1_3 + t2_8 * t1_2;
        let t4_4 = t2_2 * t2_2;
        let t4_5 = 2.0 * (t2_2 * t2_3);
        let t4_6 = 2.0 * (t2_2 * t2_4) + t2_3 * t2_3;
        let t4_7 = 2.0 * (t2_2 * t2_5 + t2_3 * t2_4);
        let t4_8 = 2.0 * (t2_2 * t2_6 + t2_3 * t2_5) + t2_4 * t2_4;
        let t4_9 = 2.0 * (t2_2 * t2_7 + t2_3 * t2_6 + t2_4 * t2_5);
        let t4_10 = 2.0 * (t2_2 * t2_8 + t2_3 * t2_7 + t2_4 * t2_6) + t2_5 * t2_5;
        let t5_6 = t4_4 * t1_2 + t4_5 * t1_1;
        let t5_8 = t4_4 * t1_4 + t4_5 * t1_3 + t4_6 * t1_2 + t4_7 * t1_1;
        let t5_10 = t4_6 * t1_4 + t4_7 * t1_3 + t4_8 * t1_2 + t4_9 * t1_1;
        let t6_6 = t4_4 * t2_2;
        let t6_7 = t4_4 * t2_3 + t4_5 * t2_2;
        let t6_8 = t4_4 * t2_4 + t4_5 * t2_3 + t4_6 * t2_2;
        let t6_9 = t4_4 * t2_5 + t4_5 * t2_4 + t4_6 * t2_3 + t4_7 * t2_2;
        let t6_10 = t4_4 * t2_6 + t4_5 * t2_5 + t4_6 * t2_4 + t4_7 * t2_3 + t4_8 * t2_2;
        let t7_8 = t6_6 * t1_2 + t6_7 * t1_1;
        let t7_10 = t6_6 * t1_4 + t6_7 * t1_3 + t6_8 * t1_2 + t6_9 * t1_1;
        let t8_8 = t6_6 * t2_2;
        let t8_9 = t6_6 * t2_3 + t6_7 * t2_2;
        let t8_10 = t6_6 * t2_4 + t6_7 * t2_3 + t6_8 * t2_2;
        let t9_10 = t8_8 * t1_2 + t8_9 * t1_1;
        let t10_10 = t8_8 * t2_2;
        let mut u = 1.0;
        let mut v = 0.0;
        v += (1.0 / 12.0) * t1_2 + (1.0 / 80.0) * t1_4;
        u -= (1.0 / 24.0) * t2_2
            + (1.0 / 160.0) * t2_4
            + (1.0 / 896.0) * t2_6
            + (1.0 / 4608.0) * t2_8;
        v -= (1.0 / 480.0) * t3_4
            + (1.0 / 2688.0) * t3_6
            + (1.0 / 13824.0) * t3_8
            + (1.0 / 67584.0) * t3_10;
        u += (1.0 / 1920.0) * t4_4
            + (1.0 / 10752.0) * t4_6
            + (1.0 / 55296.0) * t4_8
            + (1.0 / 270336.0) * t4_10;
        v += (1.0 / 53760.0) * t5_6 + (1.0 / 276480.0) * t5_8 + (1.0 / 1.35168e6) * t5_10;
        u -= (1.0 / 322560.0) * t6_6 + (1.0 / 1.65888e6) * t6_8 + (1.0 / 8.11008e6) * t6_10;
        v -= (1.0 / 1.16122e7) * t7_8 + (1.0 / 5.67706e7) * t7_10;
        u += (1.0 / 9.28973e7) * t8_8 + (1.0 / 4.54164e8) * t8_10;
        v += (1.0 / 4.08748e9) * t9_10;
        u -= (1.0 / 4.08748e10) * t10_10;
        let th = (((th4 * s + th3) * s + th2) * s + th1) * s;
        let cth = th.cos();
        let sth = th.sin();
        x += cth * u - sth * v;
        y += cth * v + sth * u;
        s += ds;
    }

    [x * ds, y * ds]
}

fn ks_bend(ks: &[f64; 4]) -> f64 {
    ks[0].abs() + (0.5 * ks[1]).abs() + (0.125 * ks[2]).abs() + ((1.0 / 48.0) * ks[3]).abs()
}

fn ks_theta(s0: f64, s1: f64, ks: &[f64; 4]) -> f64 {
    let s = (s0 + s1) / 2.0;
    s * ks[0]
        + 0.5 * s * s * ks[1]
        + (1.0 / 6.0) * s * s * s * ks[2]
        + (1.0 / 24.0) * s * s * s * s * ks[3]
}

fn divide_ks(s0: f64, s1: f64, ks: &[f64; 4]) -> [f64; 4] {
    let s = (s0 + s1) / 2.0;
    let t = s1 - s0;

    [
        t * (ks[0] + s * ks[1] + 0.5 * s * s * ks[2] + (1.0 / 6.0) * s * s * s * ks[3]),
        t * t * (ks[1] + s * ks[2] + 0.5 * s * s * ks[3]),
        t * t * t * (ks[2] + s * ks[3]),
        t * t * t * t * ks[3],
    ]
}

#[cfg(test)]
mod tests {
    use super::{Knot, KnotType, spiro_path};
    use kurbo::PathEl;

    fn knots(points: &[(f64, f64, KnotType)]) -> Vec<Knot> {
        points
            .iter()
            .map(|&(x, y, ty)| Knot::new(x, y, ty))
            .collect()
    }

    #[test]
    fn two_open_knots_make_a_line() {
        let k = knots(&[(0.0, 0.0, KnotType::Open), (400.0, 0.0, KnotType::OpenEnd)]);
        let path = spiro_path(&k, false, 1.0);
        let elements: Vec<PathEl> = path.elements().to_vec();
        assert_eq!(elements.len(), 2);
        assert!(matches!(elements[0], PathEl::MoveTo(p) if p == kurbo::Point::new(0.0, 0.0)));
        assert!(matches!(elements[1], PathEl::LineTo(p) if p == kurbo::Point::new(400.0, 0.0)));
    }

    #[test]
    fn closed_corner_square_has_four_segments() {
        let k = knots(&[
            (0.0, 0.0, KnotType::Corner),
            (400.0, 0.0, KnotType::Corner),
            (400.0, 400.0, KnotType::Corner),
            (0.0, 400.0, KnotType::Corner),
        ]);
        let path = spiro_path(&k, true, 1.0);
        let elements: Vec<PathEl> = path.elements().to_vec();
        assert_eq!(elements.len(), 5);
        assert!(matches!(elements[0], PathEl::MoveTo(p) if p == kurbo::Point::new(0.0, 0.0)));

        for element in &elements[1..] {
            assert!(matches!(element, PathEl::LineTo(_)));
        }
    }

    #[test]
    fn smooth_closed_loop_is_a_curve() {
        let k = knots(&[
            (0.0, 0.0, KnotType::G4),
            (400.0, 0.0, KnotType::G4),
            (400.0, 400.0, KnotType::G4),
            (0.0, 400.0, KnotType::G4),
        ]);
        let path = spiro_path(&k, true, 1.0);
        assert!(
            path.elements()
                .iter()
                .any(|e| matches!(e, PathEl::CurveTo(..)))
        );
    }

    #[test]
    fn evaluation_is_deterministic() {
        let k = knots(&[
            (0.0, 0.0, KnotType::G2),
            (300.0, -50.0, KnotType::G2),
            (600.0, 200.0, KnotType::G2),
            (300.0, 500.0, KnotType::G2),
        ]);
        let first = spiro_path(&k, true, 1.0);
        let second = spiro_path(&k, true, 1.0);
        assert_eq!(first, second);
    }
}
