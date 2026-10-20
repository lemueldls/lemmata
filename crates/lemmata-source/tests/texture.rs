//! Checks that the port draws the texture forms it covers.
//!
//! The build falls back to scaling the captured outline, so the drawn path has
//! to be visible on its own: a base letter whose variant the registry builds
//! must have all six forms, and one the port does not build must be absent
//! rather than silently scaled under a drawn name.

use lemmata_build::texture::{FORMS, Form};
use lemmata_capture::dump_path;
use lemmata_plan::Plan;

#[test]
fn the_port_draws_the_forms_it_covers() {
    let Ok(data) = std::fs::read(dump_path()) else {
        eprintln!("skipping: no capture");
        return;
    };
    let captured = lemmata_capture::load(&data).expect("decode the capture");
    let drawn = lemmata_source::texture_forms(&Plan::default(), &captured);

    // `l` and `m` are ported definitions, so their forms are re-drawn.
    for base in ["l", "m"] {
        for form in FORMS {
            assert!(
                drawn.contains_key(&(base.to_string(), *form)),
                "{base} has no drawn {}",
                form.suffix()
            );
        }
    }
    // `i` is a composed glyph (a dotless stem plus a dot), and that composition
    // is not ported yet, so its forms are still the scaled fallback.
    assert!(
        !drawn.contains_key(&("i".to_string(), Form::ExtL)),
        "i now has a drawn form, and the note above is stale"
    );
}
