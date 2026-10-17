# The MATH table

Iosevka ships none. The reference builds carry fourteen tables (`DSIG GDEF GPOS GSUB OS/2 cmap glyf head hhea hmtx loca maxp name post`) and MATH is not among them, so the math support the plan asks for is new work rather than something to inherit. That also means there are no stretchy size variants to take: a font with no MATH table has no reason to draw three widths of a parenthesis.

`lemmata-build` writes the table. `write-fonts` has a MATH writer and `read-fonts` has a reader, so neither side needed hand-written table code.

## What is written

`crates/lemmata-build/src/math.rs` writes `MathConstants`. It is the first of the table's three parts and the only one that needs no math glyphs, which is what made it a clean starting point.

The specification treats a constant of zero as "not specified, use the renderer's default". A profile therefore only names the constants it has an opinion about, and everything else stays zero. The profile sets the two script-scale percentages, which must be non-zero or a renderer scales by zero and draws nothing. The rest follow from the font's own measurements:

| constant | from |
| --- | --- |
| `axis_height` | half the x-height |
| `accent_base_height`, `flattened_accent_base_height` | the cap height |
| the four rule thicknesses | the underline thickness, standing in for a stroke width the plan does not carry yet |

These are plumbing, not typography. They make the table exist and parse. A design pass over a reference math font, Latin Modern Math or STIX Two Math, is what would replace them.

## Per-glyph information

`MathGlyphInfo` carries italic corrections and top accent attachments, each as a coverage table over glyphs. Both are written for the math alphanumerics, the 904 codepoints of U+1D400 to U+1D7FF.

An italic correction is how far a slanted letter leans, which a renderer needs to place what follows it. The port measures it rather than taking it from a design: the ink is split at its midline, and the correction is the upper half's centre against the lower half's, clamped at zero and at a quarter of the advance. An upright style measures near zero and an italic one does not, which is what makes the measure testable.

The clamp matters. An italic `l` has a tail that reaches right at the baseline, so its lower half is wide and a raw measurement would report a lean larger than a renderer could use.

A top accent attachment is the horizontal midpoint of the upper ink, which is where an accent is centred over a letter.

These are proxies, like the constants. A design pass against a reference math font is what would replace them.

## Verification

Two tests, because the table has two halves and they read different captures.

`crates/lemmata-cli/tests/build.rs` builds the font, reopens it with `read-fonts`, and checks that the constants survive the write and the read: the profile's two values, the four derived from measurements, one left at zero to confirm it stays zero, and the mandatory-nonzero set. That is `r[math.table]`.

`crates/lemmata-cli/tests/math.rs` builds from the math capture and checks that every math letter has a correction and an attachment, that both are inside the glyph, and that the italic block leans more than the upright block. That last check is the one that matters: a measure that cannot tell the two apart is not measuring a slant. The italic block averages 77 and the upright block 8.

## The math capture

It is separate from the coverage capture, because it holds a different set. The coverage one is 72 glyphs of Latin letters, digits and punctuation and the outline tests are calibrated against it, so widening it would perturb a calibrated gate. This one holds the math alphanumerics and the operators.

Regenerate it by adding the math codepoints to a build:

```
LEMMATA_GLYPH_DUMP=target/math.bin LEMMATA_GLYPH_WIDTH=500 \
LEMMATA_GLYPH_CODEPOINTS=$(python3 -c "print(','.join(str(c) for lo, hi in
  [(0x1D400, 0x1D7FF), (0x2200, 0x22FF), (0x2308, 0x230B), (0x27E8, 0x27EB)]
  for c in range(lo, hi + 1)))") \
npm run build -- ttf-unhinted::RefA
```

It lands 1168 glyphs: 904 alphanumerics and 256 operators.

The alphanumerics are the whole block, and Iosevka draws every style in it except one. `r[math.alphabets]` names eight: bold, italic, bold italic, script, fraktur, double-struck, sans-serif and monospace. The pinned build has seven of them. **Script is absent**, both the plain and the bold pair, 104 codepoints with no glyphs behind them.

That gap is in the glyph source rather than in the table, so the capture cannot close it and neither can the table. The test pins it: it asserts that script is the only style missing, so the day the source grows script letters the assertion flips and the requirement can be closed.

The ranges are not full even where the styles exist, and that is Unicode rather than Iosevka: italic is 51 of 52 codepoints because U+1D455 is reserved, and fraktur and double-struck have five and seven reserved holes respectively.

## The italic face

Math variables are italic. A renderer picks the italic face of a family by the style bits, so a family with only a Regular cannot set an equation: Typst falls back for every variable, which is nearly every equation. This was measured rather than assumed. With the font set on `math.equation`, `1 + 2` and `2/3` render from ours and `x` does not; `upright(x)` renders from ours and `italic(x)` does not.

`Metrics::italic_angle` is the one place the style is decided. A non-zero angle sets the italic bit in `head.macStyle` and in `OS/2.fsSelection`, and writes the angle into `post`. The specification has `fsSelection.REGULAR` mean neither bold nor italic, so an italic face sets `ITALIC` alone.

The outlines come from the capture, so an italic capture is a build of the same plan at a slope. `reference/private-build-plans.toml` carries one at 15 degrees, which is upstream's own slanted preset, and `RefA-Italic.ttf` is what it produces.

Two tests cover it. One builds the same glyphs at two angles and checks the bits, the name table and the angle agree, since a renderer reads the bits and a designer reads the name. The other compares the slanted capture against the upright one and checks the ink actually leans: 26 letters, 114 units more on average. The captures hold different sets in a different order, so that comparison is by codepoint rather than by position.

With both faces built, Typst sets the equation corpus from the family with no fallback to its bundled math font. It embeds the Regular rather than the Italic, which is not what the style bits alone would predict. The likely reason is that the Regular carries the math italic alphanumerics, U+1D465 for an italic `x`, so a renderer that reaches for those never needs the slanted face. That is worth knowing rather than assuming: the faces are paired by the typographic family and subfamily records, nameID 16 and 17, and the corpus renders correctly either way.

## What is deferred

`MathVariants` holds the size variants and the assembly recipes a stretchy delimiter is built from. Iosevka has no MATH table, so it has no size variants to inherit: a font with no MATH table has no reason to draw three widths of a parenthesis. Those are new glyph designs rather than table code, and they are the last of the table's three parts.

Math kerning and the extended shape coverage inside `MathGlyphInfo` are also unwritten. Neither is needed by a simple equation.
