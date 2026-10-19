//! Expands every reader dump, because a shape the expander does not handle is a
//! macro that would reach the translator unexpanded.
//!
//! Point `LEMMATA_PTL_DIR` at a directory of `patel-c --dump-ast` output. The
//! test reports every file that fails and every macro head that survives, so a
//! new construct names itself rather than hiding behind the first failure.

use std::path::Path;

use lemmata_ptl_front::{Sexpr, expand, parse};

/// A macro head left in an expanded program, if there is one.
fn residual(node: &Sexpr) -> Option<String> {
    let Sexpr::List(items) = node else {
        return None;
    };
    if let Some(head) = node.form() {
        // A macro definition holds its own patterns, not code to expand.
        if matches!(
            head,
            ".quote" | ".list" | "define-macro" | "define-operator"
        ) {
            return None;
        }
        if lemmata_ptl_front::MACROS.contains(&head) {
            return Some(head.to_string());
        }
    }
    items.iter().find_map(residual)
}

#[test]
fn every_definition_expands() {
    let Some(directory) = std::env::var_os("LEMMATA_PTL_DIR") else {
        eprintln!("skipping: set LEMMATA_PTL_DIR to a directory of reader dumps");
        return;
    };

    let mut files = 0usize;
    let mut failures = Vec::new();
    for entry in std::fs::read_dir(Path::new(&directory)).expect("read the dump directory") {
        let path = entry.expect("a directory entry").path();
        if path.extension().is_none_or(|extension| extension != "json") {
            continue;
        }
        let name = path
            .file_name()
            .expect("a file name")
            .to_string_lossy()
            .to_string();
        files += 1;

        let text = std::fs::read_to_string(&path).expect("read a dump");
        let program = parse(&text).expect("parse a dump");
        match expand(&program) {
            Ok(expanded) => {
                if let Some(head) = residual(&expanded) {
                    failures.push(format!("{name}: {head} survived expansion"));
                }
            }
            Err(error) => failures.push(format!("{name}: {error}")),
        }
    }
    assert!(files > 0, "the directory holds no dumps");
    let mut report = String::new();
    for failure in &failures {
        report.push_str("\n  ");
        report.push_str(failure);
    }
    assert!(
        failures.is_empty(),
        "{} of {files} dumps still need work:{report}",
        failures.len()
    );
}
