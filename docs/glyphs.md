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

The budget is calibrated to that, not chosen. The deviation is 0.075 px at 16 ppem and 0.225 px at 48 ppem. Rasterizing the worst glyph at 32 px moves 9 of its 325 pixels by 5% coverage or more, 0.19% of the area summed, and 0.28% at 128 px. That is a handful of antialiasing pixels along edges, so `r[drift.outline]`'s TOL is 5 units and the capture test asserts it. `cargo test -p lemmata-glyphs --test rendering_svg` writes `target/lemmata-rendering.svg` and prints the pixel table.

The six glyphs that still differ have one cause, and it is upstream's own artifact. Its rebuild emits a straight line for any polygon edge it cannot match to a source arc, and grid snapping leaves edges a hundredth of a unit long. Fairization classifies a junction as a corner when one side is straight and the other is not, so such an edge splits a run. On `s` the peak at (251, 528) carries one, and upstream's run breaks into two where ours stays whole. Reproducing that means reproducing the snapping and the segment table, which is the one part of typo-geom's clipper we substitute.

It survived every earlier test because of coverage. `tests/fairize.rs` and `tests/quadify.rs` each check one half of the step on their own corpora and both are exact, and a boundary comparison that tolerates seams hid the structural difference. The glyph arcs are now in `corpus/fairize.json` as seven minimal reproductions, so the fairize half is pinned against upstream directly.

## The lower-a variants

`lower-a.ptl` builds thirteen `a` variants, and the capture holds all thirteen. Three are ported and verified. The rest need six shared shapes that are not: `Arch.adjust-x.top`, `HSerif.rt`, `ArcStartSerif.InwardL`, `VBar.l`, the `arcvh` and `archv` blends, and `bezControls`.

Two things about that work are worth writing down, because reading the source misled me twice and measuring did not.

**`arcvh` and `archv` are blends, not knots.** A shape that writes `archv` where a bare `g4` knot looks equivalent will have the right number of *items* and the wrong number of *knots*: the blend expands into the pair the solver reads. Emitting `kit.archv(None, None)` instead of a knot is what took `RightwardTailedBar` from five knots to the capture's nine.

**The capture holds the flipped shape.** `OBarRight.earlessCorner` reaches its outline through `OBarLeft.toothlessCorner`, which is itself `OBarLeft.earlessCorner` flipped, so the nesting reads as two flips and the shape as unflipped. The capture says otherwise: its first knot for `a.singleStoreyFlatTopSerifless` is `(64.01, 0.00)`, which is the *top* knot of an unflipped corner seen through one flip, not the `yend` knot the source suggests. Dumping the capture's knots and aligning them against the port's is the way to settle it, and it is how the `SmoothA` bug and the `archv` bug were both found.

Two measurements narrow the rest. Aligning the earless corner's first knot against the capture gives `ours (126.20, 200.01)` against `theirs (64.01, 0.00)`, so both of its inputs are wrong in the same way: `HSwToV(sw - fine)` is about sixty units too wide, which says the `fine` that reaches the ring is far below `sw` where the capture needs the two to be nearly equal, and `y_end = bot + ada + TINY` is two hundred where the capture wants `TINY`, which says the arch depth reaching the corner is zero rather than the `smallArchDepth` the plan carries. Both are values the source passes down a chain of wrappers, and both are cheaper to read off a single ring than off a whole glyph.

The capture's ring for `a.singleStoreyFlatTopSerifless` has twenty-three knots, and they say what the two constants are without any tracing:

```
  0 (   64.01,    0.00)     flat:  left + HSwToV(sw - fine),  yend
  1 (  250.00,    0.00)
  2 (  333.38,   13.45)     the arch from the bar's foot
  ...
  7 (  444.00,  200.00)
  8 (  444.00,  320.00)     the straight side, so right + OX reaches 444
  ...
 14 (  285.10,  528.00)     the top, overshooting XH by the overshoot
  ...
 22 (  126.20,  319.99)     curl:  left + TINY,  top
```

The first and last knots pin it: `yend` is zero, and `left + TINY` is 126.20 while the bar sits at 397.15, so the ring's `left` is not the glyph's sidebearing and its `top` is not the x-height. The four numbers a variant needs are all in that list, which is why dumping the ring is cheaper than porting the body and aligning twenty-three knots at a time.

The list also says the knots are already transformed. `collect_di_spiro` walks only `Combine` and `Boolean`, and it reaches this node, so the node is not wrapped in a `Transformed`: `composite-proc` bakes `FlipAround` into the knots it produces. Applying a flip in the port on top of that is a flip too many, which is what put the first knot at 126.20 instead of 64.01.

The remaining two constants are the tail's `hookTurn`, where the port reads `AdviceStroke 16` as taking an extra scalar of one and the capture says otherwise, and the earless corner's `yend`, where the capture's `tiny` says the arch depth that reaches it is zero rather than the `smallArchDepth` of two hundred the plan carries.
