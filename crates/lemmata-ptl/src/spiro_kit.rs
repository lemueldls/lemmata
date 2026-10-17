//! The knot builders glyph definitions are written with.
//!
//! Ported from `packages/font-kits/src/spiro-kit.mjs`. The builders produce
//! [`Item`]s for a [`Flattener`]; the sink helpers at the bottom turn a finished
//! sequence into geometry the tree can consume.
//!
//! The kit is parameterized the way upstream's is: a stroke width, a superness,
//! a contrast and the tiny offset the directed knot pairs are built from.

use kurbo::Vec2;
use lemmata_geom::{Geometry, KnotType, MonoKnot};

use crate::control::{Af, Amendment, Collector, Control, Flattener, Interpolator, Item};

/// The eight knot families, each a triple of knot types for a directed pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    G4,
    G2,
    Corner,
    /// A flat-to-curl transition: `flat`, `g2`, `curl`.
    Straight,
    G2Corner,
    CornerG2,
    FlatCorner,
    CornerCurl,
}

impl Family {
    /// The pre, centre and post knot types.
    #[must_use]
    pub const fn types(self) -> (KnotType, KnotType, KnotType) {
        match self {
            Self::G4 => (KnotType::G4, KnotType::G4, KnotType::G4),
            Self::G2 => (KnotType::G2, KnotType::G2, KnotType::G2),
            Self::Corner => (KnotType::Corner, KnotType::Corner, KnotType::Corner),
            Self::Straight => (KnotType::Left, KnotType::G2, KnotType::Right),
            Self::G2Corner => (KnotType::G2, KnotType::Corner, KnotType::Corner),
            Self::CornerG2 => (KnotType::Corner, KnotType::Corner, KnotType::G2),
            Self::FlatCorner => (KnotType::Left, KnotType::Corner, KnotType::Corner),
            Self::CornerCurl => (KnotType::Corner, KnotType::Corner, KnotType::Right),
        }
    }
}

/// A straight direction a directed pair can head in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    RightUp,
    RightDown,
    LeftUp,
    LeftDown,
}

impl Direction {
    #[must_use]
    pub const fn vector(self) -> Vec2 {
        match self {
            Self::Up => Vec2::new(0.0, 1.0),
            Self::Down => Vec2::new(0.0, -1.0),
            Self::Left => Vec2::new(-1.0, 0.0),
            Self::Right => Vec2::new(1.0, 0.0),
            Self::RightUp => Vec2::new(1.0, 1.0),
            Self::RightDown => Vec2::new(1.0, -1.0),
            Self::LeftUp => Vec2::new(-1.0, 1.0),
            Self::LeftDown => Vec2::new(-1.0, -1.0),
        }
    }
}

/// Where along a directed pair the knots sit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    Start,
    Mid,
    End,
}

/// The builders, bound to one set of metrics.
#[derive(Clone, Copy, Debug)]
pub struct Kit {
    pub stroke: f64,
    pub superness: f64,
    pub contrast: f64,
    pub tiny: f64,
}

impl Kit {
    #[must_use]
    pub const fn new(stroke: f64, superness: f64, contrast: f64, tiny: f64) -> Self {
        Self {
            stroke,
            superness,
            contrast,
            tiny,
        }
    }

    /// A plain knot builder by name: `g4`, `g2`, `corner`, `flat` or `curl`.
    #[must_use]
    pub fn simple(&self, name: &str) -> Option<KnotType> {
        Some(match name {
            "g4" => KnotType::G4,
            "g2" => KnotType::G2,
            "corner" => KnotType::Corner,
            // `flat` and `curl` are the left and right knot types.
            "flat" => KnotType::Left,
            "curl" => KnotType::Right,
            _ => return None,
        })
    }

    /// A plain knot.
    #[must_use]
    pub const fn knot(&self, ty: KnotType, x: f64, y: f64, af: Af) -> Item {
        Item::Control(Control::Knot(crate::control::UserControlKnot {
            ty,
            x,
            y,
            af,
        }))
    }

