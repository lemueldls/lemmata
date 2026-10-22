//! Checks that the port draws the texture forms it covers.
//!
//! The build falls back to scaling the captured outline, so the drawn path has
//! to be visible on its own: a base letter whose variant the registry builds
//! must have all six forms rather than a scaled copy under a drawn name.

use lemmata_build::texture::FORMS;
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

    // These are ported definitions, including the composed `i`, so their forms
    // are re-drawn.
    for base in ["i", "l", "m"] {
        for form in FORMS {
            assert!(
                drawn.contains_key(&(base.to_string(), *form)),
                "{base} has no drawn {}",
                form.suffix()
            );
        }
    }
}
