//! How much of Iosevka's glyph source has been ported.
//!
//! `lemmata-glyphs` mirrors `vendor/Iosevka/packages/font-glyphs/src`, one
//! module per `.ptl` file, so the two trees can be compared directly. This
//! walks the upstream tree, maps each file to the module it should have become,
//! and reports what exists.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

/// Where the ported modules live.
const PORTED: &str = "crates/lemmata-glyphs/src";
/// Where the source they mirror lives.
const UPSTREAM: &str = "vendor/Iosevka/packages/font-glyphs/src";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The module a `.ptl` file should have become.
///
/// Upstream names files with hyphens, Rust with underscores, and the extension
/// changes. Everything else is the same path, so a module can be found from its
/// source without a table.
fn module_path(ptl: &Path) -> PathBuf {
    let mut path = ptl.to_path_buf();
    let name = path
        .file_name()
        .expect("file name")
        .to_string_lossy()
        .replace('-', "_")
        .replace(".ptl", ".rs");
    path.set_file_name(name);

    path
}

fn line_count(path: &Path) -> usize {
    fs::read_to_string(path)
        .map(|text| text.lines().count())
        .unwrap_or(0)
}

/// Walks the upstream tree and collects, per top-level directory, the files
/// that exist and their sizes.
fn collect(
    workspace: &Path,
    relative: &Path,
    out: &mut BTreeMap<String, (usize, usize, usize, usize)>,
) {
    let upstream = workspace.join(UPSTREAM);
    let Ok(entries) = fs::read_dir(upstream.join(relative)) else {
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(fs::DirEntry::file_name);

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let child = relative.join(&name);
        if path.is_dir() {
            collect(workspace, &child, out);
            continue;
        }
        if path.extension().and_then(std::ffi::OsStr::to_str) != Some("ptl") {
            continue;
        }
        let top = relative.components().next().map_or_else(
            || ".".to_string(),
            |part| part.as_os_str().to_string_lossy().to_string(),
        );
        let slot = out.entry(top).or_default();
        slot.1 += 1;
        slot.3 += line_count(&path);
        let module = workspace.join(PORTED).join(module_path(&child));

        if module.exists() {
            slot.0 += 1;
            slot.2 += line_count(&module);
        }
    }
}

/// Prints the report. Returns whether every upstream file has a module.
pub fn run() -> bool {
    let root = workspace_root();
    let mut by_directory = BTreeMap::new();
    collect(&root, Path::new(""), &mut by_directory);

    let (mut ported, mut files, mut ported_lines, mut lines) = (0, 0, 0, 0);
    println!(
        "{:<16} {:>10}  {:>18}",
        "directory", "files", "lines ported/total"
    );
    for (directory, (p, f, pl, l)) in &by_directory {
        println!("{directory:<16} {p:>4}/{f:<5}  {pl:>8}/{l:<9}");
        ported += p;
        files += f;
        ported_lines += pl;
        lines += l;
    }
    println!();
    println!("ported {ported} of {files} files ({ported_lines} of {lines} lines)");

    ported == files
}
