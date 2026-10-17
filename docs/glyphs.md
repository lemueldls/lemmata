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

Overlap resolution is the one step that is not exact. Iosevka resolves overlaps with typo-geom on an integer grid, and `lemmata-geom` uses linesweeper on exact curves. Where two shapes cross at a shallow angle the two boundaries part, and near-coincident edges can leave contours joined in one engine and split in the other.

That accounted for the visible holes at first, but the larger part of the gap was a bug in the constructor. `SpiroExpander::expand` built its four knot lists as `Corner`, then filled in the type only for important knots. Since most knots in a real skeleton are unimportant, they reached the solver as corners, their spans came out straight, and the cubic fit had nothing to curve. Fixing it to carry every knot's type and importance moved the raw solve from a 7.7 unit interior divergence to exact, the tree arcs from 23 of 59 glyphs over the drift target to 2, and the coverage mismatch from 2.6% of a glyph's area to 0.54%.

What is left is the clipper on three glyphs, `four.lnum`, `w` and `numberSign`, all built from nested intersections and differences: 3 of 59 over the 1 unit boundary target, worst 6.8 units. Closing that means matching typo-geom's clipper more closely.
