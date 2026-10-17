# Glyph strategy

How Lemmata gets from Iosevka's glyph definitions to outlines. Measured against the RefA plan on 17 October 2026, with the build cache cleared so the counts cover a full build.

## The two sizes

The glyph definitions are large and keep changing: 349 `.ptl` files, 62,560 lines, 2,669 `create-glyph` blocks (docs/inventory.md).

The geometry surface they call is small. A full RefA build (two faces) recorded these calls:

| Primitive | Calls |
| --- | --- |
| `CombineGeometry.toBezArcs` | 370,920 |
| `SpiroGeometry.toBezArcs` | 358,892 |
| `DiSpiroGeometry.toBezArcs` | 266,786 |
| `TransformedGeometry.toBezArcs` | 95,869 |
| `BooleanGeometry.toBezArcs` | 95,358 |
| `ReferenceGeometry.toBezArcs` | 86,769 |
| `SimplifyGeometry.toBezArcs` | 70,915 |
| `SimplifyGeometry.toContours` | 70,915 |
| `StrokeGeometry.toBezArcs` | 972 |
| `SpiroPenGeometry.toBezArcs` | 480 |
| `ContourSetGeometry.toBezArcs` | 306 |
| `RemoveHolesGeometry.toBezArcs` | 28 |

Twelve primitives, about 1.5 million calls over roughly 71,000 glyphs.

## What the numbers say

The primitive set is small enough to port. A dozen class methods cover everything these faces needed, and they are the same methods `lemmata-geom` is being built around.

The call volume is not small. Routing 1.5 million calls one by one across a JavaScript-to-Rust boundary would dominate a build, so the boundary has to be coarse. One crossing per glyph evaluates about 71,000 geometry graphs, or the graph can be handed to Rust once and walked there.

## Decision

Host upstream's compiled glyph code, with the geometry graph evaluated in Rust behind a per-glyph boundary.

The glyph definitions stay where upstream maintains them, so an update is a tag bump rather than a re-port of 62,560 lines (the alternative). The Rust half is a small, testable API, and the oracle stays the drift gate. The bundled glyph code is upstream's source in another form, so the glyph pipeline stays under the OFL.

If coarse batching turns out to be impossible, the fallback is hand-porting the glyphs, measured against the same oracle.

## Port surface

A pilot on `letter/latin/lower-a.ptl` (473 lines, and the file behind the captured `a`) asked what a port needs beyond the twelve primitives.

The primitives are what a *host* shims. A *port* also needs the control-knot layer the definitions are written in, because that is what turns `corner`, `curl`, `flat`, `archv` and `widths.rhs` into the knots the primitives consume:

| Piece | Lines | Language |
| --- | --- | --- |
| `geometry/spiro-control.mjs` | 352 | JS |
| `font-kits/spiro-kit.mjs` | 549 | JS |
| `font-kits/letter-model.mjs` | 298 | JS |
| `font-kits/weight-control.mjs`, `boole-kit.mjs` | 122 | JS |
| `glyph/glyph.mjs`, `block.mjs`, `relation.mjs`, `store.mjs` | 1,161 | JS |
| `util` | 443 | JS |
| `meta/aesthetics` | ~544 | PTL |
| **Total** | **~3,470** | mostly plain JS |

That layer is one-time and shared, and the whole corpus uses **51 distinct names** from it. The twenty most used are `corner`, `heading`, `dispiro`, `flat`, `curl`, `g4`, `widths.rhs`, `widths.center`, `widths.lhs`, `spiro-outline`, `archv`, `Arch.lhs`, `Arch.rhs`, `Arch.adjust-x.bot`, `widths.heading`, `g4.down.mid`, `Arch.adjust-x.top`, `g4.up.start`, `g4.right.mid`, `g4.down.start`.

So the two strategies cost, in round numbers:

- **Host**: the twelve primitives, which `lemmata-geom` already implements, plus an engine.
- **Port**: ~3,470 lines of shared DSL, then the definitions per file, with the stage capture as an exact gate.

The second number is about twenty times the first, which is the correction this pilot makes to the Phase 1 reading. The definitions themselves are 60,020 lines across 349 files, and a single variant of one glyph is a small slice of its file: `a.doubleStoreySerifless` is the union of two shape functions, roughly 60 lines of PTL.

