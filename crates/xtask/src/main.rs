//! Repository tasks for CI and local checks.
//!
//! Run through `cargo xtask <command>`.

mod parity;
mod port_status;
mod requirements;

use std::process::ExitCode;

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("requirements") => {
            for (id, stage) in requirements::ENFORCED_FROM {
                println!("{id}\t{stage}");
            }
            ExitCode::SUCCESS
        }
        Some("regen") => match parity::regen() {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("lemmata: {message}");
                ExitCode::FAILURE
            }
        },
        Some("parity") => match parity::check() {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("lemmata: {message}");
                ExitCode::FAILURE
            }
        },
        Some("port-status") => {
            // An incomplete port is the normal state, not a failure.
            port_status::run();

            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("usage: cargo xtask <requirements|port-status>");
            ExitCode::FAILURE
        }
    }
}
