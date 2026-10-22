//! The spiro control layer the glyph definitions are written in.
//!
//! Ported from `packages/geometry/src/spiro-control.mjs`. A glyph definition
//! builds a list of controls: knots, knot pairs, interpolators and terminators.
//! [`Flattener`] resolves the interpolators against their neighbours and reduces
//! everything to plain knots, then [`Flattener::pipe`] hands them to a
//! [`Collector`], which turns each one into a [`BiKnot`] the geometry tree can
//! consume.
//!
//! The coordinate-propagation path in the original (`resolveCoordiantePropogation`)
//! is not ported: nothing calls it.

#![allow(clippy::cast_precision_loss, clippy::suboptimal_flops)]

use kurbo::Vec2;
use lemmata_geom::{BiKnot, KnotType, math};

/// One operation an amendment function performs on the collector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Amendment {
    Widths { l: f64, r: f64 },
    WidthsHeading { l: f64, r: f64, dir: Vec2 },
    Heading(Vec2),
    DisableContrast,
    Unimportant,
    Important,
}

/// The amendment functions attached to a control, applied in order.
pub type Af = Vec<Amendment>;

/// A plain control knot.
#[derive(Clone, Debug, PartialEq)]
pub struct UserControlKnot {
    pub ty: KnotType,
    pub x: f64,
    pub y: f64,
    pub af: Af,
}

/// Two knots offset from a centre along a direction.
#[derive(Clone, Debug, PartialEq)]
pub struct UserCloseKnotPair {
    pub center: UserControlKnot,
    pub ty_pre: KnotType,
    pub ty_post: KnotType,
    pub dir: Vec2,
    pub d_pre: f64,
    pub d_post: f64,
}

/// A control that is resolved against the knots either side of it.
#[derive(Clone, Debug, PartialEq)]
pub enum Interpolator {
    SimpleMix {
        ty: KnotType,
        rx: f64,
        ry: f64,
        dx: f64,
        dy: f64,
        af: Af,
    },
    DistanceFromStart {
        ty: KnotType,
        distance: f64,
        dx: f64,
        dy: f64,
        af: Af,
    },
    DistTillEnd {
        ty: KnotType,
        distance: f64,
        dx: f64,
        dy: f64,
        af: Af,
    },
    MultiMix {
        rs: Vec<(f64, f64, f64)>,
        af: Af,
        ty: KnotType,
    },
    ArcHv {
        steps: usize,
        superness: f64,
    },
    ArcVh {
        steps: usize,
        superness: f64,
    },
    Decor(Vec<Control>),
}

/// One item in a control sequence.
#[derive(Clone, Debug, PartialEq)]
pub enum Control {
    Knot(UserControlKnot),
    Pair(UserCloseKnotPair),
    /// Contributes nothing, but still splits a run of interpolation.
    Virtual(UserControlKnot),
    Terminate {
        close: bool,
    },
    Interpolator(Interpolator),
}

/// What a sequence item is, before the flattener sorts it.
#[derive(Clone, Debug)]
pub enum Item {
    List(Vec<Item>),
    Af(Af),
    Close,
    End,
    Control(Control),
}

const DEP_SKIP: u32 = 0x1;
const DEP_PRE: u32 = 0x2 | 0x4;
const DEP_POST: u32 = 0x20 | 0x40;
const RES_DEP_STAGE_INTERPOLATION: u32 = 1;

impl Control {
    const fn dependency(&self, stage: u32) -> u32 {
        match self {
            Self::Knot(_) => 0,
            Self::Pair(_) => 0,
            Self::Virtual(_) => 0,
            Self::Terminate { .. } => 0,
            Self::Interpolator(_) => {
                if stage == RES_DEP_STAGE_INTERPOLATION {
                    DEP_PRE | DEP_POST
                } else {
                    0
                }
            }
        }
    }

    const fn kernel(&self) -> Option<&UserControlKnot> {
        match self {
            Self::Knot(knot) => Some(knot),
            Self::Pair(pair) => Some(&pair.center),
            Self::Virtual(knot) => Some(knot),
            Self::Terminate { .. } => None,
            Self::Interpolator(_) => None,
        }
    }

    /// What the control becomes once interpolation is done.
    fn resolve_non_interpolated(&self) -> Vec<Self> {
        match self {
            Self::Knot(_) => vec![self.clone()],
            Self::Virtual(_) => Vec::new(),
            Self::Pair(pair) => {
                vec![
                    Self::Knot(UserControlKnot {
                        ty: pair.ty_pre,
                        x: pair.center.x + pair.dir.x * pair.d_pre,
                        y: pair.center.y + pair.dir.y * pair.d_pre,
                        af: pair.center.af.clone(),
                    }),
                    Self::Knot(UserControlKnot {
                        ty: pair.ty_post,
                        x: pair.center.x + pair.dir.x * pair.d_post,
                        y: pair.center.y + pair.dir.y * pair.d_post,
                        af: pair.center.af.clone(),
                    }),
                ]
            }
            Self::Terminate { .. } => Vec::new(),
            Self::Interpolator(_) => unreachable!("interpolators are resolved before this"),
        }
    }

