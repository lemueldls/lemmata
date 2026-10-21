# The MATH table

Iosevka ships none. The reference builds carry fourteen tables (`DSIG GDEF GPOS GSUB OS/2 cmap glyf head hhea hmtx loca maxp name post`) and MATH is not among them, so the math support the plan asks for is new work rather than something to inherit. That also means there are no stretchy size variants to take: a font with no MATH table has no reason to draw three widths of a parenthesis.

`lemmata-build` writes the table. `write-fonts` has a MATH writer and `read-fonts` has a reader, so neither side needed hand-written table code.

## What is written

`crates/lemmata-build/src/math.rs` writes `MathConstants`. It is the first of the table's three parts and the only one that needs no math glyphs, which is what made it a clean starting point.

The specification treats a constant of zero as "not specified, use the renderer's default", and a renderer that reads it literally then places scripts on the baseline and never grows a display operator. Leaving one at zero is not neutral, so every constant the specification describes is written, from two sources.

Most of them the specification gives a formula for, in multiples of the rule thickness: a fraction's numerator sits one rule above its bar and three in display style, a stack's gap is three rules and seven, a radical's gap is one and a quarter. The rest are design values, and those come from New Computer Modern Math. It is set at the same 1000 units per em, so its numbers are ours as they stand. Scaling them by the ratio of the two x-heights is the mistake to avoid: this font's x-height is much the larger, and scaling made every fraction taller, which made every delimiter stretch further to enclose one.

| constant | from |
| --- | --- |
| the shifts, gaps and limit distances | New Computer Modern Math, at the same 1000 units per em |
| the rule thicknesses and the gaps around bars, fractions and stacks | the underline thickness, which the specification measures them in |
| `accent_base_height` | the x-height plus overshoot, which is the one the specification defines against this font |
| `flattened_accent_base_height` | the cap height |
| `delimited_sub_formula_min_height`, `display_operator_min_height` | 1.3 em |

The values are floors rather than the last word. A renderer also clears the base it places a script against, and that rule reads this font's own x-height, so a shift that is too small here is still raised there.

Two defects followed from leaving those at zero, both visible in a Typst render of the corpus: superscripts sat on the baseline because `superscriptShiftUp` was zero, and a display summation and integral stayed at text size because `displayOperatorMinHeight` was. A renderer that substitutes its own default for a zero hides the difference; Typst reads the constant as written.

## Per-glyph information

`MathGlyphInfo` carries italic corrections and top accent attachments, each as a coverage table over glyphs. Both are written for every letter the font carries, 1076 of them, rather than for the math alphanumerics alone. A renderer sets a plain Latin or Greek letter in math as readily as it reaches for U+1D465, so covering only that block leaves the letters it actually uses without corrections.

An italic correction is how far a slanted letter leans, which a renderer needs to place what follows it. The port measures it rather than taking it from a design: the ink is split at its midline, and the correction is the upper half's centre against the lower half's, clamped at zero and at a quarter of the advance. An upright style measures near zero and an italic one does not, which is what makes the measure testable.

The clamp matters. An italic `l` has a tail that reaches right at the baseline, so its lower half is wide and a raw measurement would report a lean larger than a renderer could use.

A top accent attachment is the horizontal midpoint of the upper ink, which is where an accent is centred over a letter.

A third coverage names the glyphs a renderer positions against their ink box rather than the baseline. Those are the stretchy glyphs and the sizes derived from them, 120 in all. Without it, a superscript beside a parenthesis grown to enclose a fraction is aligned with the parenthesis's own baseline instead of the line the rest of the equation sits on.

The table also carries the corner kerns. A renderer places a script against the letter's advance, but the ink can leave that box: an italic stem leans past the right edge, and a hook can reach left of the origin. `MathKernInfo` records the overhang at the heights where it changes, so a renderer can move the script with the ink instead of the cell. The derivation samples the outline six times between the baseline and the top of the ink and keeps a band only where the edge moves. Most glyphs stay inside their box and get no table at all. Fifteen of the 51 italic letters in the capture have a top-right overhang worth recording, and the upright block has none.

