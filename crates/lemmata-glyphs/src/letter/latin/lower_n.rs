//! The lower-case `n` from `letter/latin/lower-n.ptl`.
//!
//! A stem and a shoulder, with a straight or tailed terminal and four serif
//! settings. Four bodies differ in how the shoulder's top closes.
//!
//! The `HTB` bodies, the Yat serifs, `engCap`, the hooks, the curly tail and the
//! derived glyphs need shapes outside this file and are not ported. The anchors
//! are not either.

// The arithmetic mirrors upstream's expression order and stays clear of fused
// multiply-add, which would round differently from JavaScript.
#![allow(clippy::suboptimal_flops)]

use lemmata_geom::Geometry;
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::serif::SerifFrame;
use lemmata_ptl::shapes::{self, Design};
use lemmata_ptl::shoulder::{NShoulder, n_shoulder_earless_corner, n_shoulder_earless_rounded};
use lemmata_ptl::spiro_kit::Kit;
use lemmata_ptl::util::join_camel;

/// The four bodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Eared,
    FlatTop,
    EarlessCorner,
    EarlessRounded,
}

/// The terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terminal {
    Straight,
    Tailed,
}

/// The four serif settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Serif {
    Serifless,
    TopLeftSerifed,
    MotionSerifed,
    Serifed,
}

/// The bodies, in the table's order.
pub const BODIES: &[(&str, Body)] = &[
    ("", Body::Eared),
    ("flatTop", Body::FlatTop),
    ("earlessCorner", Body::EarlessCorner),
    ("earlessRounded", Body::EarlessRounded),
];

/// The terminals, in the table's order.
pub const TERMINALS: &[(&str, Terminal)] = &[
    ("straight", Terminal::Straight),
    ("tailed", Terminal::Tailed),
];

/// The serifs, in the table's order.
pub const SERIFS: &[(&str, Serif)] = &[
    ("serifless", Serif::Serifless),
    ("topLeftSerifed", Serif::TopLeftSerifed),
    ("motionSerifed", Serif::MotionSerifed),
    ("serifed", Serif::Serifed),
];

/// The lower-case `n` under construction.
pub struct LowerN<'a> {
    pub df: &'a DivFrame,
    pub metrics: &'a Metrics,
    pub kit: &'a Kit,
    pub design: &'a Design,
}

