# The plan

What a caller chooses about the font, and how far that reaches today.

## What exists

`lemmata-plan` carries the values the writer puts in `head`, `hhea` and `OS/2`: the em size, the vertical metrics, the x-height and cap height, the underline and strikeout positions, the weight class, the width in font units, the width class and the italic angle. `Plan::default` is Iosevka's vertical metrics at 1000 units per em and its 600 width grade, so an empty plan is the normal face the project ships. `tests/default.rs` pins that, along with a changed field reaching the tables.

`width` is the drawing width, which for normal spacing is also the advance. Iosevka grades it at 416, 456, 500, 547, 600, 657 and 720 and blends between them, so any value in that span is a coherent frame: 580 is a wider normal rather than an error. The OS/2 class follows from the nearest grade unless a plan sets `widthClass` on its own. A plan written with the old class scale, `"width": 6`, is rejected rather than read as six units.

It carries the character variants as well, in the shape an Iosevka plan uses: a group and a kebab-case variant name, either directly under `variants` or under `variants.design`. `a = "double-storey-serifed"` is the glyph `a.doubleStoreySerifed`, and `lemmata_build::character` moves that drawing onto the glyph the group's codepoints are on. The codepoint stays where it was rather than moving to the variant's glyph, because every feature the font writes names glyphs: `dtls` replaces `i`, texture healing replaces `a` and `m`, and `ssty` names the math letters. A cmap pointing at `a.doubleStoreySerifed` would leave all of them matching nothing, and Iosevka makes the same move for the same reason.

A choice the capture does not hold is ignored rather than fatal, and the group's other drawings are dropped, so the font carries one drawing per group rather than one plus the alternatives it did not choose. `lemmata_web::variants_of` lists what a capture holds for a group, which is what the page's picker offers, so the list follows the glyphs rather than a table written into the page.

The boundary format is JSON, because the page sends an object and `serde_json` is already in the tree. A plan *file* is meant to be TOML with Iosevka's keys, and the struct is the part that survives that change, but the format is not settled: `facet-toml 0.50.0-rc.7` reads a document into a derived type and reports an error by field path, and it neither publishes a schema nor carries a span, so an error cannot be located by line. The pins that were meant for it are gone from the manifest, since nothing depended on them.

The CLI takes `--plan <file>` and the page has a text area holding the plan. Both hand it to the same `lemmata_plan::from_json`, and any key either omits takes its default, so a control only has to send what it changed.

## Spacing

`spacing` takes `normal` or `quasi-proportional`. It says which capture the caller supplies and whether the plan's width is the advance: a normal face has one cell, while a quasi-proportional capture carries an advance per letter class and the plan keeps those. The spacing is a property of the drawings: Iosevka draws each glyph into a frame whose width comes from its letter class, so the two are separate builds.

Measured against the monospace build of the same set:

| glyph | advance | ink | outline |
| --- | --- | --- | --- |
| `i`, `l` | 600 → 400 | 402 → 296 | redrawn |
| `m` | 600 → 900 | 470 → 724 | redrawn |
| `M`, `W` | 600 → 800 | — | redrawn |
| `x`, `a` | 600 → 600 | unchanged | byte-identical |
| `period` | 600 → 400 | 156 → 156 | advance only |
| `space` | 600 → 350 | — | — |

The class ratios come from `params/parameters.toml`'s `spacing-quasi-proportional` table: `advanceScaleMM` 1.5, `M` 1.333, `T` 1.167, `F` 0.833, `I` 0.667, `II` 0.5, `Sp` 0.583. A plan chooses the preset and not the ratios. `metricOverride` is the only way to move one, and its documented table lists just `advanceScaleSp` of the seven: setting `advanceScaleSp = 1.0` moves `space` from 350 to 600, and setting `advanceScaleM` or `advanceScaleI` does nothing at all, silently, whether it is written as a plan key or under `metricOverride`.

So a page offers the two presets and cannot offer a slider. `web/public/glyphs-quasi.bin` is the same set drawn at the second preset, and `lemmata-web`'s test asserts the classes: `i` at 400 and `m` at 900 against 600 for both.

## What it does not reach

The capture. `lemmata-ptl::param` derives Iosevka's parameter set from the plan's weight, width and slope, and the ported definitions draw from it. A glyph still in the capture was recorded at one weight and one width, so a plan changes its advance but not its drawing. At another width the build moves the captured outline half the difference, which keeps the face spaced without pretending the glyph was redrawn. A quasi-proportional face keeps the capture's per-class advances, because the plan has no ratios of its own to give.

So the way to make the font follow the plan is to move glyphs out of the capture. Each definition added to `lemmata-glyphs` is one more that the plan reaches.
