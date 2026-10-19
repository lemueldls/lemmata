# Dependency versions

Pinned 17 October 2026. Update only together with a drift report.

## Release candidates (exact pins)

None. The workspace depends on released versions only, so there is nothing to pin
exactly and nothing to record here. A candidate that nothing depends on is not a
dependency and belongs in neither place: the last ones, `facet`, `facet-toml`,
`figue`, `picante` and `rediff`, were carried here for a while with a row saying
what each was for and no crate using any of them.

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
