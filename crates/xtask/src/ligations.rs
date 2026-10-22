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

use read_fonts::{
    FontRef, TableProvider,
    tables::{
        gsub::{SubstitutionLookup, SubstitutionSubtables},
        layout::LookupList,
    },
    types::{GlyphId16, Tag},
};
use skrifa::MetadataProvider;

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
) -> (Vec<String>, Vec<i32>) {
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
    harfrust::shape(
        shaper,
        &mut buffer,
        harfrust::ShapeOptions::default().features(&features),
    )
    .expect("shape the run");

    (
        buffer
            .glyph_infos()
            .iter()
            .map(|info| reader.name(GlyphId16::new(u16::try_from(info.glyph_id).unwrap_or(0))))
            .collect(),
        buffer
            .glyph_positions()
            .iter()
            .map(|position| position.x_advance)
            .collect(),
    )
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
    let mut found = Vec::new();
    for text in samples {
        let (plain, _) = shape(&shaper, &reader, &text, false);
        let (joined, _) = shape(&shaper, &reader, &text, true);
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

    Ok(order_by_dependency(&rules))
}

/// Orders rules so one whose context names a glyph another rule produces runs
/// after it. A shaper applies lookups in order and each substitution changes
/// the buffer, so the right half of a pair only matches once the left half has
/// become its join form.
fn order_by_dependency(rules: &[Rule]) -> Vec<Rule> {
    use std::collections::HashMap;

    let mut producers: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, rule) in rules.iter().enumerate() {
        producers
            .entry(rule.target.as_str())
            .or_default()
            .push(index);
    }
    let mut visited = vec![false; rules.len()];
    let mut out = Vec::with_capacity(rules.len());
    for index in 0..rules.len() {
        visit(index, rules, &producers, &mut visited, &mut out);
    }

    out
}

