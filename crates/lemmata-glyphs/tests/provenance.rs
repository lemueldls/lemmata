//! The provenance suite: `r[port.provenance]`.
//!
//! Every ported glyph names the upstream file it came from, and every record
//! points at a file that exists on both sides. A record is the only thing that
//! says a file is a port rather than ours, so a ported glyph without one is a
//! file whose licence and lineage are unstated.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The `ours` and `upstream` paths in the record, in the order they appear.
fn entries(record: &str) -> Vec<(String, String)> {
    let values: Vec<String> = record
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if !line.starts_with("ours = ") && !line.starts_with("upstream = ") {
                return None;
            }
            line.split('"').nth(1).map(str::to_string)
        })
        .collect();

    values
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| (pair[0].clone(), pair[1].clone()))
        .collect()
}

/// The `.rs` files under a crate that hold a port rather than module wiring.
fn ported_sources(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![directory.to_path_buf()];
    while let Some(next) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&next) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|extension| extension != "rs") {
                continue;
            }
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name == "lib.rs" || name == "mod.rs" {
                continue;
            }
            found.push(path);
        }
    }
    found.sort();

    found
}

/// `r[port.provenance]`: every ported glyph has a record, every record points at
/// files that exist, and the commit is the one the rest of the repository pins.
#[test]
fn ported_glyphs_have_provenance() {
    let root = root();
    let record =
        std::fs::read_to_string(root.join("PROVENANCE.toml")).expect("read the provenance record");

    let recorded: Vec<(String, String)> = entries(&record);
    assert!(!recorded.is_empty(), "the record names nothing");

    for (ours, upstream) in &recorded {
        assert!(
            root.join(ours).exists(),
            "the record names {ours}, which is not there"
        );
        assert!(
            root.join("vendor/Iosevka").join(upstream).exists(),
            "the record names {upstream}, which upstream does not have"
        );
    }

    // A ported glyph definition with no entry is the failure this suite is for.
    let missing: Vec<String> = ported_sources(&root.join("crates/lemmata-glyphs/src"))
        .iter()
        .map(|path| {
            path.strip_prefix(&root)
                .unwrap_or(path)
                .to_string_lossy()
                .into_owned()
        })
        .filter(|ours| !recorded.iter().any(|(recorded, _)| recorded == ours))
        .collect();
    assert!(
        missing.is_empty(),
        "these ported glyphs have no provenance: {missing:?}"
    );

    // The commit is the one the tag stamp carries and the fingerprint was built
    // from, so the three cannot come to describe different builds.
    let pinned = record
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("commit = ")
                .and_then(|value| value.split('"').nth(1))
        })
        .expect("the record names a commit");
    let golden = std::fs::read_to_string(root.join("golden/parity.json"))
        .expect("read the parity fingerprint");
    let upstream = golden
        .split("\"upstream\": \"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .expect("the fingerprint records an upstream commit");
    assert_eq!(
        pinned, upstream,
        "the provenance record and the fingerprint name different commits"
    );

    eprintln!(
        "provenance: {} records, {} ported glyphs, commit {}",
        recorded.len(),
        ported_sources(&root.join("crates/lemmata-glyphs/src")).len(),
        &pinned[..8]
    );
}
