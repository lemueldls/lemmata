//! The parity pair: regenerating it, and running both halves.
//!
//! `golden/parity.json` records the capture, the plan, the toolchain and the
//! font, so the native build and the wasm build can both be compared against
//! one fingerprint. `r[wasm.parity-bytes]` asks for exactly this: pairs tracked
//! per pinned toolchain, failing when one that was byte-identical stops being
//! so.
//!
//! Both halves shell out rather than link, so this tool needs neither the
//! geometry nor the font writer and cannot disagree with the CLI about what it
//! built.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn capture_path() -> PathBuf {
    std::env::var_os("LEMMATA_GLYPH_DUMP")
        .map_or_else(|| root().join("target/glyphs.bin"), PathBuf::from)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("reading {}: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn run(command: &mut Command) -> Result<(), String> {
    let shown = format!("{command:?}");
    let status = command
        .status()
        .map_err(|error| format!("running {shown}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{shown} failed"))
    }
}

fn capture_line(command: &mut Command) -> Result<String, String> {
    let output = command
        .output()
        .map_err(|error| format!("running {command:?}: {error}"))?;
    if !output.status.success() {
        return Err(format!("{command:?} failed"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// The `wasm-bindgen` version the workspace pins, so the fingerprint names the
/// generator as well as the compiler.
fn wasm_bindgen_pin() -> Result<String, String> {
    let manifest = fs::read_to_string(root().join("Cargo.toml"))
        .map_err(|error| format!("reading Cargo.toml: {error}"))?;
    manifest
        .lines()
        .find_map(|line| line.strip_prefix("wasm-bindgen = "))
        .map(|value| {
            value
                .trim()
                .trim_matches('"')
                .trim_start_matches('=')
                .to_string()
        })
        .ok_or_else(|| "no wasm-bindgen pin in [workspace.dependencies]".to_string())
}

/// Rebuilds the pair. Update the file only with a drift report explaining the
/// change, as `golden/README.md` asks.
pub fn regen() -> Result<(), String> {
    let built = root().join("target/parity-regen.ttf");
    run(Command::new("cargo")
        .current_dir(root())
        .args([
            "run",
            "-q",
            "--release",
            "-p",
            "lemmata-cli",
            "--offline",
            "--",
            "build",
            "--out",
        ])
        .arg(&built))?;

    // The upstream commit is not this command's to choose, so it is carried
    // over from the file being replaced.
    let path = root().join("golden/parity.json");
    let upstream = fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|golden| golden["upstream"].as_str().map(str::to_string))
        .ok_or_else(|| "golden/parity.json has no upstream commit to carry over".to_string())?;

    let golden = json!({
        "oracle": "parity",
        "upstream": upstream,
        "capture": sha256_file(&capture_path())?,
        "plan": {},
        "toolchain": {
            "rustc": capture_line(Command::new("rustc").arg("--version"))?
                .trim_start_matches("rustc ")
                .to_string(),
            "wasm-bindgen": wasm_bindgen_pin()?,
        },
        "font": sha256_file(&built)?,
    });
    fs::write(
        &path,
        serde_json::to_string_pretty(&golden).expect("serialize") + "\n",
    )
    .map_err(|error| format!("writing {}: {error}", path.display()))?;
    println!("golden/parity.json regenerated");
    Ok(())
}

/// Runs both halves: the native one through its test, then the wasm one through
/// the node harness.
pub fn check() -> Result<(), String> {
    run(Command::new("cargo").current_dir(root()).args([
        "test",
        "-p",
        "lemmata-web",
        "--offline",
        "--test",
        "parity",
    ]))?;
    run(Command::new("wasm-pack").current_dir(root()).args([
        "build",
        "crates/lemmata-web",
        "--target",
        "nodejs",
        "--out-dir",
        "../../target/lemmata-node",
        "--offline",
    ]))?;
    run(Command::new("node")
        .current_dir(root())
        .arg("harness/parity.mjs"))
}
