//! The substitution features a font carries.
//!
//! GSUB is a list of lookups, a list of features that point at them, and a list
//! of scripts that point at those. `write-fonts` has the tables but no builders
//! for the three lists, so this assembles them from a description small enough
//! to read.
//!
//! Two kinds of feature are expressible. A plain substitution covers a set of
//! glyphs and replaces each with its partner, which is what a form that differs
//! per glyph needs. A contextual one fires only where the neighbours allow,
//! which is what a form that borrows space from the glyph beside it needs.

use write_fonts::tables::gsub::{
    AlternateSet, AlternateSubstFormat1, Gsub, Ligature, LigatureSet, LigatureSubstFormat1,
    SingleSubst, SubstitutionChainContext, SubstitutionLookup,
};
use write_fonts::tables::layout::{
    ChainedSequenceContext, ChainedSequenceContextFormat3, CoverageTable, Feature as FeatureTable,
    FeatureList, FeatureRecord, LangSys, Lookup, LookupFlag, LookupList, Script, ScriptList,
    ScriptRecord, SequenceLookupRecord,
};
use write_fonts::types::{GlyphId16, Tag};

/// A language system with no required feature. The specification uses `0xFFFF`.
const NO_REQUIRED_FEATURE: u16 = 0xFFFF;

/// One contextual rule: when the input glyph is in `input`, with the glyph
/// before it in `backtrack` and the one after in `lookahead`, substitute the
/// input through `apply`.
///
/// An empty coverage matches anything, which is how a rule that only cares
/// about one side is written.
#[derive(Clone, Debug, Default)]
pub struct Rule {
    /// The glyphs before the input, nearest first. Each entry is one position
    /// and names every glyph that fits it.
    pub backtrack: Vec<Vec<GlyphId16>>,
    /// The glyphs the input position accepts.
    pub input: Vec<GlyphId16>,
    /// The glyphs after the input, nearest first, one entry per position.
    pub lookahead: Vec<Vec<GlyphId16>>,
    pub apply: Vec<(GlyphId16, GlyphId16)>,
}

/// What a feature does.
#[derive(Clone, Debug)]
pub enum Effect {
    /// One lookup substituting each of these glyphs for its partner.
    Single(Vec<(GlyphId16, GlyphId16)>),
    /// One lookup offering each of these glyphs a list of forms to choose from.
    ///
    /// This is a different lookup type from [`Effect::Single`], and the
    /// difference matters: the feature does not say which form is used, the
    /// renderer picks one by index. `ssty` is written this way, so the two
    /// optical sizes of a script glyph are alternatives rather than one
    /// substitution.
    Alternate(Vec<(GlyphId16, Vec<GlyphId16>)>),
    /// A sequence of rules, applied in order, so a later rule sees what an
    /// earlier one left behind.
    Contextual(Vec<Rule>),
    /// A ligature: when the components appear in order, replace them with the
    /// target.
    ///
    /// Each entry is the whole sequence and the glyph it becomes. The first
    /// component is the one the coverage names.
    Ligature(Vec<(Vec<GlyphId16>, GlyphId16)>),
}

/// One feature: a tag, the scripts it is registered under, and what it does.
#[derive(Clone, Debug)]
pub struct Feature {
    pub tag: Tag,
    pub scripts: Vec<Tag>,
    pub effect: Effect,
}

/// A coverage from a glyph list.
///
/// `write-fonts`' builder takes the glyphs in order and does not sort them, and
/// a coverage that is out of order reads as a coverage that does not match: the
/// lookup is well formed, `ots-sanitize` accepts it, and the shaper quietly
/// does nothing. Anything that arrives in another order is sorted here.
fn coverage(glyphs: Vec<GlyphId16>) -> CoverageTable {
    let mut glyphs = glyphs;
    glyphs.sort_unstable();
    glyphs.dedup();

    glyphs.into_iter().collect()
}

/// A single-substitution lookup.
fn substitution(pairs: &[(GlyphId16, GlyphId16)]) -> SubstitutionLookup {
    // Format 2 pairs the coverage with the replacements in order, so the pairs
    // are sorted before they are split. Sorting the coverage alone, which is
    // what the builder needs, would leave each glyph pointing at another
    // glyph's form.
    let mut pairs = pairs.to_vec();
    pairs.sort_unstable();
    let (covered, replacements): (Vec<GlyphId16>, Vec<GlyphId16>) = pairs.into_iter().unzip();

    SubstitutionLookup::Single(Lookup::new(
        LookupFlag::empty(),
        vec![SingleSubst::format_2(coverage(covered), replacements)],
    ))
}

