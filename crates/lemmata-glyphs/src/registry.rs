//! The glyphs the port builds, by name.
//!
//! Each ported definition enumerates what it builds. This is the one list a
//! caller needs: the browser, the CLI, and the capture comparison all read it,
//! so a glyph that exists in a module cannot be missing from the font.
//!
//! A name is the one the build gives the glyph, which is what a plan's variant
//! selection and a capture's entries use.

use lemmata_geom::Geometry;
use lemmata_ptl::letter_model::DivFrame;
use lemmata_ptl::metrics::Metrics;
use lemmata_ptl::shapes::Design;
use lemmata_ptl::spiro_kit::Kit;

use crate::letter::latin::{
    lower_a, lower_b, lower_d, lower_f, lower_h, lower_il, lower_j, lower_m, lower_n, lower_p, u,
    upper_b, upper_d, upper_e, upper_f, upper_h, upper_i, upper_j, upper_l, upper_m, upper_n,
    upper_p, upper_q, upper_t, upper_y, v,
};
use crate::symbol::math::dots_and_colons::DotsAndColons;
use crate::symbol::punctuation::asterisk::Asterisk;
use crate::symbol::punctuation::brackets::Brackets;

/// Every glyph the ported definitions build, by name.
#[must_use]
pub fn build(metrics: &Metrics, design: &Design) -> Vec<(String, Geometry)> {
    build_at(metrics, design, 1.0)
}

/// Every glyph the ported definitions build, at a frame scale.
///
/// The scale is `DivFrame`'s advance scale: `1.0` is the glyph as the font draws
/// it, `1 + 1/12` is upstream's extension form and `1 - 1/12` its shrink. A
/// texture form is a glyph re-drawn in that frame, not a transformed outline,
/// which is why a dot stays round.
#[must_use]
pub fn build_at(metrics: &Metrics, design: &Design, scale: f64) -> Vec<(String, Geometry)> {
    let df = DivFrame::new(metrics.clone(), Some(scale), None, None);
    let kit = Kit::new(
        metrics.stroke,
        design.superness,
        metrics.contrast,
        metrics.tiny,
    );
    let mut out = Vec::new();
    out.extend(
        Asterisk {
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(DotsAndColons { metrics, kit: &kit }.glyphs());
    out.extend(
        Brackets {
            df: &df,
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        lower_a::LowerA {
            df: &df,
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        lower_b::LowerB {
            df: &df,
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        lower_d::LowerD {
            df: &df,
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        lower_f::LowerF {
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        lower_h::LowerH {
            df: &df,
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        lower_il::LowerIl {
            df: &df,
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        lower_j::LowerJ {
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        lower_m::LowerM {
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        lower_n::LowerN {
            df: &df,
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        lower_p::LowerP {
            df: &df,
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        upper_t::UpperT {
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        u::UpperU {
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        upper_y::UpperY {
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        v::LowerV {
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        upper_b::UpperB {
            df: &df,
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        upper_d::UpperD {
            df: &df,
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        upper_m::UpperM {
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        upper_n::UpperN {
            df: &df,
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        upper_p::UpperP {
            df: &df,
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        upper_q::UpperQ {
            df: &df,
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        upper_e::UpperE {
            df: &df,
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        upper_f::UpperF {
            df: &df,
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        upper_h::UpperH {
            df: &df,
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        upper_i::UpperI {
            df: &df,
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        upper_j::UpperJ {
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );
    out.extend(
        upper_l::UpperL {
            df: &df,
            metrics,
            kit: &kit,
            design,
        }
        .glyphs(),
    );

    out
}
