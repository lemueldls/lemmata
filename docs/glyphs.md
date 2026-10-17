# Glyphs

Glyph outlines come from upstream's glyph definitions, replayed through `lemmata-geom`. That is the seam the glyph strategy chose: the definitions stay where upstream maintains them, the geometry is ours.

## Capture

A patched Iosevka build writes, for each selected glyph:

- its name and unicodes,
- its advance width,
- its glyph gizmo,
- the geometry tree its definitions built, as a small binary encoding,
- the arcs the tree resolves to, before quadification,
- and the quadratic contours Iosevka's own conversion produced.

Selection is by codepoint (`LEMMATA_GLYPH_CODEPOINTS`) or by name (`LEMMATA_GLYPH_SET`), the output path is `LEMMATA_GLYPH_DUMP`, and `LEMMATA_GLYPH_WIDTH` pins the width so a build that emits more than one font dumps only the one asked for. Without that pin the dump lands on whichever font the build writes last, which is how an earlier capture ended up holding the extended width while the reference font was the regular one. The reference contours land beside the binary as `<path>.json`, the arcs as `<path>.arcs.json`. The patched tree is disposable and lives outside the repository, like the other oracles.

`crates/lemmata-glyphs` decodes the capture into `lemmata_geom::Geometry` values. `tests/capture.rs` renders each glyph with `Geometry::Simplify { gizmo, geom }.to_contours()`, which is what the build does, and compares the result with the captured contours; it skips when no capture is present. The comparison is coverage-aware: a boundary point of one outline that lies outside the other counts as drift, while a point inside the other is a seam that two boolean engines can split differently while describing the same region.

## Building a font

`lemmata-build` lays out `head`, `hhea`, `maxp`, `OS/2`, `hmtx`, `cmap`, `name`, `post`, `glyf` and `loca` with `write-fonts`. `lemmata build` reads a capture and writes a TrueType file. The metrics default to Iosevka's: 1000 units per em, ascender 965, descender -215, line gap 70, x-height 520, cap height 735.

`crates/lemmata-cli/tests/build.rs` reads the font back with `read-fonts` and checks that every mapped codepoint has the captured advance and bounding box. It also compares the bounding boxes against `target/reference/RefA-Regular.ttf`: 58 codepoints, worst difference 1 unit.

## The open gap

Everything in the pipeline is exact against upstream to 1e-6 or better, except overlap resolution. Iosevka resolves overlaps with typo-geom on an integer grid, and `lemmata-geom` uses linesweeper on exact curves. Where two shapes cross at a shallow angle the two boundaries part, and near-coincident edges can leave contours joined in one engine and split in the other.

Contour orientation is part of that match, not a detail. typo-geom's clipper leaves outer contours clockwise and linesweeper leaves them counter-clockwise. Since `SimplifyGeometry` unions contours from several constructors under the non-zero rule, a boolean result wound the other way from its neighbours cancels over their overlap instead of merging, which leaves wedges missing at stroke joints. That is what made `k`, `w`, `0`, `5` and `$` render with holes. `boolean.rs` now reverses every contour it returns and `SpiroPenGeometry` unions its strips as upstream does.

What is left is boundary-level. The coverage test, which samples a grid and compares inside/outside, reports a worst mismatch of 2.6% of a glyph's area, on `parenLeft`. The boundary comparison reports a worst of 11.9 font units and puts 33 of the 59 captured glyphs above the 1 unit drift target. Those are thin bands along curves where the two engines place a boundary slightly differently, not missing regions. Closing them means matching typo-geom's clipper more closely.