    fn resolve_interpolation(
        &self,
        before: &UserControlKnot,
        after: &UserControlKnot,
    ) -> Vec<Self> {
        let Self::Interpolator(interpolator) = self else {
            unreachable!("only interpolators resolve");
        };
        let knot =
            |ty: KnotType, x: f64, y: f64, af: Af| Self::Knot(UserControlKnot { ty, x, y, af });

        match interpolator {
            Interpolator::SimpleMix {
                ty,
                rx,
                ry,
                dx,
                dy,
                af,
            } => {
                vec![knot(
                    *ty,
                    mix(before.x, after.x, *rx) + dx,
                    mix(before.y, after.y, *ry) + dy,
                    af.clone(),
                )]
            }
            Interpolator::DistanceFromStart {
                ty,
                distance,
                dx,
                dy,
                af,
            } => {
                let total = math::hypot(after.x - before.x, after.y - before.y);
                let r = (distance / total).clamp(0.0, 1.0);

                vec![knot(
                    *ty,
                    mix(before.x, after.x, r) + dx,
                    mix(before.y, after.y, r) + dy,
                    af.clone(),
                )]
            }
            Interpolator::DistTillEnd {
                ty,
                distance,
                dx,
                dy,
                af,
            } => {
                let total = math::hypot(after.x - before.x, after.y - before.y);
                let r = (1.0 - distance / total).clamp(0.0, 1.0);

                vec![knot(
                    *ty,
                    mix(before.x, after.x, r) + dx,
                    mix(before.y, after.y, r) + dy,
                    af.clone(),
                )]
            }
            Interpolator::MultiMix { rs, af, ty } => {
                rs.iter()
                    .map(|(rx, ry, _)| {
                        knot(
                            *ty,
                            mix(before.x, after.x, *rx),
                            mix(before.y, after.y, *ry),
                            af.clone(),
                        )
                    })
                    .collect()
            }
            Interpolator::ArcHv { steps, superness } => {
                (1..*steps)
                    .map(|j| {
                        let theta =
                            ((j + 1) as f64 / (*steps + 2) as f64) * std::f64::consts::PI / 2.0;
                        let c = math::pow(math::cos(theta), 2.0 / superness);
                        let s = math::pow(math::sin(theta), 2.0 / superness);

                        knot(
                            KnotType::G2,
                            mix(before.x, after.x, s),
                            mix(before.y, after.y, 1.0 - c),
                            vec![Amendment::Unimportant],
                        )
                    })
                    .collect()
            }
            Interpolator::ArcVh { steps, superness } => {
                (1..*steps)
                    .map(|j| {
                        let theta =
                            ((j + 1) as f64 / (*steps + 2) as f64) * std::f64::consts::PI / 2.0;
                        let c = math::pow(math::cos(theta), 2.0 / superness);
                        let s = math::pow(math::sin(theta), 2.0 / superness);

                        knot(
                            KnotType::G2,
                            mix(before.x, after.x, 1.0 - c),
                            mix(before.y, after.y, s),
                            vec![Amendment::Unimportant],
                        )
                    })
                    .collect()
            }
            Interpolator::Decor(items) => items.clone(),
        }
    }
}

/// Collects controls, resolves interpolation, and pipes knots into a collector.
#[derive(Clone, Debug, Default)]
pub struct Flattener {
    pre_controls: Vec<Af>,
    controls: Vec<Control>,
    post_controls: Vec<bool>,
}

impl Flattener {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// # Errors
    ///
    /// Returns a message when an amendment comes after a control, or a
    /// terminator comes before one, which the sequence order forbids.
    pub fn add(&mut self, item: Item) -> Result<(), String> {
        match item {
            Item::List(items) => {
                for item in items {
                    self.add(item)?;
                }
            }
            Item::Af(af) => {
                if !self.controls.is_empty() {
                    return Err("amendments must be added before any control".to_string());
                }
                self.pre_controls.push(af);
            }
            Item::Close | Item::End => {
                self.post_controls.push(matches!(item, Item::Close));
            }
            Item::Control(control) => {
                if !self.post_controls.is_empty() {
                    return Err("terminators must be added last".to_string());
                }
                self.controls.push(control);
            }
        }
        Ok(())
    }

    /// Resolves every interpolator against the knots around it.
    pub fn flatten(&mut self) {
        for _ in 0..32 {
            if self.interpolate() == 0 {
                break;
            }
        }
        let mut resolved = Vec::with_capacity(self.controls.len());
        for control in std::mem::take(&mut self.controls) {
            resolved.extend(control.resolve_non_interpolated());
        }
        self.controls = resolved;
    }

    /// Feeds the flattened sequence to a collector.
    pub fn pipe(&self, collector: &mut Collector) {
        for af in &self.pre_controls {
            collector.apply(af);
        }
        for control in &self.controls {
            if let Control::Knot(knot) = control {
                collector.push_knot(knot);
            }
        }
        for close in &self.post_controls {
            if *close {
                collector.closed = true;
            }
        }
        collector.finish();
    }

