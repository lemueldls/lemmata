//! The family mode of `build`: four faces, their style bits, and the files
//! around them.
//!
//! r[verify family.names]
//! r[verify family.static]

use std::path::{Path, PathBuf};
use std::process::Command;

use read_fonts::tables::head::MacStyle;
use read_fonts::tables::os2::SelectionFlags;
use read_fonts::types::NameId;
use read_fonts::{FontRef, TableProvider};

fn workspace(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn capture() -> Option<PathBuf> {
    let path = workspace("web/public/glyphs.bin");
    path.exists().then_some(path)
}

fn scratch() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "lemmata-family-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).expect("create the scratch directory");
    path
}

fn name(font: &FontRef, id: NameId) -> String {
    let name = font.name().expect("name");
    let string_data = name.string_data();
    name.name_record()
        .iter()
        .find(|record| record.name_id() == id && record.platform_id() == 3)
        .and_then(|record| record.string(string_data).ok())
        .map(|value| value.to_string())
        .expect("name record")
}

fn family_document(capture: &Path) -> String {
    let capture = capture.display();
    format!(
        r#"
[family]
name = "Lemmata Test"
version = "Version 2.000"
vendor = "TEST"
out = "out"
formats = ["ttf", "woff2", "css"]
policy = "mixed"
codepoints = "U+0061"
capture = "{capture}"

[family.base]
capHeight = 735
xHeight = 520
ascender = 965
descender = -215
lineGap = 70
width = 500
spacing = "normal"
serifs = "sans"

[family.instances.regular]
capture = "{capture}"

[family.instances.bold]
weight = 700
capture = "{capture}"

[family.instances.italic]
slope = 12
capture = "{capture}"

[family.instances.bold-italic]
weight = 700
slope = 12
capture = "{capture}"
"#
    )
}

#[test]
fn four_faces_carry_their_style_and_their_names() {
    let Some(capture) = capture() else {
        eprintln!("skipping: no capture");
        return;
    };
    let dir = scratch();
    let config = dir.join("family.toml");
    std::fs::write(&config, family_document(&capture)).expect("write the family");
    let status = Command::new(env!("CARGO_BIN_EXE_lemmata"))
        .args([
            "build",
            "--family",
            config.to_str().expect("config path"),
            "--subset",
            "U+0061",
        ])
        .status()
        .expect("run the family build");
    assert!(status.success(), "the family build failed");

    let out = dir.join("out");
    for file in [
        "LemmataTest-Regular.ttf",
        "LemmataTest-Bold.ttf",
        "LemmataTest-Italic.ttf",
        "LemmataTest-BoldItalic.ttf",
        "LemmataTest-Regular.woff2",
        "lemmata.json",
        "lemmata.css",
    ] {
        assert!(out.join(file).exists(), "{file} is missing");
    }

    let regular = std::fs::read(out.join("LemmataTest-Regular.ttf")).expect("regular");
    let regular = FontRef::new(&regular).expect("parse regular");
    assert_eq!(name(&regular, NameId::FAMILY_NAME), "Lemmata Test");
    assert_eq!(name(&regular, NameId::SUBFAMILY_NAME), "Regular");
    assert_eq!(
        name(&regular, NameId::TYPOGRAPHIC_SUBFAMILY_NAME),
        "Regular"
    );
    assert_eq!(
        name(&regular, NameId::POSTSCRIPT_NAME),
        "LemmataTest-Regular"
    );
    assert_eq!(regular.os2().expect("OS/2").us_weight_class(), 400);
    assert!(
        regular
            .os2()
            .expect("OS/2")
            .fs_selection()
            .contains(SelectionFlags::REGULAR)
    );
    assert_eq!(
        regular.head().expect("head").mac_style() & MacStyle::BOLD,
        MacStyle::empty()
    );
    assert_eq!(regular.post().expect("post").is_fixed_pitch(), 1);

    let bold = std::fs::read(out.join("LemmataTest-Bold.ttf")).expect("bold");
    let bold = FontRef::new(&bold).expect("parse bold");
    assert_eq!(name(&bold, NameId::SUBFAMILY_NAME), "Bold");
    assert_eq!(name(&bold, NameId::TYPOGRAPHIC_SUBFAMILY_NAME), "Bold");
    assert_eq!(name(&bold, NameId::POSTSCRIPT_NAME), "LemmataTest-Bold");
    assert_eq!(bold.os2().expect("OS/2").us_weight_class(), 700);
    assert!(
        bold.os2()
            .expect("OS/2")
            .fs_selection()
            .contains(SelectionFlags::BOLD)
    );
    assert_eq!(
        bold.head().expect("head").mac_style() & MacStyle::BOLD,
        MacStyle::BOLD
    );

    let italic = std::fs::read(out.join("LemmataTest-Italic.ttf")).expect("italic");
    let italic = FontRef::new(&italic).expect("parse italic");
    assert_eq!(name(&italic, NameId::SUBFAMILY_NAME), "Italic");
    assert!(
        italic
            .os2()
            .expect("OS/2")
            .fs_selection()
            .contains(SelectionFlags::ITALIC)
    );
    assert_eq!(italic.post().expect("post").italic_angle().to_f64(), -12.0);
    assert_eq!(italic.hhea().expect("hhea").caret_slope_rise(), 1000);

    let bold_italic = std::fs::read(out.join("LemmataTest-BoldItalic.ttf")).expect("bold italic");
    let bold_italic = FontRef::new(&bold_italic).expect("parse bold italic");
    assert_eq!(name(&bold_italic, NameId::SUBFAMILY_NAME), "Bold Italic");
    assert_eq!(
        name(&bold_italic, NameId::TYPOGRAPHIC_SUBFAMILY_NAME),
        "Bold Italic"
    );
    assert_eq!(
        name(&bold_italic, NameId::POSTSCRIPT_NAME),
        "LemmataTest-BoldItalic"
    );
    let selection = bold_italic.os2().expect("OS/2").fs_selection();
    assert!(selection.contains(SelectionFlags::BOLD));
    assert!(selection.contains(SelectionFlags::ITALIC));
    assert!(!selection.contains(SelectionFlags::REGULAR));

    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(out.join("lemmata.json")).expect("manifest"))
            .expect("parse manifest");
    assert_eq!(
        manifest["instances"].as_array().expect("instances").len(),
        4
    );
    assert_eq!(
        manifest["instances"][0]["coverage"]["capture"],
        "file-unverified"
    );

    let css = std::fs::read_to_string(out.join("lemmata.css")).expect("css");
    assert!(css.contains("font-weight: 700"));
    assert!(css.contains("font-style: italic"));
    assert!(css.contains("LemmataTest-BoldItalic.woff2"));

    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_strict_family_refuses_unported_glyphs() {
    let Some(capture) = capture() else {
        eprintln!("skipping: no capture");
        return;
    };
    let dir = scratch();
    let config = dir.join("family.toml");
    let text = family_document(&capture).replace("policy = \"mixed\"", "policy = \"strict\"");
    std::fs::write(&config, text).expect("write the family");
    let output = Command::new(env!("CARGO_BIN_EXE_lemmata"))
        .args([
            "build",
            "--family",
            config.to_str().expect("config path"),
            "--subset",
            "U+0061",
        ])
        .output()
        .expect("run the family build");

    assert!(!output.status.success(), "a strict family should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("strict"), "{stderr}");
    std::fs::remove_dir_all(dir).ok();
}
