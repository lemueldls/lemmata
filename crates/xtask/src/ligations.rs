//! Reads the ligature substitutions a font carries.
//!
//! The ligation definitions are generated at build time from
//! `params/ligation-set.toml`, so the only place the result exists as data is
//! the font the build produced. This walks its GSUB and reports, by glyph name,
//! the sequences a shaper would join and the glyph each becomes.
//!
//! Upstream writes a ligation as a chaining rule that applies a ligature
//! lookup, so both kinds are walked: the chaining rule adds the neighbours a
//! sequence must not have, and the nested lookup carries the sequence itself.

use std::collections::BTreeSet;

use read_fonts::tables::gsub::{SubstitutionLookup, SubstitutionSubtables};
use read_fonts::tables::layout::LookupList;
use read_fonts::types::{GlyphId16, Tag};
use read_fonts::{FontRef, TableProvider};

/// One contextual rule, as glyph names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rule {
    /// One entry per position, nearest first.
    pub backtrack: Vec<Vec<String>>,
    pub input: String,
    /// One entry per position, nearest first.
    pub lookahead: Vec<Vec<String>>,
    pub target: String,
}

/// One ligature, as glyph names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ligature {
    pub components: Vec<String>,
    pub target: String,
}

struct Reader<'a> {
    font: FontRef<'a>,
}

impl Reader<'_> {
    fn name(&self, id: GlyphId16) -> String {
        self.font
            .post()
            .ok()
            .and_then(|post| post.glyph_name(id).map(str::to_string))
            .unwrap_or_else(|| format!("gid{}", id.to_u32()))
    }
}

fn coverage_glyphs(table: &read_fonts::tables::layout::CoverageTable<'_>) -> Vec<GlyphId16> {
    table.iter().collect()
}

/// The ligatures the `calt` feature reaches.
///
/// # Errors
///
/// Returns a message when the font cannot be read or carries no GSUB.
pub fn read(path: &str) -> Result<Vec<Ligature>, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("reading {path}: {error}"))?;
    let font = FontRef::new(&bytes).map_err(|error| format!("{path}: {error}"))?;
    let gsub = font.gsub().map_err(|error| format!("{path}: {error}"))?;
    let reader = Reader { font };
    let lookups = gsub.lookup_list().map_err(|error| format!("{error}"))?;

    // The `calt` feature names the lookups a shaper applies by default. A font
    // with no `calt` has no ligatures to report.
    let feature_list = gsub.feature_list().map_err(|error| format!("{error}"))?;
    let Some(record) = feature_list
        .feature_records()
        .iter()
        .find(|record| record.feature_tag() == Tag::new(b"calt"))
    else {
        return Ok(Vec::new());
    };
    let feature = record
        .feature(feature_list.offset_data())
        .map_err(|error| format!("{error}"))?;

    if std::env::var_os("LEMMATA_LIGATIONS_DEBUG").is_some() {
        for record in feature_list.feature_records() {
            eprintln!("feature {}", record.feature_tag());
        }
        for (index, lookup) in lookups.lookups().iter().enumerate() {
            if let Ok(lookup) = lookup {
                eprintln!("lookup {index}: type {}", lookup.lookup_type());
            }
        }
    }
    if std::env::var_os("LEMMATA_LIGATIONS_DEBUG").is_some() {
        for index in feature.lookup_list_indices() {
            let index = usize::from(index.get());
            let kind = lookups
                .lookups()
                .get(index)
                .map(|lookup| lookup.lookup_type())
                .unwrap_or(u16::MAX);
            eprintln!("calt lookup {index}: type {kind}");
        }
    }
    let mut found = Vec::new();
    let mut seen = BTreeSet::new();
    for index in feature.lookup_list_indices() {
        let index = usize::from(index.get());
        if seen.insert(index) {
            walk(&reader, &lookups, index, &mut found, &mut BTreeSet::new())
                .map_err(|error| format!("{error}"))?;
        }
    }
    found.sort();
    found.dedup();

    Ok(found)
}

