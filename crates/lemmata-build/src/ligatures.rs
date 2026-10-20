//! The ligature rules the font carries, and the glyphs they draw.
//!
//! Upstream generates the rules while it builds, so the only place the result
//! exists as data is the reference font. `cargo xtask ligations --fixture`
//! shapes the sequences the ligation set names and writes both: the contextual
//! rules, by glyph name, and the outline of every glyph they mention.
//!
//! The fixture is embedded because both front ends must produce the same font.
//! A page and a native build that disagree about ligatures are the same
//! divergence the parity pair exists to catch.

use std::sync::OnceLock;

use lemmata_geom::{QuadifyKind, QuadifyPoint};

use crate::font::{GlyphData, Ligature};

/// The fixture, generated from the reference build.
const FIXTURE: &str = include_str!("../../../reference/ligatures.json");

type Fixture = (Vec<Ligature>, Vec<GlyphData>);

fn fixture() -> &'static Fixture {
    static PARSED: OnceLock<Fixture> = OnceLock::new();

    PARSED.get_or_init(|| {
        let value: serde_json::Value =
            serde_json::from_str(FIXTURE).expect("the ligature fixture parses");
        let names = |value: &serde_json::Value| -> Vec<Vec<String>> {
            value
                .as_array()
                .map(|positions| {
                    positions
                        .iter()
                        .filter_map(|position| position.as_array())
                        .map(|position| {
                            position
                                .iter()
                                .filter_map(|name| name.as_str().map(str::to_string))
                                .collect()
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        let rules = value["rules"]
            .as_array()
            .expect("the fixture carries rules")
            .iter()
            .map(|rule| Ligature {
                backtrack: names(&rule["backtrack"]),
                input: rule["input"].as_str().expect("an input").to_string(),
                lookahead: names(&rule["lookahead"]),
                target: rule["target"].as_str().expect("a target").to_string(),
            })
            .collect();
        let glyphs = value["glyphs"]
            .as_object()
            .expect("the fixture carries glyphs")
            .iter()
            .map(|(name, glyph)| GlyphData {
                name: name.clone(),
                unicodes: Vec::new(),
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                advance: glyph["advance"]
                    .as_f64()
                    .unwrap_or(0.0)
                    .round()
                    .clamp(0.0, f64::from(u16::MAX)) as u16,
                contours: glyph["contours"]
                    .as_array()
                    .map(|contours| {
                        contours
                            .iter()
                            .filter_map(|contour| contour.as_array())
                            .map(|contour| {
                                contour
                                    .iter()
                                    .map(|point| QuadifyPoint {
                                        kind: if point["quadratic"].as_bool().unwrap_or(false) {
                                            QuadifyKind::Quadratic
                                        } else {
                                            QuadifyKind::Corner
                                        },
                                        x: point["x"].as_f64().unwrap_or(0.0),
                                        y: point["y"].as_f64().unwrap_or(0.0),
                                    })
                                    .collect()
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            })
            .collect();

        (rules, glyphs)
    })
}

/// Adds the ligature glyphs a set does not already hold, and returns the rules.
///
/// A rule that names a glyph the caller did not supply is dropped by the build,
/// so a subset that removes one costs that ligature and nothing else.
pub fn add(glyphs: &mut Vec<GlyphData>) -> &'static [Ligature] {
    let (rules, extra) = fixture();
    for glyph in extra {
        if !glyphs.iter().any(|existing| existing.name == glyph.name) {
            glyphs.push(glyph.clone());
        }
    }

    rules
}