Only the top corners are written. A descender can overhang below the baseline, and a subscript at the bottom corner would use that the way a superscript uses the top, but the bottom records are left null until a case asks for them.

These are proxies, like the constants. A design pass against a reference math font is what would replace them.

## Verification

Two test files, because the table has two halves and they read different captures.

`crates/lemmata-cli/tests/build.rs` builds the font, reopens it with `read-fonts`, and checks that the constants survive the write and the read: the profile's two values, the four derived from measurements, one left at zero to confirm it stays zero, and the mandatory-nonzero set. That is `r[math.table]`.

`crates/lemmata-cli/tests/math.rs` builds from the math capture and checks that every math letter has a correction and an attachment, that both are inside the glyph, and that the italic block leans more than the upright block. That last check is the one that matters: a measure that cannot tell the two apart is not measuring a slant. The italic block averages 77 and the upright block 8.

## The math capture

It is separate from the coverage capture, because it holds a different set. The coverage one is 72 glyphs of Latin letters, digits and punctuation and the outline tests are calibrated against it, so widening it would perturb a calibrated gate. This one holds the math alphanumerics and the operators.

Regenerate it by naming the set it holds and adding the combining marks:

```
LEMMATA_GLYPH_DUMP=target/math.bin LEMMATA_GLYPH_WIDTH=500 \
LEMMATA_GLYPH_SET=$(python3 -c "import json; print(','.join(g['name'] for g in
  json.load(open('target/math.bin.json'))))") \
LEMMATA_GLYPH_CODEPOINTS=$(python3 -c "print(','.join(str(c) for c in range(0x300, 0x370)))") \
npm run build -- ttf-unhinted::RefA
```

It lands 1709 glyphs: 904 math alphanumerics, 256 operators, 56 Greek letters,
58 Latin ones, 10 digits, 308 other symbols and 112 combining marks.

**The plan must hold one slope, and it has to be the upright one.** With both,
both are 500 units wide and the dump lands on whichever the build writes last,
which is the italic, and a slanted delimiter's band cannot repeat: its stem
wanders sideways over the band, so `assembly` judges every one of them unable to
stack and the font ships without assemblies. Two of the math tests fail on that,
and the message names the glyphs rather than the slope.

The alphanumerics are the whole block, and Iosevka draws every style in it except one. `r[math.alphabets]` names eight: bold, italic, bold italic, script, fraktur, double-struck, sans-serif and monospace. The pinned build has seven of them. **Script is absent**, both the plain and the bold pair, 104 codepoints with no glyphs behind them.

That gap is in the glyph source rather than in the table, so the capture cannot close it and neither can the table. The test pins it: it asserts that script is the only style missing, so the day the source grows script letters the assertion flips and the requirement can be closed.

The ranges are not full even where the styles exist, and that is Unicode rather than Iosevka: italic is 51 of 52 codepoints because U+1D455 is reserved, and fraktur and double-struck have five and seven reserved holes respectively.

## The italic face

Math variables are italic. A renderer picks the italic face of a family by the style bits, so a family with only a Regular cannot set an equation: Typst falls back for every variable, which is nearly every equation. This was measured rather than assumed. With the font set on `math.equation`, `1 + 2` and `2/3` render from ours and `x` does not; `upright(x)` renders from ours and `italic(x)` does not.

`Metrics::italic_angle` is the one place the style is decided. A non-zero angle sets the italic bit in `head.macStyle` and in `OS/2.fsSelection`, and writes the angle into `post`. The specification has `fsSelection.REGULAR` mean neither bold nor italic, so an italic face sets `ITALIC` alone.

The outlines come from the capture, so an italic capture is a build of the same plan at a slope. `reference/private-build-plans.toml` carries one at 15 degrees, which is upstream's own slanted preset, and `RefA-Italic.ttf` is what it produces.

