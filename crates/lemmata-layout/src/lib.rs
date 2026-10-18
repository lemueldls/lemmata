//! The substitution features a font carries.
//!
//! GSUB is a list of lookups, a list of features that point at them, and a list
//! of scripts that point at those. `write-fonts` has the tables but no builders
//! for the three lists, so this assembles them from a description small enough
//! to read.
//!
//! Every feature here is a single substitution, which is what a form that
//! differs per glyph needs: one coverage naming the glyphs that change, and one
//! replacement per covered glyph, in coverage order.

use write_fonts::tables::gsub::{Gsub, SingleSubst, SubstitutionLookup};
use write_fonts::tables::layout::Feature as FeatureTable;
use write_fonts::tables::layout::{
    CoverageTable, FeatureList, FeatureRecord, LangSys, Lookup, LookupFlag, LookupList, Script,
    ScriptList, ScriptRecord,
};
use write_fonts::types::{GlyphId16, Tag};

/// A language system with no required feature. The specification uses `0xFFFF`.
const NO_REQUIRED_FEATURE: u16 = 0xFFFF;

/// One feature: a tag, the script it is registered under, and the glyphs it
/// substitutes, in coverage order.
#[derive(Clone, Debug)]
pub struct Feature {
    pub tag: Tag,
    pub script: Tag,
    pub substitutions: Vec<(GlyphId16, GlyphId16)>,
}

/// Builds a GSUB with one lookup per feature.
///
/// A feature with no substitutions is skipped: a lookup with an empty coverage
/// is valid but names nothing, and a feature that changes nothing is worse than
/// an absent one because it hides the gap.
#[must_use]
pub fn gsub(features: &[Feature]) -> Gsub {
    let mut lookups = Vec::new();
    let mut records = Vec::new();
    for feature in features {
        if feature.substitutions.is_empty() {
            continue;
        }
        let index = u16::try_from(lookups.len()).unwrap_or(u16::MAX);
        let (covered, replacements): (Vec<GlyphId16>, Vec<GlyphId16>) =
            feature.substitutions.iter().copied().unzip();
        lookups.push(SubstitutionLookup::Single(Lookup::new(
            LookupFlag::empty(),
            vec![SingleSubst::format_2(
                covered.into_iter().collect::<CoverageTable>(),
                replacements,
            )],
        )));
        records.push((feature.tag, index));
    }
    if records.is_empty() {
        return Gsub::default();
    }

    // One feature record per tag, each pointing at its own lookup.
    let feature_list = FeatureList {
        feature_records: records
            .iter()
            .map(|(tag, lookup)| FeatureRecord {
                feature_tag: *tag,
                feature: FeatureTable {
                    feature_params: None.into(),
                    lookup_list_indices: vec![*lookup],
                }
                .into(),
            })
            .collect(),
    };

    // A script points at every feature registered under it. Two features under
    // one script are two entries in the same language system, which is what
    // makes them both apply.
    let mut scripts: Vec<(Tag, Vec<u16>)> = Vec::new();
    for (index, (tag, _)) in records.iter().enumerate() {
        let script = features
            .iter()
            .find(|feature| feature.tag == *tag)
            .map_or(Tag::new(b"DFLT"), |feature| feature.script);
        let Ok(index) = u16::try_from(index) else {
            continue;
        };
        match scripts.iter_mut().find(|(recorded, _)| *recorded == script) {
            Some((_, indices)) => indices.push(index),
            None => scripts.push((script, vec![index])),
        }
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