/// An alternate-substitution lookup.
fn alternates(entries: &[(GlyphId16, Vec<GlyphId16>)]) -> SubstitutionLookup {
    // Format 1 orders the alternate sets by coverage index, so the entries are
    // sorted together. A glyph named twice would give the coverage a duplicate
    // that its own sort removes, and the sets would then be off by one from the
    // glyphs they belong to, so the first entry for a glyph wins.
    let mut entries = entries.to_vec();
    entries.sort_unstable_by_key(|(glyph, _)| *glyph);
    entries.dedup_by_key(|(glyph, _)| *glyph);
    let (covered, sets): (Vec<GlyphId16>, Vec<Vec<GlyphId16>>) = entries.into_iter().unzip();

    SubstitutionLookup::Alternate(Lookup::new(
        LookupFlag::empty(),
        vec![AlternateSubstFormat1::new(
            coverage(covered),
            sets.into_iter().map(AlternateSet::new).collect(),
        )],
    ))
}

/// A ligature-substitution lookup.
///
/// A ligature set holds every sequence that starts with the same glyph, and the
/// specification orders that set by preference, so the longest sequence comes
/// first: a shaper tries the entries in order and the shorter one would
/// otherwise win.
fn ligatures(entries: &[(Vec<GlyphId16>, GlyphId16)]) -> Option<SubstitutionLookup> {
    let mut entries: Vec<&(Vec<GlyphId16>, GlyphId16)> = entries
        .iter()
        .filter(|(components, _)| !components.is_empty())
        .collect();
    if entries.is_empty() {
        return None;
    }
    entries.sort_by(|(a, _), (b, _)| {
        b.len()
            .cmp(&a.len())
            .then_with(|| a.as_slice().cmp(b.as_slice()))
    });

    // Group by the covering glyph, preserving the preference order within a
    // group. The sets are emitted in coverage order, which is sorted.
    let mut groups: Vec<(GlyphId16, Vec<Ligature>)> = Vec::new();
    for (components, target) in entries {
        let first = components[0];
        let ligature = Ligature::new(*target, components[1..].to_vec());
        match groups.iter_mut().find(|(glyph, _)| *glyph == first) {
            Some((_, ligatures)) => ligatures.push(ligature),
            None => groups.push((first, vec![ligature])),
        }
    }
    groups.sort_by_key(|(glyph, _)| *glyph);
    let (covered, sets): (Vec<GlyphId16>, Vec<LigatureSet>) = groups
        .into_iter()
        .map(|(glyph, ligatures)| (glyph, LigatureSet::new(ligatures)))
        .unzip();

    Some(SubstitutionLookup::Ligature(Lookup::new(
        LookupFlag::empty(),
        vec![LigatureSubstFormat1::new(coverage(covered), sets)],
    )))
}

