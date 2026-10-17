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

## Verification

`crates/lemmata-cli/tests/build.rs` builds the font, reopens it with `read-fonts`, and checks that the constants survive the write and the read: the profile's two values, the four derived from measurements, one left at zero to confirm it stays zero, and the mandatory-nonzero set. That is `r[math.table]`.

## What is deferred

`MathGlyphInfo` holds italic corrections, top accent attachments and math kerning, each as a coverage table over glyphs. It needs math glyphs in the font to cover, and the capture holds none: its 72 glyphs are Latin letters, digits and punctuation.

`MathVariants` holds the size variants and the assembly recipes a stretchy delimiter is built from. It needs those variants to exist as glyphs.

Both are therefore blocked on glyphs rather than on table code, and the next slice is to widen the capture to the operators and delimiters so the table has something to describe. Iosevka's math symbols live in `packages/font-glyphs/src/symbol/math/`, 243 glyphs across 17 files, and the capture selects by name or codepoint, so widening it is a build run rather than new code.
