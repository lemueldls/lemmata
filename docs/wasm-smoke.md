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

## Native and wasm do not agree on the last bit

Every crate in the pipeline compiles for `wasm32-unknown-unknown`, and the browser build runs: `crates/lemmata-web` produces a font from a capture, and the page offers it as a download. Checked 17 October 2026.

What it does not do is produce the *same* font. Building the same capture on the host and in wasm gives `glyf` tables that differ by two bytes, across two glyphs whose point counts differ by three and two. Everything else, including the MATH table, is byte-identical.

The cause is float math. Rust's `f64` methods call the platform's, and on `wasm32-unknown-unknown` that is Rust's own port of `libm` rather than the host's C library. A sweep over 400 inputs shows `sin`, `cos`, `tan`, `tanh`, `exp`, `ln`, `log2`, `pow` and `hypot` all differing, mostly by one unit in the last place. `atanh` and `sqrt` agree. The pipeline routes every one of those calls through `lemmata_geom::math`, which is `libm` on both targets, so our own arithmetic now agrees by construction.

It is not enough, because the dependencies compute with the platform too. `kurbo` calls `atan2`, `acos`, `cbrt`, `cos`, `sin`, `tan`, `ln` and `log2` internally, and those sit on the path from a point to an outline.

So `r[wasm.parity-bytes]` as written, byte-identical output between the native and browser builds, is not reachable while a dependency on the path uses platform math. The options are to relax it to geometric parity inside the existing drift budget, which is what the tolerance policy already expresses, or to audit and vendor or patch every math call the path touches. It needs a decision before the requirement is claimed.
