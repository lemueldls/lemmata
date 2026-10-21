//! Host capture keys, sidecars and the shape they describe.
//!
//! r[verify family.capture]

use lemmata_host::{CaptureShape, capture_key, plan_name, shape_matches};
use lemmata_ptl::param::Shape;

#[test]
fn a_shape_name_keeps_the_angle_serifs_and_spacing() {
    let upright = Shape::default();
    let italic = Shape {
        slope_angle: 12.5,
        serifs: Some("slab".to_string()),
        spacing: Some("quasi-proportional".to_string()),
        ..Shape::default()
    };
    let name = plan_name(&italic);

    assert!(name.contains("slope12_5"), "{name}");
    assert!(name.contains("slab"), "{name}");
    assert!(name.contains("quasi-proportional"), "{name}");
    assert_ne!(plan_name(&upright), name);
}

#[test]
fn the_capture_key_changes_with_the_codepoint_set() {
    let shape = Shape::default();
    assert_ne!(
        capture_key(&shape, &[0x61]),
        capture_key(&shape, &[0x61, 0x62])
    );
    assert_eq!(
        capture_key(&shape, &[0x62, 0x61]),
        capture_key(&shape, &[0x61, 0x62])
    );
}

#[test]
fn a_sidecar_has_to_match_the_shape_and_the_set() {
    let shape = Shape {
        weight: 700.0,
        ..Shape::default()
    };
    let recorded = CaptureShape {
        weight: 700.0,
        width: 500.0,
        slope_angle: 0.0,
        serifs: None,
        spacing: None,
        codepoints: vec![0x62, 0x61],
    };
    assert!(shape_matches(&recorded, &shape, &[0x61, 0x62]));
    assert!(!shape_matches(&recorded, &shape, &[0x61]));
    assert!(!shape_matches(
        &recorded,
        &Shape {
            weight: 400.0,
            ..shape
        },
        &[0x61, 0x62]
    ));
}