Two tests cover it. One builds the same glyphs at two angles and checks the bits, the name table and the angle agree, since a renderer reads the bits and a designer reads the name. The other compares the slanted capture against the upright one and checks the ink actually leans: 26 letters, 114 units more on average. The captures hold different sets in a different order, so that comparison is by codepoint rather than by position.

With both faces built, Typst sets the equation corpus from the family with no fallback to its bundled math font. It embeds the Regular rather than the Italic, which is not what the style bits alone would predict. The likely reason is that the Regular carries the math italic alphanumerics, U+1D465 for an italic `x`, so a renderer that reaches for those never needs the slanted face. That is worth knowing rather than assuming: the faces are paired by the typographic family and subfamily records, nameID 16 and 17, and the corpus renders correctly either way.

## The corpus and the harness

`corpus/equations.typ` is the equation corpus `r[test.corpus]` asks for. It holds every construct a math font has to serve: a variable, arithmetic, a script, a fraction, a radical, a large operator with limits, a stretchy delimiter pair, a limit and Greek letters.

`crates/lemmata-cli/tests/typst.rs` compiles it against the built font and reads the PDF back. A renderer that cannot serve a glyph or a table falls back to a font of its own, and the PDF says so in its font names, so the test asserts that every embedded font is ours. That is what `r[math.typst]` means from the outside: no missing glyph and no missing table, observed rather than assumed.

Two details make it work, and both cost time to find. The font needs `--font-path`, and the font has to be set on `math.equation` rather than on `text`, because a text font does not reach math mode. A cache directory of the harness's own keeps a stale font list from deciding the result.

It skips when Typst is not installed, like the tests that skip without a capture. It found the font wanting the first time it ran: the corpus needs the Greek letters and `times`, which the capture did not have.

## Stretchy glyphs

`MathVariants` names the sixteen codepoints a renderer may stretch, and each construction carries both things a renderer needs: the sizes it picks from and the assembly it builds when none is tall enough.

The sizes are derived rather than drawn, and the two families grow differently. A delimiter stretches: a parenthesis is a curve that gets taller while the stroke keeps its width, so scaling y alone is what a taller one looks like. A large operator is not stretched but set at a larger size, so its display form is a scaled drawing, wider as well as taller, and its advance grows with it or it would run into its neighbour. Deriving a taller sigma without widening it is what left it looking thin at display size.

A delimiter gets two, at 1.5 and 2 times its own size. The summation, the product and the integral get three, because a display operator is set larger than a stretched delimiter and the renderer can only pick from what it is given: at twice its size a summation is no taller than the integral beside it, which is what left the integral looking no bigger than its text size. `displayOperatorMinHeight` sits at 1.7 em, above the second scale, so a display operator picks the third.

Growing in both axes is right for the summation and the product, whose display forms are scaled drawings, but not by the same factor in each direction. A scale multiplies the stroke, and the weight of a vertical stroke is its width, so growing width by the height's factor made the summation's ink as heavy as the text was tall. Width grows by `WIDEN`, 1.25, while height takes the full scale. It is wrong for the integral: its limits sit in the gaps of the shape, so a wider drawing pushes them out and it stops reading as one operator. The integral stretches like a delimiter and takes the larger size in y alone.

A grown operator's cell grows with it, and the drawing moves by half the difference so that it stays centred in the new cell. Scaled about the old cell's centre alone its ink starts left of the new origin, which puts the operator off-centre in its line and its limits off-centre over it.

The test checks that each size is taller than its base, that a large operator is wider too, that a delimiter's width is unchanged, and that every derived size is named by the table and covered as an extended shape.

An assembly is how a renderer goes taller than that: it stacks a bottom piece, repeats a middle one, and caps it with a top piece. The pieces are the glyph cut into three horizontal bands, so the joins are the cuts and the three tile the original outline. They are derived and encoded in the same private-use block as the sizes. The test stacks the parts the way a renderer does and checks that each one's ink spans its declared advance, that consecutive parts cross, that no join overlaps by more than the stroke width the table names as its minimum, and that the middle piece repeats without stepping.

