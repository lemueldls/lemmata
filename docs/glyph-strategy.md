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

The ported runtime and the definition file in the tree are that fallback being costed, not the plan: they measure what a port needs and they are the layer a port would build on. Nothing on the path that writes a font reads them. `cargo xtask port-status` counts them against the mirrored tree, which is how far the fallback would have to go before it could be chosen instead.

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

The control layer is ported. `lemmata-ptl::control` carries the controls, the flattener and the collector; `lemmata-ptl::spiro_kit` carries the knot builders, the directed pairs, the amendments, the interpolators and the sink helpers. That is `spiro-control.mjs` and the builders in `spiro-kit.mjs`, about 900 of the 3,470 lines.

`reference/spiro-kit-oracle` runs upstream's own layer over `corpus/spiro-kit.json` and writes `golden/spiro-kit.json`; `crates/lemmata-glyphs/tests/spiro_kit.rs` builds the same sequences and compares. Sixteen cases and 69 knots, with every type, width, importance and normal exact and coordinates within 1e-9. The tolerance is for the arc interpolators, which go through `cos`, `sin` and `pow`: V8's implementations differ from Rust's by around 3e-14, so bit-exactness is not available there without matching V8's libm.

The metrics layer is ported too. `lemmata-ptl::metrics` carries `calculateMetrics` from `meta/aesthetics.ptl` and the stroke-width advice from `weight-control.mjs`: the frame, the stroke width, the arch depths, the overshoots, the balance offsets, the ess ratios, and the weight-control sigmoid. The capture writes the plan parameters the build used and the metrics it derived from them, and `tests/metrics.rs` compares all 105 fields. The worst deviation is 1.4e-14, from `tan`, `atanh` and `tanh` differing between V8 and Rust in the last bits.

The per-glyph frame is ported as well. `lemmata-ptl::letter_model` carries `DivFrame`: the division, the sidebearings, the middle, the stroke width, the arch depths, and the slicing and advice methods. The capture dumps eleven frames at several divisions plus the values they derive, and `tests/letter_model.rs` compares 275 of them. The worst deviation is 2.8e-14.

The composition layer is ported too. `lemmata-ptl::glyph` carries `Glyph` (the accumulating combine, the transforms, the width, the context tag), `GlyphProc` and `ForkGlyphProc`, `combineWith`, and the boolean procs from `boole-kit.mjs` (`union`, `intersection`, `difference`, `with-knockout`). `reference/glyph-oracle` builds compositions with upstream's own layer and `tests/glyph_compose.rs` compares thirteen of them: includes, transforms, widths, references, nested procs, tag ejection and booleans. Worst deviation 0.000.

The numeric vocabulary is ported as `lemmata-ptl::util`: `mix`, `barMixL`, `linreg`, `clamp`, `quantize`, `fallback`, `bez2`, `bez3`, `boole`, `boolePn`, `strokeOffset`, `slY`, `slX`, `distP` and the waveforms. `reference/util-oracle` runs upstream's helpers over a grid and `tests/numeric.rs` compares 124 results exactly. Two of them need JavaScript's arithmetic rather than Rust's: `quantize` rounds half toward positive infinity, and the square waveform returns zero for zero where Rust's `signum` returns one.

The collection path is ported as `lemmata-ptl::store`: `GlyphStore` and the save sink. A definition saves through the sink, which names the glyph, derives a `uniXXXX` name from a bare code point, refuses a duplicate, takes the advance width and the global transform from the metrics, and files the result. `reference/glyph-oracle/store-oracle.mjs` runs nine saves through upstream's own sink and store and `tests/glyph_store.rs` replays them, including a save the build filter skips, a forced save and a duplicate name. Exact.

## The first file

`letter/latin/lower-a.ptl` is ported whole. It builds thirteen `a` variants, a
half-height `aHalf` for each serifless double-storey one and a cap-height `aCap`
for each single-storey one, and all sixty are verified. The port is
`lemmata-ptl::shapes` (the helpers a definition calls) and
`lemmata-glyphs::letter::latin::lower_a` (the bodies and the variant table's
stroke advice), 1403 lines, plus `tests/lower_a.rs`.

`shapes.rs` carries `determineMixR`, `HookShape`, the `Arch` namespace including
`centerAt`, `flatside` in its four orientations, `HSerif.rt` and `.rb`, `VBar.l`
and `.r`, `ArcStartSerif`, `InwardSlabArcStart`, `bezControls` and the clamped
arch depths. `lower_a.rs` carries `DoubleStorey`'s three hook styles and seven
bodies, `SingleStorey`'s five bodies over five bars, and `CrossingBody`.

`tests/lower_a.rs` compares the bi-knots of every `dispiro` sequence against the
ones the build produced: 60 variants, 2057 knots, worst deviation 0. Every
coordinate, knot type, importance and normal matches, and so do the expansions
the solver consumes. That is the whole shape path, so the solver, the boolean
stage and the outline all follow from a match here. The outline's bounding box
is checked as well, because `OBarRight.shape` is `OBarLeft.shape` rotated half a
turn and a missing or wrong flip leaves every knot in the right place.

