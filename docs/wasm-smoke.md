# wasm32 smoke results

Checked 17 October 2026 with rustc 1.99.0 on x86_64-unknown-linux-gnu, target wasm32-unknown-unknown. Each crate was checked alone in a scratch project with `cargo check --target wasm32-unknown-unknown --no-default-features --features <crate>`.

| Crate | Result |
| --- | --- |
| facet 0.50.0-rc.7 | pass |
| facet-toml 0.50.0-rc.7 | pass |
| vox-schema 0.10.0-rc.7 | pass |
| ttf2woff2 0.14.0 | pass |
| picante 3.0.0-rc.5 | fail |

picante pulls tokio 1.53.3 with the `fs` and `rt-multi-thread` features. Tokio refuses to build for wasm32-unknown-unknown with those features enabled. The dependency is not optional in picante, so picante cannot run in the browser as published. Consequence: the browser build uses the in-house memo backend and picante stays native-only.

The ttf2woff2 result settles the WOFF2 question for the browser build. WOFF2 export does not need a C++ toolchain or a native-only path.
