# Dependency versions

Pinned 17 October 2026. Update only together with a drift report.

## Release candidates (exact pins)

| Crate | Version | For |
| --- | --- | --- |
| facet | =0.50.0-rc.7 | Reflection the CLI's argument structs derive |
| figue | =5.0.0-rc.6 | CLI arguments, config and environment from one Facet struct |

A candidate nothing depends on belongs in neither the manifest nor this table.

## Font stack

| Crate | Version |
| --- | --- |
| read-fonts | 0.45.1 |
| write-fonts | 0.54.1 |
| sha2 | 0.10.9 |
| skrifa | 0.48.0 |
| kurbo | 0.13.1 |
| libm | 0.2.16 |
| linesweeper | 0.5.0 |
| harfrust | 0.14.0 |
| ttf2woff2 | 0.14.0 |
| wasm-bindgen | 0.2.129 |

## Tooling

| Tool | Version |
| --- | --- |
| rust | nightly-2026-09-30, wasm32-unknown-unknown installed (`rust-toolchain.toml`) |
| tracey | 1.3.0 |
| capn | 1.4.0 |
| cargo-deny | latest at install time |
| ots-sanitize | system binary, version recorded in CI |
| wasm-pack | 0.15.0, with the wasm-bindgen 0.2.129 CLI cached |

## Upstream oracle

Iosevka v34.9.0, commit recorded in `docs/upstream.md`.
