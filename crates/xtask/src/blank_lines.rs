//! Puts a blank line before a block's tail expression.
//!
//! The style the repository uses: the expression a block returns is separated
//! from the work above it, so the reader can see where the value comes from.
//!
//! `cargo fmt` cannot express this, and a rewrite template cannot either, since
//! inserting the line needs the block's own indentation. So the rule in
//! `rules/blank-line-before-tail-expression.yml` finds the blocks and their
//! bodies, and this reads the ranges out of the JSON and edits the files.
//!
//! Two guards keep the result readable. A body holding two items or fewer is
//! left alone, because a single expression has nothing above it to be separated
//! from, and that covers most of the small helpers. And a body whose last item
//! is a statement rather than an expression is left alone, since a statement is
//! not what the block returns.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// The finder.
const RULE: &str = "rules/blank-line-before-tail-expression.yml";

/// A body holding this many items or fewer is left alone.
const MAX_BODY_ITEMS: usize = 2;

/// A body's last item can be an item rather than an expression. Statements end
/// with a semicolon, which is checked separately; these are the ones that end
/// with a brace.
const ITEM_PREFIXES: &[&str] = &[
    "fn ",
    "pub ",
    "unsafe fn ",
    "const fn ",
    "async fn ",
    "struct ",
    "enum ",
    "union ",
    "impl ",
    "trait ",
    "mod ",
    "use ",
    "const ",
    "static ",
    "type ",
    "extern ",
    "macro_rules!",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every Rust file in the workspace, sorted so a run is reproducible.
fn sources(root: &Path) -> Vec<PathBuf> {
    fn walk(directory: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(directory) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().and_then(std::ffi::OsStr::to_str) == Some("rs") {
                out.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk(&root.join("crates"), &mut found);
    found.sort();

    found
}

/// The line each file needs a blank line inserted before.
fn edits(root: &Path) -> Result<BTreeMap<PathBuf, Vec<usize>>, String> {
    let sources = sources(root);
    let output = Command::new("ast-grep")
        .args(["scan", "-r", RULE, "--json=stream"])
        .args(&sources)
        .current_dir(root)
        .output()
        .map_err(|error| format!("running ast-grep: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "ast-grep failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let mut by_file: BTreeMap<PathBuf, Vec<usize>> = BTreeMap::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if !line.trim_start().starts_with('{') {
            continue;
        }
        let match_: Value = serde_json::from_str(line)
            .map_err(|error| format!("parsing ast-grep output: {error}"))?;
        let Some(body) = match_["metaVariables"]["multi"]["BODY"].as_array() else {
            continue;
        };
        if body.len() <= MAX_BODY_ITEMS {
            continue;
        }
        let tail = body.last().expect("body is non-empty");
        let text = tail["text"].as_str().unwrap_or_default().trim();
        if text.ends_with(';') || ITEM_PREFIXES.iter().any(|prefix| text.starts_with(prefix)) {
            continue;
        }
        let Some(file) = match_["file"].as_str() else {
            continue;
        };
        let line = usize::try_from(tail["range"]["start"]["line"].as_u64().unwrap_or_default())
            .unwrap_or_default();
        by_file.entry(root.join(file)).or_default().push(line);
    }

    Ok(by_file)
}

/// Reports what would change, or applies it. Returns the number of lines added.
///
/// # Errors
///
/// Returns a message when ast-grep cannot be run or its output cannot be read.
pub fn run(check: bool) -> Result<usize, String> {
    let root = workspace_root();
    let mut added = 0;
    for (path, mut lines) in edits(&root)? {
        lines.sort_unstable();
        lines.dedup();
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("reading {}: {error}", path.display()))?;
        let mut split: Vec<String> = source.split_inclusive('\n').map(str::to_string).collect();
        let mut file_added = 0;
        for line in lines.iter().rev() {
            // Already separated, or the expression opens the block.
            if *line == 0 || split[line - 1].trim().is_empty() {
                continue;
            }
            split.insert(*line, "\n".to_string());
            file_added += 1;
        }
        if file_added == 0 {
            continue;
        }
        added += file_added;
        let relative = path.strip_prefix(&root).unwrap_or(&path);
        println!("{:<56} {file_added:>3}", relative.display());

        if !check {
            fs::write(&path, split.concat())
                .map_err(|error| format!("writing {}: {error}", path.display()))?;
        }
    }

    Ok(added)
}
