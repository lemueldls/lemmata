//! Family expansion: the four faces, their release names, and the checks that
//! keep two instances from claiming one name.
//!
//! r[verify family.schema]

use std::path::Path;

use lemmata_plan::family_from_json;

fn document(instances: &str) -> lemmata_plan::FamilyDocument {
    family_from_json(&format!(
        r#"{{
          "family": {{
            "name": "Lemmata",
            "version": "Version 1.000",
            "vendor": "LEMM",
            "out": "dist",
            "formats": ["ttf", "css"],
            "policy": "mixed",
            "base": {{
              "capHeight": 735,
              "xHeight": 520,
              "ascender": 965,
              "descender": -215,
              "lineGap": 70,
              "width": 500,
              "spacing": "normal",
              "serifs": "sans"
            }},
            "instances": {{ {instances} }}
          }}
        }}"#
    ))
    .expect("parse the family")
}

#[test]
fn four_faces_get_ribbi_names() {
    let document = document(
        r#"
          "regular": {},
          "bold": { "weight": 700 },
          "italic": { "slope": 12 },
          "bold-italic": { "weight": 700, "slope": 12 }
        "#,
    );
    let resolved = document
        .family
        .resolve(Path::new("."))
        .expect("resolve the family");

    let names: Vec<(&str, &str, &str, &str)> = resolved
        .iter()
        .map(|instance| {
            (
                instance.id.as_str(),
                instance.legacy_family.as_str(),
                instance.legacy_subfamily.as_str(),
                instance.postscript_name.as_str(),
            )
        })
        .collect();
    assert_eq!(
        names,
        [
            ("bold", "Lemmata", "Bold", "Lemmata-Bold"),
            (
                "bold-italic",
                "Lemmata",
                "Bold Italic",
                "Lemmata-BoldItalic"
            ),
            ("italic", "Lemmata", "Italic", "Lemmata-Italic"),
            ("regular", "Lemmata", "Regular", "Lemmata-Regular"),
        ]
    );
    assert_eq!(resolved[0].plan.weight_class, 700);
    assert_eq!(resolved[0].typographic_subfamily, "Bold");
    assert_eq!(resolved[2].plan.italic_angle, 12.0);
    assert_eq!(resolved[2].typographic_subfamily, "Italic");
}

#[test]
fn a_width_moves_the_legacy_family_and_keeps_the_typographic_one() {
    let document = document(r#""expanded": { "width": 600 }"#);
    let resolved = document
        .family
        .resolve(Path::new("."))
        .expect("resolve the family");

    assert_eq!(resolved[0].legacy_family, "Lemmata Expanded");
    assert_eq!(resolved[0].legacy_subfamily, "Regular");
    assert_eq!(resolved[0].typographic_family, "Lemmata");
    assert_eq!(resolved[0].typographic_subfamily, "Expanded");
    assert_eq!(resolved[0].postscript_name, "Lemmata-Expanded");
    assert_eq!(resolved[0].plan.width_class, 7);
}

#[test]
fn a_non_ribbi_weight_moves_the_legacy_family() {
    let document = document(r#""semi": { "weight": 600 }"#);
    let resolved = document
        .family
        .resolve(Path::new("."))
        .expect("resolve the family");

    assert_eq!(resolved[0].legacy_family, "Lemmata SemiBold");
    assert_eq!(resolved[0].legacy_subfamily, "Regular");
    assert_eq!(resolved[0].typographic_subfamily, "SemiBold");
}

#[test]
fn an_instance_plan_overrides_the_base() {
    let document = document(r#""tall": { "plan": { "xHeight": 600 } }"#);
    let resolved = document
        .family
        .resolve(Path::new("."))
        .expect("resolve the family");

    assert_eq!(resolved[0].plan.x_height, 600);
    assert_eq!(resolved[0].plan.cap_height, 735, "the base is still there");
}

#[test]
fn an_instance_can_change_the_serifs_and_spacing() {
    let document = document(r#""slab": { "serifs": "slab", "spacing": "quasi-proportional" }"#);
    let resolved = document
        .family
        .resolve(Path::new("."))
        .expect("resolve the family");

    assert_eq!(resolved[0].plan.serifs, "slab");
    assert_eq!(
        resolved[0].plan.spacing,
        lemmata_plan::Spacing::QuasiProportional
    );
}

#[test]
fn two_instances_cannot_share_a_postscript_name() {
    let document = document(
        r#"
          "one": { "postscript": "Lemmata-Same" },
          "two": { "postscript": "Lemmata-Same" }
        "#,
    );
    let error = document
        .family
        .resolve(Path::new("."))
        .expect_err("the duplicate should fail");
    assert!(error.contains("already used"), "{error}");
}

#[test]
fn an_iosevka_name_is_refused() {
    let document = document(r#""regular": { "legacy_family": "Iosevka" }"#);
    let error = document
        .family
        .resolve(Path::new("."))
        .expect_err("the reserved name should fail");
    assert!(error.contains("reserved"), "{error}");
}

#[test]
fn a_capture_path_is_relative_to_the_document() {
    let document = document(r#""regular": { "capture": "captures/regular.bin" }"#);
    let resolved = document
        .family
        .resolve(Path::new("/tmp/lemmata-family"))
        .expect("resolve the family");
    assert_eq!(
        resolved[0].capture.as_deref(),
        Some(Path::new("/tmp/lemmata-family/captures/regular.bin"))
    );
}