The extender is a straight bar, not the band itself. The band decides where to cut and how wide the bar is; the bar is rebuilt from that ink, so a repeat cannot carry the band's residual curve. A repeated curve is what made the parenthesis look scalloped, one step per repeat. With the bar the seam is zero on every assembly.

Every connector is that same minimum, so the pieces have no slack: a renderer reaches a target size by repeating the extender rather than by sliding the joints. A drawn extender with a long straight bar is what would give it that slack.

The band is what makes a piece repeatable, so the derivation measures it: it slices the band at three heights and compares the ink's horizontal extent across them. Measuring only the two cut edges is not enough, and that is the trap. A brace's middle holds its point, so both ends sit on the stem and agree while the ink bulges out and back between them.

The middle is tried first and taken as soon as it is straight enough, at half the stroke width. Moving the band up is only for a glyph whose middle cannot repeat, and it is not free: the piece below the band then holds more of the glyph than the piece above, so a brace's point lands at the bottom of the assembly instead of in the middle. Trying the straightest band first was that mistake, and it unbalanced every glyph it touched.

A brace needs a shape of its own. Its point cannot repeat, so it gets five parts: the lower hook, a lower bar, the point, an upper bar and the upper hook. Both bars carry the extender flag, which is how a renderer grows the brace on both sides of the point. Noto Sans Math and DejaVu Math TeX Gyre write the same five-part construction. The bars run from the hooks to the point at the stem's own width, so the point stays centred however tall the brace grows.

Four glyphs have no band that repeats and the test names them rather than giving them an assembly that stacks a feature: a summation's middle holds its crossbar, an angle bracket's holds its vertex, and a radical's is a diagonal. They need a drawn extender. The other twelve are the parentheses, brackets, braces, ceilings, floors, integral and product.

The radical also needed a flat top. Iosevka's `sqrt` is a check mark whose long stroke ends in a point, with no horizontal bar, and a renderer draws the vinculum itself starting at the glyph's advance. The thin rule met the stroke at one point and left a notch. The build adds a bar of the rule's own thickness from the advance to the ink's right edge, so the rule starts on an edge instead of a point.

The delimiter bases are ported in `crates/lemmata-glyphs/src/symbol/punctuation/brackets.rs`: parentheses, brackets, braces, angle brackets, floors and ceilings from `brackets.ptl`, gated against the capture. The drawn pieces at U+239B to U+23AD are still not built; the assembly derives its own bars for now.

`docs/divergences.md` is the ledger of every glyph the font has that upstream does not, and every upstream outline the build changes.

## Horizontal constructions

A horizontal arrow grows the other way, and `MathVariants` carries a second coverage and construction list for it. The same record serves both: two scales, here 2 and 3 times the base, and an assembly for a length beyond them.

The sizes scale x alone, about the left edge, and the advance grows with the drawing. Scaling both axes would make the head as tall as the longer cell, which is not what a long arrow looks like. Scaling in x stretches the head as well, and that is the compromise of deriving a variant rather than drawing one. The assembly avoids it for the lengths a renderer builds: it cuts three vertical columns, repeats the middle one, and leaves the head and the tail alone. The middle is a column of the shaft, so its ink holds the same height across the width, and the band check from the vertical assembly decides whether it repeats.

Twenty-three arrows are covered: implication, maps-to, the harpoons, and the dashed and double forms in the capture. Twenty-one build an assembly. The double-headed right arrow's heads meet in the middle and the negated arrow's stroke crosses the shaft, so neither has a column that repeats, and the test names them.

The long arrows at U+27F5 to U+27FF are drawn upstream but are not in the capture set. The construction code covers whatever the capture holds, so the font has no glyph to build those from until the set grows.

## The browser

