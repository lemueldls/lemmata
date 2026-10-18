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

- **Fix it inside kurbo**, by making `libm` win when both it and `std` are enabled, or by giving the float backend its own feature so `std` and deterministic math stop being the same switch. This is the only place the fix can live, and the reason is that patching the *enablers* does not end.
- Patch or vendor `kurbo` with its call sites rewritten to go through `libm` directly when the feature is on. Works today, costs a vendored copy (18,310 lines, 788 KB) carrying a mechanical but wide diff, maintained until upstream takes the fix.
- Treat `wasm.parity` as the font-unit fallback the risk table already names, with a documented budget. Cheapest, and it is a real answer, but the measured gap does not fit it: `g` is 10.57 font units apart, and font-unit precision is not ten units.

### Patching the enablers does not end

`linesweeper` was the first enabler found:

```
kurbo v0.13.1
└── kurbo feature "default"
    └── linesweeper v0.5.0
```

It was vendored and changed to ask for `kurbo` without default features. That moved the enabler rather than removing it:

```
kurbo v0.13.1
├── kurbo feature "default"
│   └── write-fonts v0.54.1
│       └── lemmata-build
└── kurbo feature "libm"
    └── (our crates)
```

`write-fonts` enables the same defaults, it is third-party, and it is not the last one that would be found. Cargo features are additive and there is no way to opt out from a dependent, so a chain of vendored copies is the shape of that approach, and it grows every time a dependency changes. The vendored `linesweeper` was reverted, since on its own it achieved nothing.

## What vendoring kurbo showed

kurbo was vendored and its `libm` feature made to imply `no_std`, which removes the inherent `f64` methods from the crate and makes every float call resolve to its own `FloatFuncs` trait, which is `libm`. A successful build is therefore proof that the routing is total, not a claim. `cargo tree` confirms `kurbo feature "libm"` is on. All 42 test targets pass.

The native build changed (11,216 to 11,264 bytes), so kurbo's math was on the path after all, and the earlier conclusion that it was not came from the feature never having taken effect.

It did not close the gap. The deviation is **10.5745 font units**, on `g`, exactly as before vendoring. Three more experiments say the same thing:

| change | deviation |
| --- | --- |
| our own transcendentals routed through `lemmata_geom::math` | 10.5745 |
| kurbo on `libm` | 10.5745 |
| native built in release rather than debug | 10.5745 |

Identical to four decimals across all of them. This was read at the time as ruling float math out, on the reasoning that noise would move the number. That reading was wrong, and the resolution at the end of this document says why: the number did not move because the un-routed calls were never on the path of the worst glyph, not because float math was absent.

### Pointer width is not it either

The hypothesis was `linesweeper`'s `rustc-hash`, whose `FxHasher` feeds the bytes of a `usize` into its hash, so iteration order would differ between a 32-bit and a 64-bit target. It was tested by building the same CLI for `i686-unknown-linux-gnu`, which is 32-bit with a host libc, and comparing all three builds:

| pair | worst deviation | glyph |
| --- | --- | --- |
| x86_64 vs wasm32 | 10.5745 | g |
| i686 vs wasm32 | 10.5745 | g |
| x86_64 vs i686 | 3.1770 | gid 64 |

Refuted. A 32-bit native build is no closer to wasm32 than a 64-bit one: the number is identical to four decimals, on the same glyph. So the `g` divergence is not pointer width.

What the numbers do say. The `g` gap is the same against either native, so the wasm build differs by a fixed discrete amount rather than by an amount that grows with the host's. That is a wasm-specific decision, not platform noise. The two natives also differ from each other, by 3.18 units on a different glyph, which is consistent with the same code meeting two architectures' libc float math.

Where that leaves it. Float math is ruled out for the `g` gap, because routing every transcendental we call, plus all of kurbo's, through `libm` did not move it, and neither did the build profile. Pointer width is ruled out by the table above. What remains is a call or a `cfg(target_arch)` path that still resolves differently on wasm32 specifically, and the cheapest way to find it is the sweep that found the first set: hash a range of inputs for the float functions not yet checked, `asin`, `acos`, `atan`, `sinh`, `cosh`, `cbrt`, `exp2`, `log10`, on both targets and see which one differs. That is a small program and it localises the same way.

The vendored kurbo is unrelated to the measured gap, and the honest reading is that it should be reverted unless it is wanted for the determinism property on its own merits.

## The bisect places it in the expansion

The pipeline from a captured tree to a font has four stages worth separating: the spiro expansion, the solve, the overlap resolution, and the quadify plus the writer. `lemmata_web` grew a temporary `expansion_report` that hashes the two expanded sides of every glyph, and the browser diffs it against the host's.

