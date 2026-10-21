//! The coverage report: how much of a plan the port draws, and how much comes
//! from the capture.
//!
//! r[verify family.coverage]

use std::path::Path;

fn dump_path() -> Option<std::path::PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/public/glyphs.bin");
    path.exists().then_some(path)
}

#[test]
fn the_report_counts_every_captured_glyph() {
    let Some(path) = dump_path() else {
        eprintln!("skipping: no capture");
        return;
    };
    let data = std::fs::read(path).expect("read the capture");
    let captured = lemmata_capture::load(&data).expect("decode the capture");
    let plan = lemmata_plan::Plan::default();
    let (glyphs, coverage) = lemmata_source::glyphs_reported(&plan, &captured);

    assert_eq!(
        coverage.ported + coverage.captured + coverage.missing,
        captured.len(),
        "the report has to account for every glyph in the capture"
    );
    assert!(coverage.ported > 0, "the port draws nothing");
    assert!(coverage.captured > 0, "the capture supplies nothing");
    assert_eq!(glyphs.len(), captured.len());
}