fn walk(
    reader: &Reader<'_>,
    lookups: &LookupList<'_, SubstitutionLookup<'_>>,
    index: usize,
    out: &mut Vec<Ligature>,
    visiting: &mut BTreeSet<usize>,
) -> Result<(), read_fonts::ReadError> {
    if !visiting.insert(index) {
        return Ok(());
    }
    let lookup = lookups.lookups().get(index)?;
    match lookup.subtables()? {
        SubstitutionSubtables::Ligature(subtables) => {
            for subtable in subtables.iter() {
                let subtable = subtable?;
                let covered = coverage_glyphs(&subtable.coverage()?);
                for (set_index, set) in subtable.ligature_sets().iter().enumerate() {
                    let Some(first) = covered.get(set_index) else {
                        continue;
                    };
                    for ligature in set?.ligatures().iter() {
                        let ligature = ligature?;
                        let mut components = vec![reader.name(*first)];
                        components.extend(
                            ligature
                                .component_glyph_ids()
                                .iter()
                                .map(|id| reader.name(id.get())),
                        );
                        out.push(Ligature {
                            components,
                            target: reader.name(ligature.ligature_glyph()),
                        });
                    }
                }
            }
        }
        SubstitutionSubtables::ChainContextual(subtables) => {
            for subtable in subtables.iter() {
                let subtable = subtable?;
                let read_fonts::tables::layout::ChainedSequenceContext::Format3(context) = subtable
                else {
                    continue;
                };
                let inputs: Vec<Vec<GlyphId16>> = context
                    .input_coverages()
                    .iter()
                    .filter_map(|coverage| coverage.ok())
                    .map(|coverage| coverage_glyphs(&coverage))
                    .collect();
                if inputs.is_empty() || inputs.iter().any(Vec::is_empty) {
                    continue;
                }
                // The rule applies its substitution to the first input and
                // blanks the rest, so the pair that names the first input is
                // the sequence's ligature.
                let Some(record) = context
                    .seq_lookup_records()
                    .iter()
                    .find(|record| record.sequence_index() == 0)
                else {
                    continue;
                };
                let applied = lookups
                    .lookups()
                    .get(usize::from(record.lookup_list_index()))?;
                let SubstitutionSubtables::Single(singles) = applied.subtables()? else {
                    continue;
                };
                let mut pairs = Vec::new();
                for single in singles.iter() {
                    let single = single?;
                    pairs.extend(single_pairs(&single)?);
                }
                // The coverages name the glyphs that fit each position, so the
                // sequences are their product. Upstream writes one glyph per
                // position; a wider coverage is enumerated up to a bound.
                for components in combinations(&inputs, 64) {
                    let first = components[0];
                    let Some((_, target)) = pairs.iter().find(|(source, _)| *source == first)
                    else {
                        continue;
                    };
                    out.push(Ligature {
                        components: components.iter().map(|id| reader.name(*id)).collect(),
                        target: reader.name(*target),
                    });
                }
            }
        }
        _ => {}
    }
    visiting.remove(&index);

    Ok(())
}

/// `SingleSubst`: each covered glyph and the glyph it becomes.
fn single_pairs(
    table: &read_fonts::tables::gsub::SingleSubst<'_>,
) -> Result<Vec<(GlyphId16, GlyphId16)>, read_fonts::ReadError> {
    match table {
        read_fonts::tables::gsub::SingleSubst::Format1(table) => {
            let covered = coverage_glyphs(&table.coverage()?);
            let delta = table.delta_glyph_id();
            Ok(covered
                .into_iter()
                .map(|glyph| {
                    let target =
                        i32::from(u16::try_from(glyph.to_u32()).unwrap_or(0)) + i32::from(delta);
                    let target = GlyphId16::new(u16::try_from(target).unwrap_or(0));
                    (glyph, target)
                })
                .collect())
        }
        read_fonts::tables::gsub::SingleSubst::Format2(table) => {
            let covered = coverage_glyphs(&table.coverage()?);
            Ok(covered
                .into_iter()
                .zip(table.substitute_glyph_ids().iter().map(|id| id.get()))
                .collect())
        }
    }
}

