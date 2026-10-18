# The plan

What a caller chooses about the font, and how far that reaches today.

## What exists

`lemmata-plan` carries the values the writer puts in `head`, `hhea` and `OS/2`: the em size, the vertical metrics, the x-height and cap height, the underline and strikeout positions, the weight and width classes and the italic angle. `Plan::default` is Iosevka's vertical metrics at 1000 units per em, so an empty plan reproduces the font this project built before the plan existed. `tests/default.rs` pins that, along with a changed field reaching the tables.

It carries the character variants as well, in the shape an Iosevka plan uses: a group and a kebab-case variant name, either directly under `variants` or under `variants.design`. `a = "double-storey-serifed"` is the glyph `a.doubleStoreySerifed`, and `lemmata_build::character` moves that drawing onto the glyph the group's codepoints are on. The codepoint stays where it was rather than moving to the variant's glyph, because every feature the font writes names glyphs: `dtls` replaces `i`, texture healing replaces `a` and `m`, and `ssty` names the math letters. A cmap pointing at `a.doubleStoreySerifed` would leave all of them matching nothing, and Iosevka makes the same move for the same reason.

A choice the capture does not hold is ignored rather than fatal, and the group's other drawings are dropped, so the font carries one drawing per group rather than one plus the alternatives it did not choose. `lemmata_web::variants_of` lists what a capture holds for a group, which is what the page's picker offers, so the list follows the glyphs rather than a table written into the page.

The boundary format is JSON, because the page sends an object and `serde_json` is already in the tree. PLAN.md §4.1 asks for TOML through facet-toml and Iosevka's plan keys verbatim; the struct is the part that survives that change and the format is an adapter over it.

The CLI takes `--plan <file>` and the page has a text area holding the plan. Both hand it to the same `lemmata_plan::from_json`, and any key either omits takes its default, so a control only has to send what it changed.

## Spacing

`spacing` takes `normal` or `quasi-proportional`, and it selects which capture the caller supplies rather than changing the font the builder writes. The spacing is a property of the drawings: Iosevka draws each glyph into a frame whose width comes from its letter class, so the two are separate builds.

Measured against the monospace build of the same set:

| glyph | advance | ink | outline |
| --- | --- | --- | --- |
| `i`, `l` | 500 → 333 | 350 → 259 | redrawn |
| `m` | 500 → 750 | 398 → 622 | redrawn |
| `M`, `W` | 500 → 667 | 398 → 539 | redrawn |
| `x`, `a` | 500 → 500 | unchanged | byte-identical |
| `period` | 500 → 333 | 156 → 156 | advance only |
| `space` | 500 → 292 | — | — |

The class ratios come from `params/parameters.toml`'s `spacing-quasi-proportional` table: `advanceScaleMM` 1.5, `M` 1.333, `T` 1.167, `F` 0.833, `I` 0.667, `II` 0.5, `Sp` 0.583. A plan chooses the preset and not the ratios. `metricOverride` is the only way to move one, and its documented table lists just `advanceScaleSp` of the seven: setting `advanceScaleSp = 1.0` moves `space` from 292 to 500, and setting `advanceScaleM` or `advanceScaleI` does nothing at all, silently, whether it is written as a plan key or under `metricOverride`.

So a page offers the two presets and cannot offer a slider. `web/public/glyphs-quasi.bin` is the same set drawn at the second preset, and `lemmata-web`'s test asserts the classes: `i` at 333 and `m` at 750 against 500 for both.

## What it does not reach

Outlines, except through the variants. The shapes come from the recorded glyph capture, which is one face at one weight, so a plan changes the font's tables and not its drawing, and a variant choice is a choice among the drawings the capture already carries.

Reaching arbitrary shapes needs two things that do not exist yet. Iosevka derives its parameter set from a build plan through `param`, about a thousand lines, which is what turns a plan into the numbers a glyph definition reads. And the glyph source has to respond to those numbers, which is the strategy question in `docs/glyph-strategy.md`.

Reaching the shapes needs two things that do not exist yet. Iosevka derives its parameter set from a build plan through `param`, about a thousand lines, which is what turns a plan into the numbers a glyph definition reads. And the glyph source has to respond to those numbers, which is the strategy question in `docs/glyph-strategy.md`.

So this is the first slice: the plan reaches the font, verifiably, and the outlines are where the work continues.
