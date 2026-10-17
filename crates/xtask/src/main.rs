//! Repository tasks for CI and local checks.
//!
//! Run through `cargo xtask <command>`.

mod requirements;

use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::args().nth(1).as_deref() == Some("requirements") {
        for (id, stage) in requirements::ENFORCED_FROM {
            println!("{id}\t{stage}");
        }
        return ExitCode::SUCCESS;
    }

    eprintln!("usage: cargo xtask requirements");
    ExitCode::FAILURE
}
