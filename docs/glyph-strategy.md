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

What remains of the layer: `letter-model.mjs` (`DivFrame`, `Sw`, `adviceStroke2`), the `glyph` package (`GlyphProc`, `union`, `difference`, `include`, the block executor), `util`, and `aesthetics`. Roughly 2,500 lines.

## How the measurement was taken

The scratch build's `packages/geometry/src/index.mjs` was wrapped with counters and JSON capture, the build cache was removed, and `npm run build -- ttf-unhinted::RefA` was run with `LEMMATA_DUMP` set. The same run produced the captured skeletons that `crates/lemmata-geom/tests/glyph_spiro.rs` replays. The instrumented tree is disposable and lives outside the repository.
