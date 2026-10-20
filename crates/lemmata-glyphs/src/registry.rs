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
    lower_a, lower_b, lower_d, lower_h, lower_n, lower_p, upper_b, upper_d, upper_e, upper_f,
    upper_h, upper_i, upper_l, upper_q,
};

/// Every glyph the ported definitions build, by name.
#[must_use]
pub fn build(metrics: &Metrics, design: &Design) -> Vec<(String, Geometry)> {
    let df = DivFrame::new(metrics.clone(), Some(1.0), None, None);
    let kit = Kit::new(
        metrics.stroke,
        design.superness,
        metrics.contrast,
        metrics.tiny,
    );
    let mut out = Vec::new();
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
        lower_h::LowerH {
            df: &df,
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
