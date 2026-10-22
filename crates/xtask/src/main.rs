//! Repository tasks for CI and local checks.
//!
//! Run through `cargo xtask <command>`.

mod expand;
mod ligations;
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
        Some("regen") => {
            match parity::regen() {
                Ok(()) => ExitCode::SUCCESS,
                Err(message) => {
                    eprintln!("lemmata: {message}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("parity") => {
            match parity::check() {
                Ok(()) => ExitCode::SUCCESS,
                Err(message) => {
                    eprintln!("lemmata: {message}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("expand") => {
            let Some(path) = std::env::args().nth(2) else {
                eprintln!("usage: cargo xtask expand <reader dump>");
                return ExitCode::FAILURE;
            };
            match expand::run(&path) {
                Ok(()) => ExitCode::SUCCESS,
                Err(message) => {
                    eprintln!("lemmata: {message}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("ligations") => {
            let (Some(font), Some(out)) = (std::env::args().nth(2), std::env::args().nth(3)) else {
                eprintln!("usage: cargo xtask ligations <font> <out.json> [ligation-set.toml]");
                return ExitCode::FAILURE;
            };
            match ligations::run(&font, &out, std::env::args().nth(4).as_deref()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(message) => {
                    eprintln!("lemmata: {message}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("port-status") => {
            // An incomplete port is the normal state, not a failure.
            port_status::run();

            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("usage: cargo xtask <expand|ligations|requirements|port-status>");
            ExitCode::FAILURE
        }
    }
}
