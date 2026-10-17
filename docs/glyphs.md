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

## The open gap

The geometry tree and overlap resolution match upstream exactly: `tree_arcs_match` and `resolved_arcs_are_close` report 59 of 59 glyphs within 0.25 units, worst 0.000. Quadification matches too: run on *upstream's* fairized shape it reproduces upstream's contours exactly, with no point-count difference on any glyph.

Fairization is what is left, on 6 of 59 glyphs, worst 1.4 units on `s`. It is a run-grouping difference: upstream breaks a run where a corner is not smooth, and we merge across some of those breaks. Most of the time the merge changes nothing downstream, because the quadratic fitter produces the same points either way.

The run grouping inherits from the arcs it is handed, and our overlap resolution leaves more splits than upstream's rebuild, which rejoins contiguous pieces of one source arc. `annex` does the same joining from a recovered origin and puts each arc back on its source curve, which took the glyphs with a different arc count from 28 of 59 to 18. It did not move the outline numbers, because the arcs it rejoins were classified smooth anyway.

The six glyphs that still differ have one cause, and it is upstream's own artifact. Its rebuild emits a straight line for any polygon edge it cannot match to a source arc, and grid snapping leaves edges a hundredth of a unit long. Fairization classifies a junction as a corner when one side is straight and the other is not, so such an edge splits a run. On `s` the peak at (251, 528) carries one, and upstream's run breaks into two where ours stays whole. Reproducing that means reproducing the snapping and the segment table, which is the one part of typo-geom's clipper we substitute.

It survived every earlier test because of coverage. `tests/fairize.rs` and `tests/quadify.rs` each check one half of the step on their own corpora and both are exact, and a boundary comparison that tolerates seams hid the structural difference. The glyph arcs are now in `corpus/fairize.json` as seven minimal reproductions, so the fairize half is pinned against upstream directly.