Ten of the seventy-one glyphs differ at the expansion, the first stage after the tree. The named ones are `k`, `s`, `dollar`, `y`, `two.lnum` and `three.lnum`.

That is a large narrowing. The divergence is not in the quadifier, not in the font writer, not in overlap resolution, and not in the boolean engine: all of those run after the expansion and cannot introduce it. What remains is `lemmata_geom`'s expansion path, which is our own code, where every transcendental already goes through `lemmata_geom::math` and the remaining float operations are exact.

Ten of seventy-one rather than all of them was read as a tie or a threshold being crossed. It was not: it was which glyphs happened to round differently.

The next step is to print the knot values of one of those ten on both targets rather than hashing them. A hash says they differ; the values say by how much and where, and that is what names the line.

## Resolution: it was un-routed float math

The knot values of `k` on the two targets differ in the last place:

```
native 94.87126081485027  186.65832099282113  -38.112824113375154   91.50409850541625
wasm   94.87126081485027  186.65832099282116  -38.11282411337513    91.50409850541624
```

One to two units in the last place, in the expansion's coordinates. So it was float math after all, and the earlier conclusion that it was not came from an audit whose output was cut off at five lines. On running it in full, `lemmata-geom` and `lemmata-ptl` still had these un-routed:

| file | call | where it matters |
| --- | --- | --- |
| `lemmata-ptl/src/shapes.rs` | five `f64::powf` | the arch blender's superellipse |
| `lemmata-ptl/src/control.rs` | two `f64::hypot` | the distance interpolators |
| `lemmata-ptl/src/util.rs` | two `f64::hypot` | `strokeOffset`, `distP` |
| `lemmata-geom/src/spiro.rs` | three `atan2`, three `hypot` | the spiro solve |
| `lemmata-geom/src/stroke.rs` | two `atan2`, two `hypot` | stroking |
| `lemmata-geom/src/fairize.rs` | one `hypot` | fairization |

The earlier passes had patched the specific literal strings I happened to read, `theta.cos()` and friends, and missed every `powf`, `atan2` and two-argument `hypot` in the same files. Routing them through `lemmata_geom::math` closes it.

Verified: the expansion of `k` is identical, all seventy-two glyphs' expansion hashes are identical, and the two builds are **byte-identical** with a worst deviation of **0.000000** font units. `r[wasm.parity]` and `r[wasm.parity-bytes]` both hold.

The lesson is the audit, not the arithmetic: grepping for the calls I remembered instead of the set that exists, and reading a truncated list as a complete one, cost more than every wrong hypothesis in this document put together.

## The parity pair, and where it is checked

`r[wasm.parity-bytes]` asks for byte-identical pairs to be tracked per pinned toolchain and for a failure when a pair that was byte-identical stops being so. `golden/parity.json` is that record: the upstream commit, the capture it was built from, the plan, the toolchain (`rustc` and `wasm-bindgen`), and the font's sha256. `golden/README.md`'s rule applies, so updating it means a drift report.

Three builds are compared against that one fingerprint, and all three agree:

| build | where | how to run |
| --- | --- | --- |
| native | `crates/lemmata-web/tests/parity.rs` | `cargo test -p lemmata-web --test parity` |
| wasm under node | `harness/parity.mjs` | `node harness/parity.mjs` |
| the actual page | `harness/browser/parity.spec.mjs` | `pnpm --dir harness/browser test` |

`cargo xtask parity` runs all three, and it builds what each one reads. The browser half reads the built site rather than the sources, so the wasm package it embeds and the Astro build around it are rebuilt by the same command before the suite runs. Leaving either to a manual step is how the suite came to check a stale font: the page reported a hash from before the MATH constants were written while the native and node builds were current, and every rebuild of the package went to a directory the tests do not read. That half also needs `pnpm`, so it is skipped with a line saying so when the shell is the default one rather than the browser one.

`cargo xtask regen` rebuilds the fingerprint, carrying the upstream commit over from the file it replaces, and regenerating twice produces the same file.

The profile does not move the bytes — checked before the fingerprint was written — so one hash covers a debug `cargo test` and a release wasm build.

`harness/parity.mjs` has no dependencies on purpose: node's own crypto and a `wasm-pack --target nodejs` build are enough, so the pair can be checked offline and in CI. The browser suite is the one that needs an install: `pnpm --dir harness/browser install` and `pnpm --dir harness/browser exec playwright install chromium` are enough, and `nix develop .#browser` is the hermetic alternative, providing a matched driver and browser without a download. The default shell does not carry them, because it has to build from the store alone.

What the browser suite still owes §7.1 beyond parity: the MathML torture page, offline mode, and the latency bench behind `r[wasm.responsive]`.