/// The product of the coverages, bounded so a wide coverage cannot blow up.
fn combinations(inputs: &[Vec<GlyphId16>], limit: usize) -> Vec<Vec<GlyphId16>> {
    let mut out = vec![Vec::new()];
    for coverage in inputs {
        let mut next = Vec::new();
        for prefix in &out {
            for glyph in coverage {
                if next.len() >= limit {
                    return next;
                }
                let mut combined = prefix.clone();
                combined.push(*glyph);
                next.push(combined);
            }
        }
        out = next;
    }

    out
}

/// The `samples` arrays in `ligation-set.toml`, in order.
///
/// The file is a table of groups, and each group lists the strings a shaper
/// should join. Reading the strings is all this needs of the format.
fn samples(parameters: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = parameters;
    while let Some(start) = rest.find("samples") {
        rest = &rest[start + "samples".len()..];
        let Some(open) = rest.find('[') else {
            break;
        };
        rest = &rest[open + 1..];
        let Some(close) = rest.find(']') else {
            break;
        };
        let block = &rest[..close];
        rest = &rest[close + 1..];
        let mut chars = block.chars();
        while let Some(character) = chars.next() {
            if character != '"' {
                continue;
            }
            let mut text = String::new();
            while let Some(character) = chars.next() {
                match character {
                    '"' => break,
                    '\\' => {
                        if let Some(escaped) = chars.next() {
                            text.push(escaped);
                        }
                    }
                    other => text.push(other),
                }
            }
            out.push(text);
        }
    }

    out
}

/// The glyph names a sample shapes to, with and without `calt`.
fn shape(
    shaper: &harfrust::ShaperFont<'_, '_>,
    reader: &Reader<'_>,
    text: &str,
    calt: bool,
) -> Vec<String> {
    let mut buffer = harfrust::Buffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    let features = [
        harfrust::Feature {
            tag: harfrust::Tag::new(b"calt"),
            value: u32::from(calt),
            start: 0,
            end: u32::MAX,
        },
        harfrust::Feature {
            tag: harfrust::Tag::new(b"dlig"),
            value: u32::from(calt),
            start: 0,
            end: u32::MAX,
        },
    ];
    if std::env::var_os("LEMMATA_LIGATIONS_DEFAULT").is_some() {
        harfrust::shape(shaper, &mut buffer, harfrust::ShapeOptions::default())
            .expect("shape the run");
    } else {
        harfrust::shape(
            shaper,
            &mut buffer,
            harfrust::ShapeOptions::default().features(&features),
        )
        .expect("shape the run");
    }

    if std::env::var_os("LEMMATA_LIGATIONS_DEBUG").is_some() && text == "->" && calt {
        for (info, position) in buffer.glyph_infos().iter().zip(buffer.glyph_positions()) {
            eprintln!(
                "  gid {} advance {} offset {}",
                info.glyph_id, position.x_advance, position.x_offset
            );
        }
    }
    buffer
        .glyph_infos()
        .iter()
        .map(|info| reader.name(GlyphId16::new(u16::try_from(info.glyph_id).unwrap_or(0))))
        .collect()
}

