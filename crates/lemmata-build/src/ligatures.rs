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

/// The reference targets the port can draw itself, by source name.
const SOURCES: &str = include_str!("../../../reference/ligature-sources.json");

type Fixture = (Vec<Ligature>, Vec<GlyphData>);

/// The source glyph the port draws for a captured target, when it has one.
#[must_use]
pub fn source_of(target: &str) -> Option<&'static str> {
    static MAP: OnceLock<std::collections::BTreeMap<String, String>> = OnceLock::new();
    let map =
        MAP.get_or_init(|| serde_json::from_str(SOURCES).expect("the ligature source map parses"));

    map.get(target).map(String::as_str)
}

/// Every target the port can draw, with its source name.
pub fn sources() -> impl Iterator<Item = (&'static str, &'static str)> {
    static MAP: OnceLock<std::collections::BTreeMap<String, String>> = OnceLock::new();
    let map =
        MAP.get_or_init(|| serde_json::from_str(SOURCES).expect("the ligature source map parses"));

    map.iter()
        .map(|(target, source)| (target.as_str(), source.as_str()))
}

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

/// The rules that can fire in a run whose glyph names are `text_names`, and
/// the glyphs they target.
///
/// A preview is rebuilt on every text change, so carrying the whole fixture
/// there costs more than it saves. A run with no ligature trigger gets no
/// rules and no extra glyphs.
#[must_use]
pub fn add_for(
    glyphs: &mut Vec<GlyphData>,
    text_names: &std::collections::BTreeSet<String>,
) -> Vec<Ligature> {
    let (rules, extra) = fixture();
    // The right half of a pair names the left half's joined form in its
    // context, so a rule becomes reachable only after the rule that produces
    // that form. Grow the set until it stops changing.
    let mut known = text_names.clone();
    let mut included = vec![false; rules.len()];
    let mut relevant = Vec::new();
    loop {
        let mut progress = false;
        for (index, rule) in rules.iter().enumerate() {
            if included[index] {
                continue;
            }
            let ready = known.contains(&rule.input)
                && rule
                    .lookahead
                    .iter()
                    .flatten()
                    .all(|name| known.contains(name))
                && rule
                    .backtrack
                    .iter()
                    .flatten()
                    .all(|name| known.contains(name));
            if ready {
                included[index] = true;
                known.insert(rule.target.clone());
                relevant.push(rule.clone());
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }
    let targets: std::collections::BTreeSet<&str> =
        relevant.iter().map(|rule| rule.target.as_str()).collect();
    for glyph in extra {
        if targets.contains(glyph.name.as_str())
            && !glyphs.iter().any(|existing| existing.name == glyph.name)
        {
            glyphs.push(glyph.clone());
        }
    }

    relevant
}
