# Glyph capture

Records, for a set of glyphs, the geometry tree a pinned Iosevka build produced and the contours it converted them to. `lemmata-glyphs` reads the result and replays the tree through `lemmata-geom`.

Unlike the other oracles this one patches the build, because the glyph store is built inside `buildFont` and there is no exported entry point that hands out per-glyph geometry.

Apply two edits to a scratch copy of `vendor/Iosevka`:

1. In `packages/font/src/otd-conv/geometry.mjs`, at the top of `convertGlyphGeometryToProxy`, add:

   ```js
   lemmataDumpGlyphs(glyphStore, para);
   ```

2. Append `dump.mjs` to the end of the same file.

Then build with the capture enabled. The set is the one the tests compare
against: every variant `lower-a.ptl` builds, plus the letters, digits and
punctuation the other suites walk. A capture missing one of them fails
`crates/lemmata-glyphs/tests/lower_a.rs` with its name.

```sh
IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build
LEMMATA_GLYPH_DUMP=/tmp/opencode/glyphs.bin
LEMMATA_GLYPH_WIDTH=500
LEMMATA_GLYPH_SET='a,a.crossing,a.doubleStoreyFlatBottomSerifed,\
a.doubleStoreyFlatBottomSerifless,\
a.doubleStoreyHookInwardSerifedFlatBottomSerifed,\
a.doubleStoreyHookInwardSerifedFlatBottomSerifless,\
a.doubleStoreyHookInwardSerifedSerifed,\
a.doubleStoreyHookInwardSerifedSerifless,\
a.doubleStoreyHookInwardSerifedTailed,\
a.doubleStoreyHookInwardSerifedToothlessCorner,\
a.doubleStoreyHookInwardSerifedToothlessRounded,a.doubleStoreySerifed,\
a.doubleStoreySerifless,a.doubleStoreyTailed,\
a.doubleStoreyToothlessCorner,a.doubleStoreyToothlessRounded,\
a.singleStoreyDoubleSerifed,a.singleStoreyEarlessCornerSerifed,\
a.singleStoreyEarlessCornerSerifless,\
a.singleStoreyEarlessCornerTailed,a.singleStoreyEarlessRoundedSerifed,\
a.singleStoreyEarlessRoundedSerifless,\
a.singleStoreyEarlessRoundedTailed,a.singleStoreyFlatTopDoubleSerifed,\
a.singleStoreyFlatTopSerifed,a.singleStoreyFlatTopSerifless,\
a.singleStoreyFlatTopTailed,a.singleStoreyFlatTopTailedSerifed,\
a.singleStoreySerifed,a.singleStoreySerifless,a.singleStoreyTailed,\
a.singleStoreyTailedSerifed,a.singleStoreyTopCutSerifed,\
a.singleStoreyTopCutSerifless,a.singleStoreyTopCutTailed,\
aCap.singleStoreyDoubleSerifed,aCap.singleStoreyEarlessCornerSerifed,\
aCap.singleStoreyEarlessCornerSerifless,\
aCap.singleStoreyEarlessCornerTailed,\
aCap.singleStoreyEarlessRoundedSerifed,\
aCap.singleStoreyEarlessRoundedSerifless,\
aCap.singleStoreyEarlessRoundedTailed,\
aCap.singleStoreyFlatTopDoubleSerifed,aCap.singleStoreyFlatTopSerifed,\
aCap.singleStoreyFlatTopSerifless,aCap.singleStoreyFlatTopTailed,\
aCap.singleStoreyFlatTopTailedSerifed,aCap.singleStoreySerifed,\
aCap.singleStoreySerifless,aCap.singleStoreyTailed,\
aCap.singleStoreyTailedSerifed,aCap.singleStoreyTopCutSerifed,\
aCap.singleStoreyTopCutSerifless,aCap.singleStoreyTopCutTailed,\
aHalf.doubleStoreyFlatBottomSerifed,\
aHalf.doubleStoreyFlatBottomSerifless,aHalf.doubleStoreySerifed,\
aHalf.doubleStoreySerifless,aHalf.doubleStoreyTailed,\
aHalf.doubleStoreyToothlessCorner,aHalf.doubleStoreyToothlessRounded,\
ampersand,asciiDoubleQuote,asciiSingleQuote,asterisk,at,b,c,colon,\
comma,d,dollar,e,eight.lnum,equal,exclam,f,five.lnum,four.lnum,g,\
greater,h,hyphen,i,j,k,l,less,m,n,nine.lnum,numberSign,o,one.lnum,p,\
parenLeft,parenRight,percent,period,plus,q,question,r,s,semicolon,\
seven.lnum,six.lnum,slash,space,t,three.lnum,two.lnum,u,v,w,x,y,z,\
zero.lnum'
npm run build -- ttf-unhinted::RefA
```

A second capture of the same set at `spacing = "quasi-proportional"` is what
gives the site a proportional face. The spacing is a property of the drawings,
so it is a second build rather than a second set of advances:
`web/public/glyphs-quasi.bin` is the site's set drawn that way, and
`docs/plan.md` records what changes.

The plan must hold only the upright slope. `reference/private-build-plans.toml`
gives RefA an italic slope too, both are 500 units wide, and both pass the width
guard, so the dump lands on whichever the build writes last. Build a plan with
the italic slope removed.

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
