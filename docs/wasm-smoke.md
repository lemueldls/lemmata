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

It is not enough, and the reason is a dependency feature, not arithmetic.

`kurbo` would solve it. It has a `libm` feature, and it takes that path whenever its own `std` feature is off, so a workspace can ask for `default-features = false, features = ["libm"]` and get one implementation on both targets. That was tried and it changed nothing, and `cargo tree -i kurbo -e features` says why:

```
kurbo v0.13.1
└── kurbo feature "default"
    └── linesweeper v0.5.0
        └── linesweeper feature "default"
```

`linesweeper` enables `kurbo`'s default features, which enables `kurbo/std`, and cargo features are additive: a feature any dependent enables cannot be turned off by another. So `kurbo` computes with the platform's `atan2`, `cbrt`, `cos`, `sin`, `tan`, `ln` and `log2`, and there is no way to opt out from this side of the dependency.

That is the whole of it. Our own transcendentals go through `lemmata_geom::math` and agree by construction; `linesweeper` itself computes nothing but `hypot2` and `sqrt`, both exact; `kurbo` is the remaining platform user on the path, and it is pinned there by its dependent.

Three ways out, in order of how much they cost:

- `linesweeper` declares `kurbo` with `default-features = false`, or grows a `libm` passthrough. Small, correct, and worth telling upstream about, since any user wanting deterministic output hits this.
- Patch or vendor `kurbo` in the workspace. Works today, costs a maintained fork.
- Treat `wasm.parity-bytes` as geometric rather than byte parity, inside the drift budget that already exists. Cheapest, and it is a real answer, but it should be a recorded decision rather than a side effect.

How far apart they are. `crates/lemmata-oracle/tests/parity_probe.rs` flattens both fonts and measures the worst distance from either outline to the other. Set `NATIVE_FONT` and `BROWSER_FONT` and run it. Comparing a font with itself gives exactly zero, so the number is the difference and not an artifact of the measurement.

It is **10.57 font units**, on `g`. One other glyph is above half a unit and the remaining seventy-one are below it. `r[drift.outline]` is calibrated to 5 units, which is the worst residual Iosevka's own grid-snap artifact produces, so `g` differs by more than twice the tolerance this project uses to call two outlines the same shape. At 16 ppem that is about 0.16 px, so it will not show in a text preview, and at display sizes it will. This is a defect, not a rounding artefact to accept.

So the answer to "is it only the last bit" is no. Something reshapes one glyph by ten units, which is a subdivision or a convergence decision changing, not a rounding boundary.

The next step is to bisect the pipeline by stage. `crates/lemmata-web` can expose the contours or the tree arcs and the two targets can be compared there, which localises it to the geometry or to the encoding. Until then `r[wasm.parity-bytes]` stands and is unmet, rather than being relaxed to match what the code does.

## The note to send upstream

To `kurbo` (linebender/kurbo), because the limitation is there rather than in `linesweeper`:

> `FloatFuncs` selects the `libm` implementations with `#[cfg(not(feature = "std"))]`, so `libm` is ignored whenever `std` is on. A dependent that compiles for `wasm32-unknown-unknown` alongside native has no way to ask for one implementation of `sin`, `cos`, `tan`, `exp`, `ln`, `log2` and `pow`: enabling `libm` does nothing while `std` is on, and turning `std` off is not an option for a dependent that needs it. Cargo features are additive, so if any crate in a dependency graph enables `kurbo/default`, which enables `kurbo/std`, the choice is gone for everyone.
>
> The request is for `libm` to take precedence when both are enabled, or for a separate backend selector so `std` and deterministic float math are independent. Our case is a font generator that must produce byte-identical output from a CLI and from a browser; the only difference between the two builds is that the browser's float math is Rust's `libm` port rather than the host libc, and it moves one glyph by ten font units.

`linesweeper` is the reason we cannot turn `std` off: it declares `kurbo` with `features = ["serde"]` and no `default-features = false`, so it enables `kurbo/default`, and it needs std itself.
