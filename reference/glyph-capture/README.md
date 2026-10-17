# Glyph capture

Records, for a set of glyphs, the geometry tree a pinned Iosevka build produced and the contours it converted them to. `lemmata-glyphs` reads the result and replays the tree through `lemmata-geom`.

Unlike the other oracles this one patches the build, because the glyph store is built inside `buildFont` and there is no exported entry point that hands out per-glyph geometry.

Apply two edits to a scratch copy of `vendor/Iosevka`:

1. In `packages/font/src/otd-conv/geometry.mjs`, at the top of `convertGlyphGeometryToProxy`, add:

   ```js
   lemmataDumpGlyphs(glyphStore, para);
   ```

2. Append `dump.mjs` to the end of the same file.

Then build with the capture enabled:

```
IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build
LEMMATA_GLYPH_DUMP=/tmp/opencode/glyphs.bin
LEMMATA_GLYPH_WIDTH=500
LEMMATA_GLYPH_CODEPOINTS=32,48,49,97,98,99
npm run build -- ttf-unhinted::RefA
```

`LEMMATA_GLYPH_DUMP` is the binary output path, `LEMMATA_GLYPH_SET` selects by glyph name and `LEMMATA_GLYPH_CODEPOINTS` by codepoint. `LEMMATA_GLYPH_WIDTH` pins the advance width, because a plan can emit more than one font and the dump otherwise lands on whichever is written last.

A cached build does not re-run the dump. Changing the plan back and forth to capture one slope at a time is exactly the case that hits this: the second build finds its output already in `dist/` and skips the work the dump is hooked into, so the capture keeps its old contents and looks like a build that succeeded. Remove `dist/<plan>` to force it.

The width pin is not enough on its own. A plan with two slopes at the same width, an upright and an italic, emits two fonts that both pass the guard, and the dump lands on the last one written. Capturing a particular slope means building a plan that holds only that slope: it is worth doing, because the failure is silent and produces a capture that looks correct until you compare it with the other one.

The capture writes `<path>` (the trees), `<path>.json` (the contours Iosevka converted them to), `<path>.tree-arcs.json` (the tree's arcs, before overlap resolution), `<path>.arcs.json` (after it), `<path>.expansions.json` (each di-spiro node's expanded sides and the raw arcs the solver produced from them) and `<path>.fairized.json` (the fairized shape the quadifier consumes). Copy them to `target/` for `cargo test -p lemmata-glyphs` and `cargo test -p lemmata-cli`.

## Binary layout

Little-endian throughout. A `u32` glyph count, then per glyph:

- name: `u32` length and UTF-8 bytes
- advance width: `f64`
- unicodes: `u32` count, then `u32` each
- glyph gizmo: six `f64` (`xx, xy, yx, yy, tx, ty`)
- geometry node

A node starts with a `u8` tag:

| Tag | Kind | Fields |
| --- | --- | --- |
| 0 | invalid | none |
| 1 | contours | `u32` contour count; per contour `u32` arc count; per arc four `f64` pairs |
| 2 | spiro | gizmo, `u8` closed, knots |
| 3 | di-spiro | gizmo, `f64` contrast, `u8` closed, bi-knots |
| 4 | spiro pen | gizmo, `u8` closed, profile points, pen knots |
| 5 | stroke | gizmo, `f64` radius, `f64` contrast, `u8` inside, node |
| 6 | remove holes | gizmo, node |
| 7 | simplify | gizmo, node |
| 8 | combine | `u32` part count, nodes |
| 9 | boolean | operator string, `u32` operand count, nodes |
| 10 | transformed | gizmo, node |
| 11 | tagged | node |
| 12 | radical | node |
| 13 | reference | node |

A knot is a type string, its `f64` coordinates, and flags; see `dump.mjs` for the exact fields.