impl LowerN<'_> {
    const fn left_sb(&self) -> f64 {
        self.df.sb
    }

    const fn right_sb(&self) -> f64 {
        self.df.width - self.df.sb
    }

    fn shoulder(&self, top: f64, bottom: f64, eared: bool, left_y0: f64) -> NShoulder {
        let metrics = self.metrics;

        NShoulder {
            // The eared body starts its shoulder one stroke in, so the stem and
            // the shoulder do not overlap at the join.
            left: if eared {
                self.left_sb() + metrics.stroke * metrics.hv_contrast
            } else {
                self.left_sb()
            },
            right: self.right_sb(),
            fine: metrics.shoulder_fine,
            top,
            bottom,
            ada: metrics.small_arch_depth_a,
            adb: metrics.small_arch_depth_b,
            stroke: metrics.stroke,
            left_y0,
        }
    }

    fn bar_y(&self, terminal: Terminal) -> f64 {
        if terminal == Terminal::Tailed {
            self.metrics.xh - self.metrics.small_arch_depth_b + self.metrics.o
        } else {
            0.0
        }
    }

    fn stem(&self, top: f64) -> Geometry {
        shapes::vbar_l(
            self.kit,
            self.metrics,
            self.left_sb(),
            0.0,
            top,
            self.metrics.stroke,
        )
        .geometry(kurbo::Affine::IDENTITY)
    }

    /// The stem and the shoulder, as the body draws them.
    #[must_use]
    pub fn body(&self, body: Body, terminal: Terminal) -> Vec<Geometry> {
        let metrics = self.metrics;
        let y_br = self.bar_y(terminal);
        let flat_y0 = (metrics.xh - metrics.small_arch_depth_a) - metrics.tiny;

        match body {
            Body::Eared => {
                let shoulder = self.shoulder(metrics.xh, y_br, true, flat_y0);
                vec![
                    self.stem(metrics.xh),
                    shoulder
                        .shape(self.kit, metrics, self.design)
                        .geometry(kurbo::Affine::IDENTITY),
                ]
            }
            Body::FlatTop => {
                let shoulder = self.shoulder(metrics.xh, y_br, false, flat_y0);
                vec![
                    self.stem(metrics.xh),
                    n_shoulder_earless_corner(self.kit, metrics, self.design, &shoulder, 0.0)
                        .geometry(kurbo::Affine::IDENTITY),
                ]
            }
            Body::EarlessCorner => {
                let shoulder = self.shoulder(metrics.xh, y_br, false, flat_y0);
                vec![
                    self.stem(metrics.xh - metrics.d_toothless_rise),
                    n_shoulder_earless_corner(
                        self.kit,
                        metrics,
                        self.design,
                        &shoulder,
                        metrics.d_toothless_rise,
                    )
                    .geometry(kurbo::Affine::IDENTITY),
                ]
            }
            Body::EarlessRounded => {
                let shoulder = self.shoulder(metrics.xh, y_br, false, 0.0);
                vec![
                    n_shoulder_earless_rounded(self.kit, metrics, self.design, &shoulder)
                        .geometry(kurbo::Affine::IDENTITY),
                ]
            }
        }
    }

    fn frame(&self, top: f64, bot: f64) -> SerifFrame {
        SerifFrame::from_df(self.metrics, self.df, top, bot, None, None, false)
    }

    /// The serifs the setting names.
    #[must_use]
    pub fn serifs(&self, body: Body, terminal: Terminal, serif: Serif) -> Vec<Geometry> {
        let metrics = self.metrics;
        let mut parts = Vec::new();
        if serif == Serif::Serifless {
            return parts;
        }
        // The earless bodies carry no top-left serif.
        if !matches!(body, Body::EarlessCorner | Body::EarlessRounded) {
            parts.push(self.frame(metrics.xh, 0.0).lt_outer(self.kit, metrics));
        }
        if serif == Serif::Serifed {
            parts.push(self.frame(metrics.ascender, 0.0).lb_full(self.kit, metrics));
        }
        if terminal == Terminal::Straight {
            let right = match serif {
                Serif::MotionSerifed => self
                    .frame(metrics.ascender, 0.0)
                    .rb_outer(self.kit, metrics),
                Serif::Serifed => self.frame(metrics.ascender, 0.0).rb_full(self.kit, metrics),
                _ => return parts,
            };
            parts.push(right);
        }

        parts
    }

    /// `n.<suffix>`.
    #[must_use]
    pub fn glyph(&self, body: Body, terminal: Terminal, serif: Serif) -> Geometry {
        let metrics = self.metrics;
        let mut parts = self.body(body, terminal);
        if terminal == Terminal::Tailed {
            parts.push(
                shapes::rightward_tailed_bar(
                    self.kit,
                    metrics,
                    self.df,
                    self.right_sb(),
                    0.0,
                    metrics.xh - metrics.small_arch_depth_b,
                    metrics.stroke,
                )
                .geometry(kurbo::Affine::IDENTITY),
            );
        }
        parts.extend(self.serifs(body, terminal, serif));

        Geometry::Combine(parts)
    }
}

/// The variant name `SuffixCfg.weave` gives a body, a terminal and a serif.
#[must_use]
pub fn suffix(body: &str, terminal: &str, serif: &str) -> String {
    join_camel(&join_camel(body, terminal), serif)
}

/// Every variant the table names, as `(suffix, body, terminal, serif)`.
#[must_use]
pub fn variants() -> Vec<(String, Body, Terminal, Serif)> {
    let mut out = Vec::with_capacity(BODIES.len() * TERMINALS.len() * SERIFS.len());
    for (body_key, body) in BODIES {
        for (terminal_key, terminal) in TERMINALS {
            for (serif_key, serif) in SERIFS {
                out.push((
                    suffix(body_key, terminal_key, serif_key),
                    *body,
                    *terminal,
                    *serif,
                ));
            }
        }
    }

    out
}

impl LowerN<'_> {
    /// Every glyph this definition builds, by name.
    #[must_use]
    pub fn glyphs(&self) -> Vec<(String, Geometry)> {
        let mut out = Vec::new();
        for (suffix, body, terminal, serif) in variants() {
            out.push((format!("n.{suffix}"), self.glyph(body, terminal, serif)));
        }

        out
    }
}
