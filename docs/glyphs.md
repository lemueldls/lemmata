# Glyphs

Glyph outlines come from upstream's glyph definitions, replayed through `lemmata-geom`. That is the seam the glyph strategy chose: the definitions stay where upstream maintains them, the geometry is ours.

## Capture

A patched Iosevka build writes, for each selected glyph:

- its name and unicodes,
- its advance width,
- its glyph gizmo,
- the geometry tree its definitions built, as a small binary encoding,
- each di-spiro node's expanded sides and the raw arcs the solver produced from them,
- the arcs the tree resolves to, before quadification,
- and the quadratic contours Iosevka's own conversion produced.

The set the tests run against covers the lower-case letters, the lining digits, and the punctuation the font needs, plus the `a` variants the definition port works through: the double-storey and single-storey families, the hooked, tailed and serifed forms, `a.crossing` and `aCap`. `reference/glyph-capture/README.md` has the command.

Selection is by codepoint (`LEMMATA_GLYPH_CODEPOINTS`) or by name (`LEMMATA_GLYPH_SET`), the output path is `LEMMATA_GLYPH_DUMP`, and `LEMMATA_GLYPH_WIDTH` pins the width so a build that emits more than one font dumps only the one asked for. Without that pin the dump lands on whichever font the build writes last, which is how an earlier capture ended up holding the extended width while the reference font was the regular one. The side data lands beside the binary as `<path>.json` (contours), `<path>.arcs.json`, `<path>.tree-arcs.json`, `<path>.expansions.json` and `<path>.fairized.json`. The patched tree is disposable and lives outside the repository, like the other oracles.

`crates/lemmata-glyphs` decodes the capture into `lemmata_geom::Geometry` values. `tests/capture.rs` renders each glyph with `Geometry::Simplify { gizmo, geom }.to_contours()`, which is what the build does, and compares the result with the captured contours; it skips when no capture is present. The comparison is coverage-aware: a boundary point of one outline that lies outside the other counts as drift, while a point inside the other is a seam that two boolean engines can split differently while describing the same region.

## Building a font

`lemmata-build` lays out `head`, `hhea`, `maxp`, `OS/2`, `hmtx`, `cmap`, `name`, `post`, `glyf` and `loca` with `write-fonts`. `lemmata build` reads a capture and writes a TrueType file. The metrics default to Iosevka's: 1000 units per em, ascender 965, descender -215, line gap 70, x-height 520, cap height 735.

`crates/lemmata-cli/tests/build.rs` reads the font back with `read-fonts` and checks that every mapped codepoint has the captured advance and bounding box. It also compares the bounding boxes against `target/reference/RefA-Regular.ttf`: 58 codepoints, worst difference 1 unit.

## Stages

The capture carries each stage's output, so `crates/lemmata-glyphs/tests/arcs.rs` can pin a divergence to the stage that produced it:

- `expansions_match` compares the expanded sides, knot positions, types and importance. Exact.
- `raw_solve_arcs_match` compares the spiro solve on those knots: sampled points, arc length and bend. Exact.
- `tree_arcs_match` compares the tree's arcs, before overlap resolution.
- `resolved_arcs_are_close` compares the arcs after overlap resolution.
- `fairize_stage_matches` compares the fairized shape, order-insensitively.
- `quadify_on_captured_fairize_matches` runs our quadification on *upstream's* fairized shape and compares the contours, which isolates it from fairization.

The first two are exact, so a regression in the constructors fails loudly instead of hiding inside a tolerance. The rest are measurements: the comparison is coverage-aware, so a seam that two engines split differently does not count as drift, but a region that differs does.

## Seeing the difference

`cargo test -p lemmata-glyphs --test outline_svg` writes `target/lemmata-outlines.svg`: every captured glyph, with Iosevka's outline filled gray, ours stroked blue, and the region where the two disagree filled red. The four widest are repeated magnified below the grid. The red is a boolean difference of the two outlines, so it shows exactly the area the drift is made of rather than a number.

## The open gap

The geometry tree and overlap resolution match upstream exactly: `tree_arcs_match` and `resolved_arcs_are_close` report 59 of 59 glyphs within 0.25 units, worst 0.000. Quadification matches too: run on *upstream's* fairized shape it reproduces upstream's contours exactly, with no point-count difference on any glyph.

Fairization is what is left, on 6 of 59 glyphs, worst 1.4 units on `s`. It is a run-grouping difference: upstream breaks a run where a corner is not smooth, and we merge across some of those breaks. Most of the time the merge changes nothing downstream, because the quadratic fitter produces the same points either way.

