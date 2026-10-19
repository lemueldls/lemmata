//! What the expander has to cover, counted across every definition.
//!
//! `patel-c --dump-ast <file>` writes the reader's S-expressions. Point
//! `LEMMATA_PTL_DIR` at a directory of those and this separates the heads that
//! are macros from the ones that are ordinary calls, because only the first kind
//! has to be expanded.

use std::collections::BTreeMap;
use std::path::Path;

/// The macros `meta/macros.ptl` defines, less the operator forms.
const MACROS: &[&str] = &[
    "Just",
    "lift-",
    "with-params",
    "flex-params",
    "glyph-proc",
    "composite-proc",
    "set-width",
    "include",
    "set-mark-anchor",
    "set-base-anchor",
    "eject-contour",
    "new-glyph",
    "create-glyph",
    "create-aliased-glyph",
    "create-forked-glyph",
    "glyph-is-needed",
    "glyph-module",
    "run-glyph-module",
    "glyph-block-import",
    "glyph-block-export",
    "for-width-kinds",
    "end-glyph-block",
    "glyph-block",
];

/// `PatEL`'s own forms, which the framework expands rather than the macros.
const BUILTINS: &[&str] = &[
    "begin",
    "define",
    "function",
    "local",
    "return",
    "match",
    "namespace",
    "do",
    "foreach",
    "if",
    "object",
    "export",
    "import",
];

#[test]
fn the_definitions_name_the_forms_to_expand() {
    let Some(directory) = std::env::var_os("LEMMATA_PTL_DIR") else {
        eprintln!("skipping: set LEMMATA_PTL_DIR to a directory of reader dumps");
        return;
    };
    let mut files = 0usize;
    let mut heads: BTreeMap<String, usize> = BTreeMap::new();
    for entry in std::fs::read_dir(Path::new(&directory)).expect("read the dump directory") {
        let path = entry.expect("a directory entry").path();
        if path.extension().is_none_or(|extension| extension != "json") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read a dump");
        let program = lemmata_ptl_front::parse(&text).expect("parse a dump");
        for (form, count) in lemmata_ptl_front::inventory(&program) {
            *heads.entry(form).or_insert(0) += count;
        }
        files += 1;
    }
    assert!(files > 0, "the directory holds no dumps");

    let used_macros: Vec<(&str, usize)> = MACROS
        .iter()
        .filter_map(|name| heads.get(*name).map(|count| (*name, *count)))
        .collect();
    let used_builtins: Vec<(&str, usize)> = BUILTINS
        .iter()
        .filter_map(|name| heads.get(*name).map(|count| (*name, *count)))
        .collect();
    assert!(!used_macros.is_empty(), "no macro is used at all");

    for (name, count) in &used_macros {
        eprintln!("  macro    {count:6} {name}");
    }
    for (name, count) in &used_builtins {
        eprintln!("  builtin  {count:6} {name}");
    }
    let missing: Vec<&&str> = MACROS
        .iter()
        .filter(|name| !heads.contains_key(**name))
        .collect();
    eprintln!(
        "{} files, {} distinct heads, {} macros used, {} unused: {missing:?}",
        files,
        heads.len(),
        used_macros.len(),
        missing.len()
    );
}