/// Builds a GSUB carrying `features`.
///
/// A feature with nothing to do is skipped: a lookup with an empty coverage is
/// valid but names nothing, and a feature that changes nothing is worse than an
/// absent one because it hides the gap.
#[must_use]
pub fn gsub(features: &[Feature]) -> Gsub {
    let mut lookups = Vec::new();
    let mut features_out: Vec<(Tag, Vec<Tag>, Vec<u16>)> = Vec::new();
    for feature in features {
        let mut indices = Vec::new();
        match &feature.effect {
            Effect::Single(pairs) => {
                if pairs.is_empty() {
                    continue;
                }
                indices.push(push(&mut lookups, substitution(pairs)));
            }
            Effect::Alternate(entries) => {
                if entries.is_empty() {
                    continue;
                }
                indices.push(push(&mut lookups, alternates(entries)));
            }
            Effect::Ligature(entries) => {
                if let Some(lookup) = ligatures(entries) {
                    indices.push(push(&mut lookups, lookup));
                } else {
                    continue;
                }
            }
            Effect::Contextual(rules) => {
                for rule in rules {
                    if rule.input.is_empty() || rule.apply.is_empty() {
                        continue;
                    }
                    // The substitution is reached through the rule, so it goes
                    // in the list but not in the feature's own indices: a
                    // lookup a feature names directly applies everywhere.
                    let applied = push(&mut lookups, substitution(&rule.apply));
                    // One coverage per position, each naming every glyph that
                    // fits it. A coverage per glyph instead says the sequence
                    // is as long as the set is wide, which is a rule that can
                    // never match: the table is well formed and the shaper
                    // finds nothing to apply.
                    let context = ChainedSequenceContextFormat3 {
                        backtrack_coverages: rule
                            .backtrack
                            .iter()
                            .map(|position| coverage(position.clone()).into())
                            .collect(),
                        input_coverages: vec![coverage(rule.input.clone()).into()],
                        lookahead_coverages: rule
                            .lookahead
                            .iter()
                            .map(|position| coverage(position.clone()).into())
                            .collect(),
                        seq_lookup_records: vec![SequenceLookupRecord::new(0, applied)],
                    };
                    // The newtype has no constructor, so it is filled through
                    // its `DerefMut`.
                    let mut wrapped = SubstitutionChainContext::default();
                    *wrapped = ChainedSequenceContext::Format3(context);
                    indices.push(push(
                        &mut lookups,
                        SubstitutionLookup::ChainContextual(Lookup::new(
                            LookupFlag::empty(),
                            vec![wrapped],
                        )),
                    ));
                }
            }
        }
        if !indices.is_empty() {
            features_out.push((feature.tag, feature.scripts.clone(), indices));
        }
    }
    if features_out.is_empty() {
        return Gsub::default();
    }

    // The specification requires both lists in tag order, and a shaper binary
    // searches them, so an unsorted list is a list it cannot find anything in.
    // This is invisible to a read-back and to a sanitizer, and it is what kept
    // the features out of the shaper's hands.
    let mut order: Vec<usize> = (0..features_out.len()).collect();
    order.sort_by_key(|index| features_out[*index].0);
    let mut remapped = vec![0u16; features_out.len()];
    for (position, index) in order.iter().enumerate() {
        remapped[*index] = u16::try_from(position).unwrap_or(u16::MAX);
    }

    let feature_list = FeatureList {
        feature_records: order
            .iter()
            .map(|index| {
                let (tag, _, indices) = &features_out[*index];
                FeatureRecord {
                    feature_tag: *tag,
                    feature: FeatureTable {
                        feature_params: None.into(),
                        lookup_list_indices: indices.clone(),
                    }
                    .into(),
                }
            })
            .collect(),
    };

    // A script points at every feature registered under it, and a feature may
    // be registered under more than one, which is how a contextual feature
    // reaches ordinary text as well as whatever else names it.
    let mut scripts: Vec<(Tag, Vec<u16>)> = Vec::new();
    for (index, (_, tags, _)) in features_out.iter().enumerate() {
        let feature = remapped[index];
        for tag in tags {
            match scripts.iter_mut().find(|(recorded, _)| recorded == tag) {
                Some((_, indices)) => indices.push(feature),
                None => scripts.push((*tag, vec![feature])),
            }
        }
    }
    scripts.sort_by_key(|(tag, _)| *tag);
    for (_, indices) in &mut scripts {
        indices.sort_unstable();
        indices.dedup();
    }

    Gsub {
        script_list: ScriptList {
            script_records: scripts
                .iter()
                .map(|(tag, indices)| ScriptRecord {
                    script_tag: *tag,
                    script: Script {
                        default_lang_sys: Some(LangSys {
                            required_feature_index: NO_REQUIRED_FEATURE,
                            feature_indices: indices.clone(),
                        })
                        .into(),
                        lang_sys_records: Vec::new(),
                    }
                    .into(),
                })
                .collect(),
        }
        .into(),
        feature_list: feature_list.into(),
        lookup_list: LookupList {
            lookups: lookups.into_iter().map(Into::into).collect(),
        }
        .into(),
        ..Gsub::default()
    }
}

/// Appends a lookup and answers its index.
fn push(lookups: &mut Vec<SubstitutionLookup>, lookup: SubstitutionLookup) -> u16 {
    let index = u16::try_from(lookups.len()).unwrap_or(u16::MAX);
    lookups.push(lookup);

    index
}
