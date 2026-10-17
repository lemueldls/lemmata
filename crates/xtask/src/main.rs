//! Repository tasks for CI and local checks.
//!
//! Run through `cargo xtask <command>`.

mod blank_lines;
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
        Some("port-status") => {
            // An incomplete port is the normal state, not a failure.
            port_status::run();

            ExitCode::SUCCESS
        }
        Some("blank-lines") => {
            let check = std::env::args().any(|arg| arg == "--check");
            match blank_lines::run(check) {
                Ok(0) => {
                    println!("every tail expression is already separated");
                    ExitCode::SUCCESS
                }
                Ok(added) if check => {
                    eprintln!("{added} tail expressions need a blank line before them");
                    ExitCode::FAILURE
                }
                Ok(added) => {
                    println!("added {added} blank lines");
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("lemmata: {message}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("usage: cargo xtask <requirements|port-status|blank-lines [--check]>");
            ExitCode::FAILURE
        }
    }
}