`corpus/equations.mathml` is the same twelve equations as MathML. `app/pages/math.vue` sets MathML in the face built from the math capture. The capture lives in `web/fixtures/math.bin.gz` because it is 2.4 MB raw and 400 KB compressed, and nothing in the site serves it: `pnpm --dir web font:write` turns it into the 80 KB WOFF2 the page is set in, and the page only waits for that font rather than building it. The corpus is editable, so the page is somewhere to try an equation rather than a fixed sample. `harness/browser/math.spec.mjs` runs that through a real browser and checks the family is what rendered: a renderer that cannot serve the corpus falls back, and the fallback changes the measurement.

That is `r[math.mathml]`'s rendering half. The page's stylesheet must not set `display` on a `math` element. `display: block` replaces the `block math` the user agent stylesheet gives `math[display="block"]`, and every token inside is then laid out on a line of its own with no `mrow` to run. The page did exactly that until a screenshot of it showed the equations stacked vertically. The `display` attribute on each `math` element is what puts it on its own line.

The harness does not show stretching. Chromium lays the corpus out and sets it in our font, but the delimiter boxes stay at the base glyph's size however tall the content between them is, and a large operator is not enlarged either. A second page built with a known-good math font, New Computer Modern Math, measured the same, but that font never loaded in the harness, so the measurement does not settle whether the limit is the browser or our MATH table. What covers the pieces is the geometric test in `crates/lemmata-cli/tests/math.rs`, which is what `r[math.stretch]` asks for, and Typst's `\left` and `\right` exercise an assembly end to end.

The browser shell needed `PLAYWRIGHT_BROWSERS_PATH`. Playwright looks in `~/.cache/ms-playwright` by default and the shell provides the engines from the store, so without it the suite fails with "Executable doesn't exist" and suggests downloading a browser that is already there.

## The layout tags

The specification names three layout tags for math, and all three are written.

`dtls`, the dotless forms, is a single substitution under the `math` script: `i`
and `j` draw their dot as a contour of its own, so the form is the same glyph with
that contour removed. `crates/lemmata-layout` builds the GSUB around it, since
`write-fonts` has the tables but no builders for the script, feature and lookup
lists.

What separates a tittle from an accent is worth recording, because an accent is
also a small mark above the letter. A dot is filled and as tall as it is wide,
while a grave is long and thin and a circumflex is hollow against its box. Without
that test the derivation produced a dotless `aGrave`, a dotless `aCircumflex` and
a dotless `Xi`.

`ssty`, the script-size alternates, is the second. It is an alternate
substitution rather than a single one, and that is the whole of what the feature
does: it hands the renderer a list of forms per glyph and the renderer picks one
by index, so the first is the script size and the second the scriptscript size. A
single substitution would give a renderer one form and no way to ask for the
other.

Two reference math fonts were read for the shape of that list. Latin Modern Math
covers 516 glyphs with two alternates each, STIX Two Math 529 with two or four,
and both cover every letter and digit the font has. Both widen the drawing and
hold or barely raise the height: Latin Modern's script alternate is 7 to 11
percent wider with the height unchanged, STIX's 13 percent wider and 8 percent
taller. Wider rather than taller is the shared intent, since a taller glyph
collides with the line above, so the derivation holds the height and widens the
drawing and the cell together, by 10 percent for script and 25 for scriptscript.

What it does not do is thicken the strokes independently of the counters. Latin
Modern's alternate holds the counters at their size and thickens the stems by
about 14 percent, which is a redraw rather than a transform of the outline. A
horizontal scale widens both, which serves the same end by a different route.

Only a math font carries it. The feature is registered under the `math` script,
which a text run never selects, so deriving alternates for a text font's letters
puts glyphs in it that nothing can reach: the coverage capture's font went from
167 glyphs to 243 before that was noticed. The derivation takes the math
alphanumeric block as the sign that a set is a math font's, and covers the plain
Latin, Greek and digits only when it holds one.

`flac`, the flattened accents, is the third. It is a single substitution, unlike
`ssty`: the renderer asks for `flac` and gets the flat form, with no choice to
make.