What the pilot did not measure is how fast an automated loop converges on one file with the oracle as its gate. That number, not the line counts, decides whether the definitions are portable by hand or only by machine.

## Port progress

The control layer is ported. `lemmata-glyphs::control` carries the controls, the flattener and the collector; `lemmata-glyphs::spiro_kit` carries the knot builders, the directed pairs, the amendments, the interpolators and the sink helpers. That is `spiro-control.mjs` and the builders in `spiro-kit.mjs`, about 900 of the 3,470 lines.

`reference/spiro-kit-oracle` runs upstream's own layer over `corpus/spiro-kit.json` and writes `golden/spiro-kit.json`; `crates/lemmata-glyphs/tests/spiro_kit.rs` builds the same sequences and compares. Sixteen cases and 69 knots, with every type, width, importance and normal exact and coordinates within 1e-9. The tolerance is for the arc interpolators, which go through `cos`, `sin` and `pow`: V8's implementations differ from Rust's by around 3e-14, so bit-exactness is not available there without matching V8's libm.

The metrics layer is ported too. `lemmata-glyphs::metrics` carries `calculateMetrics` from `meta/aesthetics.ptl` and the stroke-width advice from `weight-control.mjs`: the frame, the stroke width, the arch depths, the overshoots, the balance offsets, the ess ratios, and the weight-control sigmoid. The capture writes the plan parameters the build used and the metrics it derived from them, and `tests/metrics.rs` compares all 105 fields. The worst deviation is 1.4e-14, from `tan`, `atanh` and `tanh` differing between V8 and Rust in the last bits.

The per-glyph frame is ported as well. `lemmata-glyphs::letter_model` carries `DivFrame`: the division, the sidebearings, the middle, the stroke width, the arch depths, and the slicing and advice methods. The capture dumps eleven frames at several divisions plus the values they derive, and `tests/letter_model.rs` compares 275 of them. The worst deviation is 2.8e-14.

The composition layer is ported too. `lemmata-glyphs::glyph` carries `Glyph` (the accumulating combine, the transforms, the width, the context tag), `GlyphProc` and `ForkGlyphProc`, `combineWith`, and the boolean procs from `boole-kit.mjs` (`union`, `intersection`, `difference`, `with-knockout`). `reference/glyph-oracle` builds compositions with upstream's own layer and `tests/glyph_compose.rs` compares thirteen of them: includes, transforms, widths, references, nested procs, tag ejection and booleans. Worst deviation 0.000.

The numeric vocabulary is ported as `lemmata-glyphs::util`: `mix`, `barMixL`, `linreg`, `clamp`, `quantize`, `fallback`, `bez2`, `bez3`, `boole`, `boolePn`, `strokeOffset`, `slY`, `slX`, `distP` and the waveforms. `reference/util-oracle` runs upstream's helpers over a grid and `tests/numeric.rs` compares 124 results exactly. Two of them need JavaScript's arithmetic rather than Rust's: `quantize` rounds half toward positive infinity, and the square waveform returns zero for zero where Rust's `signum` returns one.

The collection path is ported as `lemmata-glyphs::store`: `GlyphStore` and the save sink. A definition saves through the sink, which names the glyph, derives a `uniXXXX` name from a bare code point, refuses a duplicate, takes the advance width and the global transform from the metrics, and files the result. `reference/glyph-oracle/store-oracle.mjs` runs nine saves through upstream's own sink and store and `tests/glyph_store.rs` replays them, including a save the build filter skips, a forced save and a duplicate name. Exact.

What remains: the dependency manager's block half and `relation.mjs`, which order glyphs and drive subsetted rebuilds rather than outlines. After that the definitions themselves, which pull from the shared PTL libraries (`common/shapes.ptl` 913, `common/derivatives.ptl` 382, `letter/shared.ptl` 1,389 and its neighbours), part of the 60,020 and amortized across glyphs.

## How the measurement was taken

The scratch build's `packages/geometry/src/index.mjs` was wrapped with counters and JSON capture, the build cache was removed, and `npm run build -- ttf-unhinted::RefA` was run with `LEMMATA_DUMP` set. The same run produced the captured skeletons that `crates/lemmata-geom/tests/glyph_spiro.rs` replays. The instrumented tree is disposable and lives outside the repository.
