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

Selection is by codepoint (`LEMMATA_GLYPH_CODEPOINTS`) or by name (`LEMMATA_GLYPH_SET`), the output path is `LEMMATA_GLYPH_DUMP`, and `LEMMATA_GLYPH_WIDTH` pins the width so a build that emits more than one font dumps only the one asked for. Without that pin the dump lands on whichever font the build writes last, which is how an earlier capture ended up holding the extended width while the reference font was the regular one. The side data lands beside the binary as `<path>.json` (contours), `<path>.arcs.json`, `<path>.tree-arcs.json` and `<path>.expansions.json`. The patched tree is disposable and lives outside the repository, like the other oracles.

`crates/lemmata-glyphs` decodes the capture into `lemmata_geom::Geometry` values. `tests/capture.rs` renders each glyph with `Geometry::Simplify { gizmo, geom }.to_contours()`, which is what the build does, and compares the result with the captured contours; it skips when no capture is present. The comparison is coverage-aware: a boundary point of one outline that lies outside the other counts as drift, while a point inside the other is a seam that two boolean engines can split differently while describing the same region.

## Building a font

`lemmata-build` lays out `head`, `hhea`, `maxp`, `OS/2`, `hmtx`, `cmap`, `name`, `post`, `glyf` and `loca` with `write-fonts`. `lemmata build` reads a capture and writes a TrueType file. The metrics default to Iosevka's: 1000 units per em, ascender 965, descender -215, line gap 70, x-height 520, cap height 735.

`crates/lemmata-cli/tests/build.rs` reads the font back with `read-fonts` and checks that every mapped codepoint has the captured advance and bounding box. It also compares the bounding boxes against `target/reference/RefA-Regular.ttf`: 58 codepoints, worst difference 1 unit.

## Stages

The capture carries each stage's output, so `crates/lemmata-glyphs/tests/arcs.rs` can pin a divergence to the stage that produced it:

- `expansions_match` compares the expanded sides, knot positions, types and importance. Exact.
- `raw_solve_arcs_match` compares the spiro solve on those knots: sampled points, arc length and bend. Exact.
- `tree_arcs_match` compares the tree's arcs, before overlap resolution.
- `resolved_arcs_are_close` compares the arcs after overlap resolution, which is where the two boolean engines part.

The first two are exact, so a regression in the constructors fails loudly instead of hiding inside a tolerance. The last two are measurements: the comparison is coverage-aware, so a seam that two engines split differently does not count as drift, but a region that differs does.

## The open gap

The geometry tree and overlap resolution now match upstream exactly: `tree_arcs_match` and `resolved_arcs_are_close` report 59 of 59 glyphs within 0.25 units, worst 0.000.

What is left is the last step, fairization into quadratic contours. Our quadification emits fewer points than Iosevka's on five glyphs, `a`, `e`, `g`, `s` and `three.lnum` (76 against 79 points on `three.lnum`), which puts the contour comparison at a worst of 4.7 units. The coverage comparison, which samples a grid and compares inside against outside, stays at 0.54% of a glyph's area, so the regions agree and this is a placement and segmentation difference.

The reason it survived every earlier test is coverage. `tests/fairize.rs` and `tests/quadify.rs` each check one half of the step on their own small corpora, and both are exact; nothing checked the composition `quadify_fair(fairize(..))` on real glyph arcs. The fix is the same as before: capture the fairized shape as a stage, compare it on its own, then compare the quadified contours against it.