/// Extracts the ligatures by shaping the samples the ligation set names.
///
/// # Errors
///
/// Returns a message when the font or the parameter file cannot be read.
pub fn read_samples(font_path: &str, parameters: &str) -> Result<Vec<Ligature>, String> {
    let bytes =
        std::fs::read(font_path).map_err(|error| format!("reading {font_path}: {error}"))?;
    let blob: harfrust::font::Blob = bytes.clone().into();
    let font = harfrust::Font::new(blob, 0)
        .ok_or_else(|| format!("{font_path}: the shaper cannot read the font"))?;
    let shaper = harfrust::ShaperFont::new(&font);
    let reader = Reader {
        font: FontRef::new(&bytes).map_err(|error| format!("{font_path}: {error}"))?,
    };

    let samples = samples(parameters);
    if std::env::var_os("LEMMATA_LIGATIONS_DEBUG").is_some() {
        eprintln!("samples: {}", samples.len());
    }
    let mut found = Vec::new();
    for text in samples {
        let plain = shape(&shaper, &reader, &text, false);
        let joined = shape(&shaper, &reader, &text, true);
        if std::env::var_os("LEMMATA_LIGATIONS_DEBUG").is_some() && text == "->" {
            eprintln!("-> plain {plain:?} joined {joined:?}");
        }
        if plain == joined {
            continue;
        }
        // The joined run names one composed glyph. When it keeps the input
        // length, the differing position is the one that changed.
        let target = joined
            .iter()
            .zip(&plain)
            .find(|(joined, plain)| joined != plain)
            .map(|(joined, _)| joined.clone())
            .or_else(|| joined.first().cloned());
        let Some(target) = target else {
            continue;
        };
        found.push(Ligature {
            components: plain,
            target,
        });
    }
    found.sort();
    found.dedup();

    Ok(found)
}

/// The contextual rules the `calt` feature applies, as glyph names.
///
/// Upstream draws a ligature by substituting each character for a joined form
/// that meets its neighbour, so a rule is one character with the neighbours
/// that allow the form. That is a chaining contextual lookup, and it is what
/// this walks.
///
/// # Errors
///
/// Returns a message when the font cannot be read or carries no GSUB.
pub fn read_rules(path: &str) -> Result<Vec<Rule>, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("reading {path}: {error}"))?;
    let font = FontRef::new(&bytes).map_err(|error| format!("{path}: {error}"))?;
    let gsub = font.gsub().map_err(|error| format!("{path}: {error}"))?;
    let reader = Reader { font };
    let lookups = gsub.lookup_list().map_err(|error| format!("{error}"))?;
    let feature_list = gsub.feature_list().map_err(|error| format!("{error}"))?;
    let record = feature_list
        .feature_records()
        .iter()
        .find(|record| record.feature_tag() == Tag::new(b"calt"))
        .ok_or("the font carries no calt")?;
    let feature = record
        .feature(feature_list.offset_data())
        .map_err(|error| format!("{error}"))?;

    let mut rules = Vec::new();
    for index in feature.lookup_list_indices() {
        collect_rules(&reader, &lookups, usize::from(index.get()), &mut rules)
            .map_err(|error| format!("{error}"))?;
    }
    rules.sort();
    rules.dedup();

    Ok(rules)
}

