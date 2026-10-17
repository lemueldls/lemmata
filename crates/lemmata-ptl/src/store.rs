//! The glyph store and the sink that fills it.
//!
//! Ported from `packages/glyph/src/store.mjs` and `GlyphSaveSink` in
//! `block.mjs`. A definition saves glyphs through the sink: the sink names them,
//! derives a `uniXXXX` name from a bare code point, refuses duplicates, sets the
//! advance width and the global transform from the metrics, and files the result
//! in the store.
//!
//! The store keeps insertion order. Upstream keys its maps by object identity,
//! which Rust does not have, so entries are keyed by index into the glyph list.
//!
//! Not ported: the dependency manager and the recursive build filter's block
//! half. Those drive subsetted rebuilds, which the outlines do not need. The
//! glyph half of the filter is here, because the sink consults it.

use std::collections::{HashMap, HashSet};

use kurbo::Affine;

use crate::glyph::{Component, Glyph};

/// The glyphs a build produced, by name and by code point.
#[derive(Clone, Debug, Default)]
pub struct GlyphStore {
    glyphs: Vec<Glyph>,
    names: HashMap<String, usize>,
    encodings: HashMap<u32, usize>,
    codes_of: HashMap<usize, Vec<u32>>,
}

impl GlyphStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of named glyphs, which is what upstream's `size` counts.
    #[must_use]
    pub fn size(&self) -> usize {
        self.names.len()
    }

    /// Every glyph, named or not.
    #[must_use]
    pub const fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }

    #[must_use]
    // `Vec` deref is not const yet, so this cannot be a `const fn`.
    #[allow(clippy::missing_const_for_fn)]
    pub fn glyphs(&self) -> &[Glyph] {
        &self.glyphs
    }

    /// Named entries in insertion order.
    #[must_use]
    pub fn named_entries(&self) -> Vec<(String, usize)> {
        self.glyphs
            .iter()
            .enumerate()
            .filter_map(|(index, glyph)| glyph.identifier.clone().map(|name| (name, index)))
            .collect()
    }

    /// The code points of one glyph, in the order they were assigned.
    #[must_use]
    pub fn unicode_of(&self, index: usize) -> Option<&[u32]> {
        self.codes_of.get(&index).map(Vec::as_slice)
    }

    #[must_use]
    pub fn add_glyph(&mut self, name: &str, glyph: Glyph) -> usize {
        let index = self.glyphs.len();
        self.glyphs.push(glyph);
        self.names.insert(name.to_string(), index);
        index
    }

    /// Files a glyph that has no name.
    #[must_use]
    pub fn add_anonymous(&mut self, glyph: Glyph) -> usize {
        let index = self.glyphs.len();
        self.glyphs.push(glyph);
        index
    }

    #[must_use]
    pub fn query_by_name(&self, name: &str) -> Option<usize> {
        self.names.get(name).copied()
    }

    /// # Errors
    ///
    /// Returns an error when no glyph carries the name.
    pub fn query_by_name_ensured(&self, name: &str) -> Result<usize, String> {
        self.query_by_name(name)
            .ok_or_else(|| format!("Glyph {name} doesn't exist."))
    }

    #[must_use]
    pub fn query_name_of(&self, index: usize) -> Option<&str> {
        self.glyphs[index].identifier.as_deref()
    }

    pub fn encode_glyph(&mut self, code: u32, index: usize) {
        self.encodings.insert(code, index);
        let codes = self.codes_of.entry(index).or_default();
        if !codes.contains(&code) {
            codes.push(code);
        }
    }

    #[must_use]
    pub fn query_by_unicode(&self, code: u32) -> Option<usize> {
        self.encodings.get(&code).copied()
    }

    #[must_use]
    pub fn query_name_by_unicode(&self, code: u32) -> Option<&str> {
        self.query_by_unicode(code)
            .and_then(|index| self.query_name_of(index))
    }

    /// # Errors
    ///
    /// Returns an error when no glyph carries the code point.
    pub fn query_name_by_unicode_ensured(&self, code: u32) -> Result<&str, String> {
        self.query_name_by_unicode(code)
            .ok_or_else(|| format!("Glyph for Unicode {code} doesn't exist."))
    }

    /// The code points of the glyph, or none when it carries no encoding.
    #[must_use]
    pub fn query_unicode_of(&self, index: usize) -> Option<&[u32]> {
        self.codes_of.get(&index).map(Vec::as_slice)
    }

    #[must_use]
    pub fn query_unicode_array_of_name(&self, name: &str) -> Option<&[u32]> {
        self.query_by_name(name)
            .and_then(|index| self.query_unicode_of(index))
    }

    pub fn delete_unicode_assignments_of(&mut self, index: usize) {
        if let Some(codes) = self.codes_of.remove(&index) {
            for code in codes {
                self.encodings.remove(&code);
            }
        }
    }

    /// A store holding only the named glyphs, with their encodings.
    #[must_use]
    pub fn filter_by_name(&self, keep: &HashSet<String>) -> Self {
        let mut filtered = Self::new();
        for (index, glyph) in self.glyphs.iter().enumerate() {
            let Some(name) = &glyph.identifier else {
                continue;
            };
            if !keep.contains(name) {
                continue;
            }
            let new_index = filtered.add_glyph(name, glyph.clone());
            for code in self.codes_of.get(&index).into_iter().flatten() {
                filtered.encode_glyph(*code, new_index);
            }
        }
        filtered
    }
}