    /// One interpolation pass. Returns how many items still carry a dependency.
    fn interpolate(&mut self) -> usize {
        let dependencies = self.dependencies();
        let mut sink = Vec::with_capacity(self.controls.len());
        let mut remaining = 0;
        for (index, control) in self.controls.iter().enumerate() {
            if dependencies.deps[index] <= DEP_SKIP {
                remaining += push_dependency(&mut sink, control);
                continue;
            }
            let before = self.controls[dependencies.prev[index]]
                .kernel()
                .cloned()
                .unwrap_or(UserControlKnot {
                    ty: KnotType::Corner,
                    x: 0.0,
                    y: 0.0,
                    af: Vec::new(),
                });
            let after = self.controls[dependencies.next[index]]
                .kernel()
                .cloned()
                .unwrap_or_else(|| before.clone());

            for item in control.resolve_interpolation(&before, &after) {
                remaining += push_dependency(&mut sink, &item);
            }
        }
        self.controls = sink;

        remaining
    }

    /// For each control, the nearest non-dependent control either side.
    fn dependencies(&self) -> Dependencies {
        let count = self.controls.len();
        let mut deps = Vec::with_capacity(count);
        let mut next = vec![usize::MAX; count];
        let mut prev = vec![usize::MAX; count];
        for control in &self.controls {
            deps.push(control.dependency(RES_DEP_STAGE_INTERPOLATION));
        }

        let mut first = usize::MAX;
        let mut last = usize::MAX;
        for index in 0..count {
            if deps[index] == 0 {
                if first == usize::MAX {
                    first = index;
                }
                if last != usize::MAX {
                    next[last] = index;
                    prev[index] = last;
                }
                last = index;
            } else if last != usize::MAX {
                prev[index] = last;
            }
        }
        if first != usize::MAX && last != usize::MAX {
            next[last] = first;
            prev[first] = last;
        }
        for value in prev.iter_mut().take(first.min(count)) {
            *value = last;
        }
        for index in 0..count {
            if deps[index] != 0 && prev[index] != usize::MAX {
                next[index] = next[prev[index]];
            }
        }

        Dependencies { deps, next, prev }
    }

    /// The knots the sequence produced, for tests and inspection.
    #[must_use]
    pub fn knots(&self) -> Vec<BiKnot> {
        let mut collector = Collector::new(0.0);
        self.pipe(&mut collector);

        collector.knots
    }
}

struct Dependencies {
    deps: Vec<u32>,
    next: Vec<usize>,
    prev: Vec<usize>,
}

fn push_dependency(sink: &mut Vec<Control>, control: &Control) -> usize {
    let dependent = control.dependency(RES_DEP_STAGE_INTERPOLATION) != 0;
    sink.push(control.clone());

    usize::from(dependent)
}

/// Turns controls into the bi-knots the geometry tree consumes.
#[derive(Clone, Debug)]
pub struct Collector {
    pub contrast: f64,
    pub closed: bool,
    pub knots: Vec<BiKnot>,
    default_d1: f64,
    default_d2: f64,
    finished: bool,
}

impl Collector {
    #[must_use]
    pub const fn new(contrast: f64) -> Self {
        Self {
            contrast,
            closed: false,
            knots: Vec::new(),
            default_d1: 0.0,
            default_d2: 0.0,
            finished: false,
        }
    }

    /// Applies an amendment to the last knot, or to the defaults before any.
    pub fn apply(&mut self, af: &Af) {
        for amendment in af {
            match amendment {
                Amendment::Widths { l, r } => self.set_widths(*l, *r),
                Amendment::WidthsHeading { l, r, dir } => {
                    self.set_widths(*l, *r);
                    self.heads_to(*dir);
                }
                Amendment::Heading(dir) => self.heads_to(*dir),
                Amendment::DisableContrast => self.contrast = 1.0,
                Amendment::Unimportant => {
                    if let Some(knot) = self.knots.last_mut() {
                        knot.unimportant = true;
                    }
                }
                Amendment::Important => {
                    if let Some(knot) = self.knots.last_mut() {
                        knot.unimportant = false;
                    }
                }
            }
        }
    }

    /// Pushes one knot, inheriting the previous knot's widths, then amends it.
    pub fn push_knot(&mut self, knot: &UserControlKnot) {
        assert!(!self.finished, "cannot push a knot after the sequence ends");
        let (d1, d2) = match self.knots.last() {
            Some(previous) => (previous.d1, previous.d2),
            None => (self.default_d1, self.default_d2),
        };
        self.knots
            .push(BiKnot::new(knot.ty, knot.x, knot.y, d1, d2));
        self.apply(&knot.af);
    }

    pub const fn finish(&mut self) {
        self.finished = true;
    }

    fn set_widths(&mut self, l: f64, r: f64) {
        if let Some(knot) = self.knots.last_mut() {
            knot.d1 = l;
            knot.d2 = r;
        } else {
            self.default_d1 = l;
            self.default_d2 = r;
        }
    }

    fn heads_to(&mut self, dir: Vec2) {
        if let Some(knot) = self.knots.last_mut() {
            knot.proposed_normal = Some(dir);
        }
    }
}

fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