Two things the layer did not have and the port needed:

- `Metrics::stroke_width_blend`, which the `SmoothA`/`SmoothB` advice calls.
- `FunctionInterpolator`. Upstream's `HookStart` and `Arch.lhs`/`Arch.rhs` are
  interpolators that read the knots either side of them when the flattener runs.
  A definition writes those neighbours itself, one line above and one below, so
  the port passes them in directly and returns the resolved knots. The arc
  blends the arch emits stay interpolators for the flattener. This avoids
  teaching the flattener to let an interpolator move its neighbours, which
  upstream's `HookShape` does: it shifts the terminal knot it attaches to and
  gives it a heading. The port returns that knot and the caller substitutes it.

The capture is the oracle for this layer. PTL functions cannot be imported into
JavaScript the way the geometry and kit modules can, so the only way to run
upstream's own code is a full build, and the capture already is one.

Getting there turned up four traps worth recording, all in `docs/glyphs.md`:
the four heading constants in `meta/aesthetics.ptl` point along the stroke, not
along the axes, so `Upward` is `(-HVContrast, 0)` and `Leftward` is `(0, -1)`;
`archv` and `arcvh` are blends rather than knots, so writing a bare `g4` where
one belongs keeps the item count and loses knots; the captured knots for the
earless corner are the *unflipped* corner inside a `Transformed`; and PTL's
indented arguments are siblings, so the tailed bar's `Math.max` takes three.

What remains of the port surface is the dependency manager's block half and
`relation.mjs`, which order glyphs and drive subsetted rebuilds rather than
outlines. After that the definitions themselves, which pull from the shared PTL
libraries (`common/shapes.ptl` 913, `common/derivatives.ptl` 382,
`letter/shared.ptl` 1,389 and its neighbours), part of the 60,020 and amortized
across glyphs.

## How the measurement was taken

The scratch build's `packages/geometry/src/index.mjs` was wrapped with counters and JSON capture, the build cache was removed, and `npm run build -- ttf-unhinted::RefA` was run with `LEMMATA_DUMP` set. The same run produced the captured skeletons that `crates/lemmata-geom/tests/glyph_spiro.rs` replays. The instrumented tree is disposable and lives outside the repository.

## The host runs

The half that turns parameters into outlines now exists for the native path. `lemmata-host` writes upstream's build plan for one shape, runs that build through `npm`, and reads back the capture it leaves. Everything downstream is unchanged, because what crosses the boundary is the same capture the frozen ones were; the plan is what makes it a seam rather than an artifact.

The gate is a shape nothing in the repository holds a capture for. RefB is slab serifs at bold, and 24 letters of the capture agree with upstream's own `RefB-Bold.ttf` within two units of ink. The tolerance is sharp rather than generous: the bold `m` is 482 units wide against the regular one's 398, so a capture of the wrong shape fails on the first glyph. The test skips unless `LEMMATA_HOST_BUILD` is set, because it starts an external build.

`lemmata-cli build` takes `--weight`, `--width`, `--slope`, `--serifs` and `--spacing`, and names a shape that way. The first build of a shape takes tens of seconds and lands under `target/host`; the next one reads the cache. A build is tens of seconds and the shape is the whole of the cache key, so a cached build does not re-run the dump and the host clears its output directory before each run rather than trusting it.

What is left of the seam is two things. The shape is a set of flags rather than a field a plan file carries. And the browser has no host at all: in a page the JavaScript engine comes free, but the glyph code has to be bundled for it, and the store builder that upstream keeps inside its build has to come with it.

## The frontend is free

The port's cost was never the semantics. `lemmata-ptl` carries those, around 3,350 lines across the layers, verified against upstream layer by layer. What was unmeasured was the frontend: PTL is a Lisp with infix and indentation on Patrisika, and nearly all of its surface is 28 macros, so recounting it looked like the project.

It is not. The pinned compiler emits three trees, and one of them is exactly what a transpiler wants:

| dump | size for `lower-a.ptl` | what it is |
| --- | --- | --- |
| `--dump-ast` | 38 KiB | the reader: S-expressions, macros unexpanded |
| `--dump-expanded` | fails to serialise | the macro-free core; the tree is circular |
| `--dump-transformed` | 72 MiB | ESTree, after lowering |

Twelve node kinds carry that file. `Identifier` 2335, `Literal` 678, `MemberExpression` 493, `CallExpression` 438, `VariableDeclarator` 363, `ExpressionStatement` 191, `BinaryExpression` 130, `NewExpression` 96, `AssignmentExpression` 76, `ArrayExpression` 70.

The 72 MiB is not the program. Every node carries `within`, the whole source pasted into it, which is also why the expanded dump cannot be stringified: the metadata is circular. Strip it, and the tree underneath is small and regular, in a documented shape rather than a bespoke one.

So the frontend is a reference script and a JSON reader, not a language implementation. What is left is the mapping: ESTree onto `lemmata-ptl`, with the Rust compiler as the type gate and the capture as the value gate. `reference/glyph-ast/oracle.mjs` reports the sizes and kinds above for any file, so the subset can be measured before it is built.