/// What a save names its glyph.
#[derive(Clone, Debug)]
pub enum GlyphTarget {
    /// A name, with a code point when it has one.
    Named { name: String, unicode: u32 },
    /// A bare code point, which the sink turns into a `uniXXXX` name.
    CodePoint(u32),
    /// A glyph that is built but not filed.
    Anonymous,
}

/// Saves glyphs into a [`GlyphStore`], the way a definition does.
pub struct GlyphSaveSink {
    pub width: f64,
    pub global_transform: Affine,
    pub store: GlyphStore,
    /// When set, only these names are saved unless the save forces it.
    needed: Option<HashSet<String>>,
}

impl GlyphSaveSink {
    #[must_use]
    pub fn new(width: f64, global_transform: Affine) -> Self {
        Self {
            width,
            global_transform,
            store: GlyphStore::new(),
            needed: None,
        }
    }

    /// Restricts saves to the given names, the way a subsetted build does.
    pub fn restrict_to(&mut self, names: HashSet<String>) {
        self.needed = Some(names);
    }

    #[must_use]
    pub fn glyph_is_needed(&self, name: &str) -> bool {
        self.needed
            .as_ref()
            .is_none_or(|names| names.contains(name))
    }

    /// `save`: skip a glyph the build filter does not want.
    ///
    /// # Errors
    ///
    /// Returns an error when the name is already taken.
    pub fn save(
        &mut self,
        target: &GlyphTarget,
        contents: &impl Component,
    ) -> Result<Option<usize>, String> {
        self.save_impl(false, target, contents)
    }

    /// `saveFalse`: save even a glyph the build filter does not want.
    ///
    /// # Errors
    ///
    /// Returns an error when the name is already taken.
    pub fn save_forced(
        &mut self,
        target: &GlyphTarget,
        contents: &impl Component,
    ) -> Result<Option<usize>, String> {
        self.save_impl(true, target, contents)
    }

    fn save_impl(
        &mut self,
        force: bool,
        target: &GlyphTarget,
        contents: &impl Component,
    ) -> Result<Option<usize>, String> {
        let (name, unicode) = match target {
            GlyphTarget::Named { name, unicode } => (Some(name.clone()), *unicode),
            GlyphTarget::CodePoint(code) => (Some(format!("uni{code:04X}")), *code),
            GlyphTarget::Anonymous => (None, 0),
        };

        if !force
            && name
                .as_ref()
                .is_some_and(|name| !self.glyph_is_needed(name))
        {
            return Ok(None);
        }

        let mut glyph = Glyph::new();
        glyph.identifier.clone_from(&name);
        glyph.set_width(self.width);
        glyph.gizmo = self.global_transform;
        glyph.include(contents, true, true);

        let index = match name {
            Some(name) => {
                if self.store.query_by_name(&name).is_some() {
                    return Err(format!("Duplicate glyph: {name}"));
                }
                let index = self.store.add_glyph(&name, glyph);
                if unicode != 0 {
                    self.store.encode_glyph(unicode, index);
                }
                index
            }
            None => self.store.add_anonymous(glyph),
        };
        Ok(Some(index))
    }
}