The reference is STIX Two Math, which maps ten combining accents to a `mathcap`
form. Read off its outlines, the flat form is the accent made shorter and wider:
the grave and the acute lose the most, 0.84 of their height and 1.27 of their
width, and the macron the least, 0.96 and 1.01. So the derivation takes one
factor in the middle of that range rather than a table per accent, scaling the
height about the accent's own bottom edge, which is the edge it attaches by, so a
flat accent sits where the ordinary one did.

Only the accents above the base are flattened, and there are 55 of them. An
accent below hangs from the other edge, where a tall base is not the problem. The
names say which is which: the glyph source calls them `graveAbove` and
`graveBelow`.

## Texture healing

The same crate carries the other feature, and it is not a math one. A monospace grid forces every letter into the same advance, so a narrow letter carries whitespace it does not need while a wide one is squeezed. Texture healing substitutes a form that gives that space to the neighbour that wants it, and the whole trick is that the advance does not change: only the drawing moves inside the cell.

Two roles and a third for the marks. A letter gives space when its ink leaves much of the cell empty and takes it when the ink nearly fills the cell; the classification is a rank rather than a threshold, because this font fills its cells far more evenly than the design texture healing was invented for. A mark that leaves more room than the narrowest giving letter stands still, because a period that moved would stop reading as a period, and the letter beside it extends instead.

Every letter carries six forms, which is upstream's set: `ExtL`, `ExtR`, `ExtLR`, `ShrL`, `ShrR` and `ShrLR`, at `1 ± 1/12` of the advance. A giver shrinks the edge that faces a taker and the taker extends into what it gave up, so the pair closes by the same amount on the side between them. Six passes run in order under `TXTR`, the tag upstream assigns to this feature: a giver between two takers shrinks both sides, a giver before or after a taker shrinks one, then a taker between two shrunk givers extends both, and a taker beside a shrunk giver extends one. The shrink is the seed: the extension rules match the forms the shrink left behind, which is what lets the chain fire from the base glyphs. A taker beside a mark extends toward it, because the mark itself must not move.

Four things had to be right that no other check sees. The coverage builder takes glyphs in order and does not sort them. Format 2 pairs a coverage with its replacements in order, so the coverage cannot be sorted on its own. The script and feature lists must be in tag order, because a shaper binary searches them. And format 3 takes one coverage per position, each naming every glyph that fits it, rather than one coverage per glyph. All four pass `ots-sanitize` and a read-back; only a shaper notices.

## What is deferred

Two things went wrong while deriving the glyphs and both are worth keeping.

The derived glyphs have to be encoded, at U+F000 and up. They are not characters, so nothing types them, but a subsetter keeps what is reachable and drops what is not: a variant named only by the MATH table disappears and leaves the table pointing at nothing. Iosevka encodes its own derived texture variants in the same block.

They also have to join the glyph set before anything counts it. Appending them later leaves them past `maxp.num_glyphs`, where they are ignored until something references one and the reference is out of range. That was the bug: `ots-sanitize` said `bad glyph ID: 1593`, one past the count, and a renderer only said `malformed font`. A renderer shrugs where the sanitizer names the number. The writer and the caller that reports the count now read one function, so a derived glyph cannot be written without being counted.

The script style `r[math.alphabets]` names is still a gap in the glyph source, and it is the last thing `r[math.script-style]` waits on: its other half is the `ssty` and `flac` features, and both are written.

The bottom corner kerns are not derived. A descender can overhang below the baseline, and a subscript at the bottom corner would use that the way a superscript uses the top, but the table leaves those records null. The top corners are where the scripts of an equation sit.

The horizontal braces at U+23DE and U+23DF are not in the source at all, so `\overbrace` needs a drawing before a construction can reach it. The long arrows are in the source but not in the capture, which is a set to widen rather than a shape to draw.

The radical is still a scaled check mark with a flat top. A proper display radical has a vertical stem beside the content, as the reference fonts draw it; no such shape exists upstream, so it would be a new drawing rather than a port.