The run grouping inherits from the arcs it is handed, and our overlap resolution leaves more splits than upstream's rebuild, which rejoins contiguous pieces of one source arc. `annex` does the same joining from a recovered origin and puts each arc back on its source curve, which took the glyphs with a different arc count from 28 of 59 to 18. It did not move the outline numbers, because the arcs it rejoins were classified smooth anyway.

The budget is calibrated to that, not chosen. The deviation is 0.075 px at 16 ppem and 0.225 px at 48 ppem. Rasterizing the worst glyph at 32 px moves 9 of its 325 pixels by 5% coverage or more, 0.19% of the area summed, and 0.28% at 128 px. That is a handful of antialiasing pixels along edges, so `r[drift.outline]`'s TOL is 6 units and the capture test asserts it. The sixth unit is for boolean-heavy glyphs: `mathbb/E` quadifies into two more points than upstream's and drifts 5.5, while its geometry tree is exact. `cargo test -p lemmata-glyphs --test rendering_svg` writes `target/lemmata-rendering.svg` and prints the pixel table.

The six glyphs that still differ have one cause, and it is upstream's own artifact. Its rebuild emits a straight line for any polygon edge it cannot match to a source arc, and grid snapping leaves edges a hundredth of a unit long. Fairization classifies a junction as a corner when one side is straight and the other is not, so such an edge splits a run. On `s` the peak at (251, 528) carries one, and upstream's run breaks into two where ours stays whole. Reproducing that means reproducing the snapping and the segment table, which is the one part of typo-geom's clipper we substitute.

It survived every earlier test because of coverage. `tests/fairize.rs` and `tests/quadify.rs` each check one half of the step on their own corpora and both are exact, and a boundary comparison that tolerates seams hid the structural difference. The glyph arcs are now in `corpus/fairize.json` as seven minimal reproductions, so the fairize half is pinned against upstream directly.

## The lower-a variants

`lower-a.ptl` builds thirteen `a` variants, a half-height `aHalf` for each
serifless double-storey one, and a cap-height `aCap` for each single-storey one.
The capture holds every one, and the port builds and verifies all sixty: their
bi-knots, expansions and outlines match the build's exactly, 2057 knots, worst
deviation zero.

`DoubleStorey` carries all three hook styles the variant table selects and the
seven bodies over them, `SingleStorey` carries five bodies over five bars, and
`CrossingBody` is `a.crossing`. The derived glyphs (`aSwash`,
`aRetroflexHook`, the script forms, `mathbb/a` and the accented compositions)
need shapes outside this file and are not ported.

Three things about the work are worth writing down, because reading the source
misled me and measuring did not.

**`arcvh` and `archv` are blends, not knots.** A shape that writes `archv` where
a bare `g4` knot looks equivalent will have the right number of *items* and the
wrong number of *knots*: the blend expands into the pair the solver reads.
Emitting `kit.archv(None, None)` instead of a knot is what took
`RightwardTailedBar` from five knots to the capture's nine. The blends are also
independent of the angles: `blendPre -- {}` suppresses the arc while leaving the
key knot's position to `mixR`, which is what `InwardSlabArcStart` and the
toothless corner's rise branch both do.

**The capture holds the unflipped knots.** `OBarRight.earlessCorner` reaches its
outline through `OBarLeft.toothlessCorner`, which is itself
`OBarLeft.earlessCorner` flipped, so the nesting reads as two flips and the shape
as unflipped. The capture says otherwise: the node it holds for
`a.singleStoreyFlatTopSerifless` is a `DiSpiro` inside a `Transformed`, so its
knots are the *unflipped* corner and the flip is one, not zero. Flipping again in
the port is a flip too many, which put the first knot at 126.20 where the capture
has 64.01.

**PTL's arguments are siblings, not nested.** `RightwardTailedBar` writes

```
local hookDepth : Math.max SideJut
    AdviceStroke 8
    0.625 * (SB - 0)
```

and the two indented lines sit at the *same* level, so `Math.max` takes three
arguments and `AdviceStroke 8` is the one-argument call. Reading the second line
as an argument of `AdviceStroke` gives `AdviceStroke(8, 0.625 * SB)`, which
saturates to the full stroke and puts the tail's depth at 70 instead of the
capture's 46.15. That misreading is what the tail's `hookTurn` error was.

Dumping the capture's knots and aligning them against the port's is the way to
settle all three, and it is how the `SmoothA` bug, the `archv` bug and the flip
were found.
