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

It is not enough. Something in the dependency closure still computes with the platform, and it is not yet identified.

What was ruled out. `kurbo` has a `libm` feature, and it only takes that path when its `std` feature is off, so the workspace now builds it with `default-features = false, features = ["libm"]`. That changed neither output: its divergent functions are not on the path from a point to an outline. `write-fonts` calls `log2` once, in the search-range calculation, where the argument is a small count and `floor` of its logarithm lands the same way on both targets. `linesweeper` computes only `hypot2` and `sqrt`, and the `hypot` calls left in `lemmata-geom` are all `kurbo`'s, which is written as `sqrt(x² + y²)` on purpose. `mul_add` agrees on both targets. There is no function-call form of these left in our crates; the routing catches all of them.

What is established. Every transcendental our own code uses now goes through one implementation, so the divergence is not ours. It is confined to `glyf`, two bytes across two glyphs, and every other table including MATH is byte-identical. The wasm build's output has not changed at any point in this work, while the native one moved four bytes and then stopped: the native side is converging towards what wasm already computes, and has not arrived.

The next step is to bisect the pipeline by stage. `crates/lemmata-web` can expose the contours or the tree arcs and the two targets can be compared there, which localises it to the geometry or to the encoding. Until then `r[wasm.parity-bytes]` stands and is unmet, rather than being relaxed to match what the code does.