fn visit(
    index: usize,
    rules: &[Rule],
    producers: &std::collections::HashMap<&str, Vec<usize>>,
    visited: &mut [bool],
    out: &mut Vec<Rule>,
) {
    if visited[index] {
        return;
    }
    visited[index] = true;
    let mut dependencies = BTreeSet::new();
    for name in rules[index]
        .backtrack
        .iter()
        .flatten()
        .chain(rules[index].lookahead.iter().flatten())
    {
        if let Some(indices) = producers.get(name.as_str()) {
            dependencies.extend(indices.iter().copied());
        }
    }
    for dependency in dependencies {
        visit(dependency, rules, producers, visited, out);
    }
    out.push(rules[index].clone());
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
        // One context can carry several substitutions, which upstream applies
        // at once. The port has no multi-position rule, so each becomes its own
        // chaining rule. The context of the later one names what the earlier
        // one produced, which is what the buffer holds when it runs.
        let mut records: Vec<(usize, Vec<(GlyphId16, GlyphId16)>)> = Vec::new();
        for record in context.seq_lookup_records() {
            let position = usize::from(record.sequence_index());
            if segments.get(position).is_none() {
                continue;
            }
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
            records.push((position, pairs));
        }
        records.sort_by_key(|(position, _)| *position);

        for (position, pairs) in &records {
            let inputs = &segments[*position];
            let mut names: Vec<Vec<String>> = segments
                .iter()
                .map(|coverage| coverage.iter().map(|glyph| reader.name(*glyph)).collect())
                .collect();
            for (earlier, earlier_pairs) in &records {
                if earlier >= position {
                    break;
                }
                let targets: BTreeSet<String> = segments[*earlier]
                    .iter()
                    .filter_map(|source| {
                        earlier_pairs
                            .iter()
                            .find(|(from, _)| from == source)
                            .map(|(_, to)| reader.name(*to))
                    })
                    .collect();
                if !targets.is_empty() {
                    names[*earlier] = targets.into_iter().collect();
                }
            }
            // OpenType orders a backtrack array nearest first, so the positions
            // before the input are reversed.
            let mut before: Vec<Vec<String>> = names[..*position].iter().rev().cloned().collect();
            before.extend(
                backtrack
                    .iter()
                    .rev()
                    .map(|glyph| vec![reader.name(*glyph)]),
            );
            let mut after: Vec<Vec<String>> = names[position + 1..].to_vec();
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
    if parameters == Some("--fixture") {
        return run_fixture(font, out, std::env::args().nth(5).as_deref());
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

/// The rules the sample sequences ask for.
///
/// Shaping each sample with `calt` off and on gives, position by position, the
/// glyph the font starts from and the joined form it becomes. That is the rule
/// a shaper applied, and it does not depend on how the lookup was encoded.
fn sample_rules(font_path: &str, parameters: &str) -> Result<Vec<Rule>, String> {
    let bytes =
        std::fs::read(font_path).map_err(|error| format!("reading {font_path}: {error}"))?;
    let blob: harfrust::font::Blob = bytes.clone().into();
    let font = harfrust::Font::new(blob, 0)
        .ok_or_else(|| format!("{font_path}: the shaper cannot read the font"))?;
    let shaper = harfrust::ShaperFont::new(&font);
    let reader = Reader {
        font: FontRef::new(&bytes).map_err(|error| format!("{font_path}: {error}"))?,
    };

    let mut rules = Vec::new();
    for text in samples(parameters) {
        let (plain, _) = shape(&shaper, &reader, &text, false);
        let (joined, _) = shape(&shaper, &reader, &text, true);
        if plain.len() != joined.len() {
            continue;
        }
        for (index, (before, after)) in plain.iter().zip(&joined).enumerate() {
            if before == after {
                continue;
            }
            // The port writes one chaining rule per changed position. A rule
            // runs after the ones to its left, so their context names are the
            // joined forms, not the plain glyphs the reference matched.
            rules.push(Rule {
                backtrack: plain[..index]
                    .iter()
                    .zip(&joined[..index])
                    .rev()
                    .map(|(_, joined)| vec![joined.clone()])
                    .collect(),
                input: before.clone(),
                lookahead: plain[index + 1..]
                    .iter()
                    .map(|name| vec![name.clone()])
                    .collect(),
                target: after.clone(),
            });
        }
    }
    // A longer context is the more specific rule, so it is tried first.
    rules.sort_by(|a, b| {
        let length = |rule: &Rule| rule.backtrack.len() + rule.lookahead.len();
        length(b).cmp(&length(a)).then_with(|| a.cmp(b))
    });
    rules.dedup();

    Ok(order_by_dependency(&rules))
}

/// Writes the fixture, from the samples when a ligation set is given and from
/// the `calt` lookups otherwise.
///
/// # Errors
///
/// Returns a message when the font cannot be read or the file cannot be written.
pub fn run_fixture(font_path: &str, out: &str, parameters: Option<&str>) -> Result<(), String> {
    let rules = match parameters {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .map_err(|error| format!("reading {path}: {error}"))?;
            sample_rules(font_path, &text)?
        }
        None => read_rules(font_path)?,
    };

    write_fixture(font_path, out, &rules)
}

/// A pen that records a glyph outline as the corner and control points the
/// build's contours use.
#[derive(Default)]
struct QuadPen {
    contours: Vec<Vec<lemmata_geom::QuadifyPoint>>,
    current: Vec<lemmata_geom::QuadifyPoint>,
}

impl QuadPen {
    fn flush(&mut self) {
        if !self.current.is_empty() {
            self.contours.push(std::mem::take(&mut self.current));
        }
    }
}

impl skrifa::outline::OutlinePen for QuadPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.flush();
        self.current.push(lemmata_geom::QuadifyPoint {
            kind: lemmata_geom::QuadifyKind::Corner,
            x: f64::from(x),
            y: f64::from(y),
        });
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.current.push(lemmata_geom::QuadifyPoint {
            kind: lemmata_geom::QuadifyKind::Corner,
            x: f64::from(x),
            y: f64::from(y),
        });
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.current.push(lemmata_geom::QuadifyPoint {
            kind: lemmata_geom::QuadifyKind::Quadratic,
            x: f64::from(cx),
            y: f64::from(cy),
        });
        self.current.push(lemmata_geom::QuadifyPoint {
            kind: lemmata_geom::QuadifyKind::Corner,
            x: f64::from(x),
            y: f64::from(y),
        });
    }

    fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, x: f32, y: f32) {
        // A TrueType outline has no cubics. Ending the segment keeps a stray
        // one from losing the rest of the contour.
        self.line_to(x, y);
    }

    fn close(&mut self) {
        self.flush();
    }
}

