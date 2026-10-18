//! The parity pair: the native build, against the fingerprint in
//! `golden/parity.json`.
//!
//! The other half of the pair is `harness/parity.mjs`, which runs the same
//! build in wasm and compares against the same file. `cargo xtask parity` runs
//! both. Failing here means a pair that was byte-identical is not any more,
//! which is what `r[wasm.parity-bytes]` asks to be told about.
//!
//! The profile does not change the output, checked before this was written, so
//! one fingerprint covers a debug test run and a release wasm build.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use sha2::{Digest, Sha256};

fn workspace(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn capture_path() -> PathBuf {
    std::env::var_os("LEMMATA_GLYPH_DUMP")
        .map_or_else(|| workspace("target/glyphs.bin"), PathBuf::from)
}

fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn the_native_build_matches_the_recorded_pair() {
    let Ok(golden) = fs::read_to_string(workspace("golden/parity.json")) else {
        panic!("golden/parity.json is missing");
    };
    let golden: Value = serde_json::from_str(&golden).expect("parse the golden");
    let Ok(capture) = fs::read(capture_path()) else {
        eprintln!("skipping: no capture");
        return;
    };

    // The inputs are half of the pair. A different capture means the comparison
    // would be against something else, so it is a failure rather than a pass.
    let recorded_capture = golden["capture"].as_str().expect("capture");
    assert_eq!(
        sha256(&capture),
        recorded_capture,
        "the capture is not the one the pair was recorded from"
    );

    let plan = golden["plan"].to_string();
    let bytes = lemmata_web::build(&capture, &plan).expect("build");
    let ours = sha256(&bytes);
    let recorded = golden["font"].as_str().expect("font");
    assert_eq!(
        ours, recorded,
        "the native build no longer matches the recorded pair.\n  \
         If the change is intended, regenerate with `cargo xtask regen` and a drift \
         report, as golden/README.md asks.\n  \
         recorded  {recorded}"
    );

    eprintln!(
        "parity: native {} matches the recorded pair (rustc {}), wasm compares against the same file",
        &ours[..16],
        golden["toolchain"]["rustc"]
            .as_str()
            .unwrap_or("unrecorded")
    );
}