    /// A directed knot pair, two knots either side of the centre.
    #[must_use]
    pub fn pair(
        &self,
        family: Family,
        direction: Direction,
        position: Position,
        x: f64,
        y: f64,
        af: Af,
    ) -> Item {
        let (pre, center, post) = family.types();
        let (d_pre, d_post) = match position {
            Position::Start => (0.0, self.tiny),
            Position::Mid => (-0.5 * self.tiny, 0.5 * self.tiny),
            Position::End => (-self.tiny, 0.0),
        };

        Item::Control(Control::Pair(crate::control::UserCloseKnotPair {
            center: crate::control::UserControlKnot {
                ty: center,
                x,
                y,
                af,
            },
            ty_pre: pre,
            ty_post: post,
            dir: direction.vector(),
            d_pre,
            d_post,
        }))
    }

    /// A knot that contributes nothing but still splits interpolation.
    #[must_use]
    pub const fn virt(&self, x: f64, y: f64, af: Af) -> Item {
        Item::Control(Control::Virtual(crate::control::UserControlKnot {
            ty: KnotType::Corner,
            x,
            y,
            af,
        }))
    }

    #[must_use]
    pub const fn close(&self) -> Item {
        Item::Close
    }

    #[must_use]
    pub const fn end(&self) -> Item {
        Item::End
    }

    /// The default number of steps an arc interpolator uses.
    pub const DEFAULT_STEPS: usize = 6;

    #[must_use]
    pub fn archv(&self, samples: Option<usize>, superness: Option<f64>) -> Item {
        Item::Control(Control::Interpolator(Interpolator::ArcHv {
            steps: samples.unwrap_or(Self::DEFAULT_STEPS),
            superness: superness.unwrap_or(self.superness),
        }))
    }

    #[must_use]
    pub fn arcvh(&self, samples: Option<usize>, superness: Option<f64>) -> Item {
        Item::Control(Control::Interpolator(Interpolator::ArcVh {
            steps: samples.unwrap_or(Self::DEFAULT_STEPS),
            superness: superness.unwrap_or(self.superness),
        }))
    }

    /// A knot mixed between the two around it.
    ///
    /// An interpolator with no amendment marks its knots unimportant, which is
    /// what upstream's `fallback(raf, unimportant)` does.
    #[must_use]
    pub fn also_thru(&self, rx: f64, ry: f64, af: Af) -> Item {
        Item::Control(Control::Interpolator(Interpolator::SimpleMix {
            ty: KnotType::G4,
            rx,
            ry,
            dx: 0.0,
            dy: 0.0,
            af: or_unimportant(af),
        }))
    }

    /// Several knots mixed between the two around them.
    #[must_use]
    pub fn also_thru_them(&self, rs: Vec<(f64, f64, f64)>, af: Af, ty: KnotType) -> Item {
        Item::Control(Control::Interpolator(Interpolator::MultiMix {
            rs,
            af: or_unimportant(af),
            ty,
        }))
    }

    /// A list of controls resolved when the sequence is flattened.
    #[must_use]
    pub fn decor(&self, items: Vec<Item>) -> Item {
        Item::Control(Control::Interpolator(Interpolator::Decor(
            items
                .into_iter()
                .filter_map(|item| match item {
                    Item::Control(control) => Some(control),
                    _ => None,
                })
                .collect(),
        )))
    }

    // Amendments.

    #[must_use]
    pub fn widths(&self, l: f64, r: f64) -> Af {
        vec![Amendment::Widths { l, r }]
    }

    #[must_use]
    pub fn widths_lhs(&self, w: Option<f64>) -> Af {
        vec![Amendment::Widths {
            l: w.unwrap_or(self.stroke),
            r: 0.0,
        }]
    }

    #[must_use]
    pub fn widths_rhs(&self, w: Option<f64>) -> Af {
        vec![Amendment::Widths {
            l: 0.0,
            r: w.unwrap_or(self.stroke),
        }]
    }

    #[must_use]
    pub fn widths_center(&self, w: Option<f64>) -> Af {
        let half = w.unwrap_or(self.stroke) / 2.0;
        vec![Amendment::Widths { l: half, r: half }]
    }

    #[must_use]
    pub fn widths_heading(&self, l: f64, r: f64, dir: Vec2) -> Af {
        vec![Amendment::WidthsHeading { l, r, dir }]
    }