/// Writes the rules and the joined glyphs as one fixture.
///
/// The fixture carries the outlines of every glyph the rules name, because the
/// font that ships to the page is built from a capture and those glyphs are not
/// letters it holds.
///
/// # Errors
///
/// Returns a message when the font cannot be read or the file cannot be written.
fn write_fixture(font_path: &str, out: &str, rules: &[Rule]) -> Result<(), String> {
    let bytes =
        std::fs::read(font_path).map_err(|error| format!("reading {font_path}: {error}"))?;
    let font =
        skrifa::prelude::FontRef::new(&bytes).map_err(|error| format!("{font_path}: {error}"))?;
    let read = read_fonts::FontRef::new(&bytes).map_err(|error| format!("{font_path}: {error}"))?;
    let post = read
        .post()
        .map_err(|error| format!("{font_path}: {error}"))?;
    let count = read
        .maxp()
        .map_err(|error| format!("{font_path}: {error}"))?
        .num_glyphs();

    // The rules name glyphs, so the names have to be turned back into ids to
    // draw them.
    let mut ids = std::collections::BTreeMap::new();
    for id in 0..count {
        let id = GlyphId16::new(id);
        if let Some(name) = post.glyph_name(id) {
            ids.insert(name.to_string(), skrifa::GlyphId::new(id.to_u32()));
        }
    }

    let metrics = font.glyph_metrics(
        skrifa::prelude::Size::unscaled(),
        skrifa::prelude::LocationRef::default(),
    );
    let outlines = font.outline_glyphs();
    let mut glyphs = serde_json::Map::new();
    let mut missing = Vec::new();
    let mut names: BTreeSet<String> = BTreeSet::new();
    for rule in rules {
        for position in &rule.backtrack {
            names.extend(position.iter().cloned());
        }
        names.insert(rule.input.clone());
        for position in &rule.lookahead {
            names.extend(position.iter().cloned());
        }
        names.insert(rule.target.clone());
    }
    for name in &names {
        let Some(id) = ids.get(name) else {
            missing.push(name.clone());
            continue;
        };
        let Some(glyph) = outlines.get(*id) else {
            missing.push(name.clone());
            continue;
        };
        let mut pen = QuadPen::default();
        glyph
            .draw(
                skrifa::outline::DrawSettings::unhinted(
                    skrifa::prelude::Size::unscaled(),
                    skrifa::prelude::LocationRef::default(),
                ),
                &mut pen,
            )
            .map_err(|error| format!("{name}: {error}"))?;
        pen.flush();
        let advance = metrics.advance_width(*id).map_or(0.0, f64::from);
        glyphs.insert(
            name.clone(),
            serde_json::json!({
                "advance": advance,
                "contours": pen.contours.iter().map(|contour| {
                    contour.iter().map(|point| serde_json::json!({
                        "x": point.x,
                        "y": point.y,
                        "quadratic": matches!(point.kind, lemmata_geom::QuadifyKind::Quadratic),
                    })).collect::<Vec<_>>()
                }).collect::<Vec<_>>(),
            }),
        );
    }
    if !missing.is_empty() {
        return Err(format!("the font has no outline for {missing:?}"));
    }
    let json = serde_json::json!({
        "rules": rules.iter().map(|rule| serde_json::json!({
            "backtrack": rule.backtrack,
            "input": rule.input,
            "lookahead": rule.lookahead,
            "target": rule.target,
        })).collect::<Vec<_>>(),
        "glyphs": glyphs,
    });
    std::fs::write(out, serde_json::to_string(&json).unwrap())
        .map_err(|error| format!("writing {out}: {error}"))?;
    eprintln!(
        "ligations: {} rules, {} glyphs, {} bytes",
        rules.len(),
        names.len(),
        std::fs::metadata(out).map_or(0, |meta| meta.len())
    );

    Ok(())
}