fn collect_rules(
    reader: &Reader<'_>,
    lookups: &LookupList<'_, SubstitutionLookup<'_>>,
    index: usize,
    out: &mut Vec<Rule>,
) -> Result<(), read_fonts::ReadError> {
    let lookup = lookups.lookups().get(index)?;
    let SubstitutionSubtables::ChainContextual(subtables) = lookup.subtables()? else {
        return Ok(());
    };
    for subtable in subtables.iter() {
        let subtable = subtable?;
        let read_fonts::tables::layout::ChainedSequenceContext::Format3(context) = subtable else {
            continue;
        };
        let segments: Vec<Vec<GlyphId16>> = context
            .input_coverages()
            .iter()
            .filter_map(|coverage| coverage.ok())
            .map(|coverage| coverage_glyphs(&coverage))
            .collect();
        let backtrack: Vec<GlyphId16> = context
            .backtrack_coverages()
            .iter()
            .filter_map(|coverage| coverage.ok())
            .flat_map(|coverage| coverage_glyphs(&coverage))
            .collect();
        let lookahead: Vec<GlyphId16> = context
            .lookahead_coverages()
            .iter()
            .filter_map(|coverage| coverage.ok())
            .flat_map(|coverage| coverage_glyphs(&coverage))
            .collect();
        if segments.is_empty() || segments.iter().any(Vec::is_empty) {
            continue;
        }
        for record in context.seq_lookup_records() {
            let position = usize::from(record.sequence_index());
            let Some(inputs) = segments.get(position) else {
                continue;
            };
            let applied = lookups
                .lookups()
                .get(usize::from(record.lookup_list_index()))?;
            let SubstitutionSubtables::Single(singles) = applied.subtables()? else {
                continue;
            };
            let mut pairs = Vec::new();
            for single in singles.iter() {
                let single = single?;
                pairs.extend(single_pairs(&single)?);
            }
            // OpenType orders a backtrack array nearest first, so the positions
            // before the input are reversed.
            let mut before: Vec<Vec<String>> = segments[..position]
                .iter()
                .rev()
                .map(|coverage| coverage.iter().map(|glyph| reader.name(*glyph)).collect())
                .collect();
            before.extend(
                backtrack
                    .iter()
                    .rev()
                    .map(|glyph| vec![reader.name(*glyph)]),
            );
            let mut after: Vec<Vec<String>> = segments[position + 1..]
                .iter()
                .map(|coverage| coverage.iter().map(|glyph| reader.name(*glyph)).collect())
                .collect();
            after.extend(lookahead.iter().map(|glyph| vec![reader.name(*glyph)]));
            for input in inputs {
                let Some((_, target)) = pairs.iter().find(|(source, _)| source == input) else {
                    continue;
                };
                out.push(Rule {
                    backtrack: before.clone(),
                    input: reader.name(*input),
                    lookahead: after.clone(),
                    target: reader.name(*target),
                });
            }
        }
    }

    Ok(())
}

/// Writes the contextual rules as JSON, with every glyph they name.
///
/// # Errors
///
/// Returns a message when the font cannot be read or the file cannot be written.
pub fn run_rules(font: &str, out: &str) -> Result<(), String> {
    let rules = read_rules(font)?;
    let mut glyphs: BTreeSet<String> = BTreeSet::new();
    for rule in &rules {
        for position in &rule.backtrack {
            glyphs.extend(position.iter().cloned());
        }
        glyphs.insert(rule.input.clone());
        for position in &rule.lookahead {
            glyphs.extend(position.iter().cloned());
        }
        glyphs.insert(rule.target.clone());
    }
    let json = serde_json::json!({
        "rules": rules
            .iter()
            .map(|rule| serde_json::json!({
                "backtrack": rule.backtrack,
                "input": rule.input,
                "lookahead": rule.lookahead,
                "target": rule.target,
            }))
            .collect::<Vec<_>>(),
        "glyphs": glyphs,
    });
    std::fs::write(out, serde_json::to_string_pretty(&json).unwrap())
        .map_err(|error| format!("writing {out}: {error}"))?;
    eprintln!(
        "ligations: {} rules over {} glyphs",
        rules.len(),
        glyphs.len()
    );

    Ok(())
}

/// Writes the ligatures as JSON.
///
/// # Errors
///
/// Returns a message when the font cannot be read or the file cannot be written.
pub fn run(font: &str, out: &str, parameters: Option<&str>) -> Result<(), String> {
    if parameters == Some("--rules") {
        return run_rules(font, out);
    }
    let ligatures = match parameters {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .map_err(|error| format!("reading {path}: {error}"))?;
            read_samples(font, &text)?
        }
        None => read(font)?,
    };
    let first_components: BTreeSet<&str> = ligatures
        .iter()
        .filter_map(|ligature| ligature.components.first().map(String::as_str))
        .collect();
    let json = serde_json::json!({
        "ligatures": ligatures
            .iter()
            .map(|ligature| serde_json::json!({
                "components": ligature.components,
                "target": ligature.target,
            }))
            .collect::<Vec<_>>(),
    });
    std::fs::write(out, serde_json::to_string_pretty(&json).unwrap())
        .map_err(|error| format!("writing {out}: {error}"))?;
    eprintln!(
        "ligations: {} ligatures over {} first components",
        ligatures.len(),
        first_components.len()
    );

    Ok(())
}