    #[must_use]
    pub fn widths_lhs_heading(&self, w: Option<f64>, dir: Vec2) -> Af {
        vec![Amendment::WidthsHeading {
            l: w.unwrap_or(self.stroke),
            r: 0.0,
            dir,
        }]
    }

    #[must_use]
    pub fn widths_rhs_heading(&self, w: Option<f64>, dir: Vec2) -> Af {
        vec![Amendment::WidthsHeading {
            l: 0.0,
            r: w.unwrap_or(self.stroke),
            dir,
        }]
    }

    #[must_use]
    pub fn widths_center_heading(&self, w: Option<f64>, dir: Vec2) -> Af {
        let half = w.unwrap_or(self.stroke) / 2.0;
        vec![Amendment::WidthsHeading {
            l: half,
            r: half,
            dir,
        }]
    }

    #[must_use]
    pub fn heading(&self, dir: Vec2) -> Af {
        vec![Amendment::Heading(dir)]
    }

    #[must_use]
    pub fn unimportant(&self) -> Af {
        vec![Amendment::Unimportant]
    }

    #[must_use]
    pub fn important(&self) -> Af {
        vec![Amendment::Important]
    }

    #[must_use]
    pub fn disable_contrast(&self) -> Af {
        vec![Amendment::DisableContrast]
    }

    /// Runs a sequence and returns the collector it filled.
    #[must_use]
    pub fn collect(&self, items: Vec<Item>) -> Collector {
        let mut flattener = Flattener::new();
        for item in items {
            flattener.add(item).expect("the sequence is well formed");
        }
        flattener.flatten();
        let mut collector = Collector::new(self.contrast);
        flattener.pipe(&mut collector);

        collector
    }

    /// A di-spiro shape: what `dispiro` builds.
    #[must_use]
    pub fn dispiro(&self, items: Vec<Item>) -> Dispiro {
        let collector = self.collect(items);
        Dispiro {
            contrast: collector.contrast,
            closed: collector.closed,
            knots: collector.knots,
        }
    }

    /// An outline shape: what `spiro-outline` builds.
    #[must_use]
    pub fn spiro_outline(&self, items: Vec<Item>) -> SpiroOutline {
        let collector = self.collect(items);
        SpiroOutline {
            closed: collector.closed,
            knots: collector
                .knots
                .iter()
                .map(|knot| MonoKnot::new(knot.ty, knot.x, knot.y, knot.unimportant))
                .collect(),
        }
    }
}

/// Upstream's `fallback(raf, unimportant)`.
fn or_unimportant(af: Af) -> Af {
    if af.is_empty() {
        vec![Amendment::Unimportant]
    } else {
        af
    }
}

/// The knots a `spiro-outline` sequence produced.
#[derive(Clone, Debug)]
pub struct SpiroOutline {
    pub closed: bool,
    pub knots: Vec<MonoKnot>,
}

impl crate::glyph::Component for SpiroOutline {
    fn apply_to_glyph(
        &self,
        glyph: &mut crate::glyph::Glyph,
        _copy_anchors: bool,
        _copy_width: bool,
    ) {
        glyph.include_geometry(Geometry::Spiro {
            gizmo: glyph.gizmo,
            closed: self.closed,
            knots: self.knots.clone(),
        });
    }
}

impl crate::glyph::Component for Dispiro {
    fn apply_to_glyph(
        &self,
        glyph: &mut crate::glyph::Glyph,
        _copy_anchors: bool,
        _copy_width: bool,
    ) {
        glyph.include_geometry(self.geometry(glyph.gizmo));
    }
}

/// The knots a `dispiro` sequence produced.
#[derive(Clone, Debug)]
pub struct Dispiro {
    pub contrast: f64,
    pub closed: bool,
    pub knots: Vec<lemmata_geom::BiKnot>,
}

impl Dispiro {
    /// The geometry tree node for this shape.
    #[must_use]
    pub fn geometry(&self, gizmo: kurbo::Affine) -> Geometry {
        Geometry::DiSpiro {
            gizmo,
            contrast: self.contrast,
            closed: self.closed,
            knots: self.knots.clone(),
        }
    }
}
