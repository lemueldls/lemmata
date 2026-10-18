# The plan

What a caller chooses about the font, and how far that reaches today.

## What exists

`lemmata-plan` carries the values the writer puts in `head`, `hhea` and `OS/2`: the em size, the vertical metrics, the x-height and cap height, the underline and strikeout positions, the weight and width classes and the italic angle. `Plan::default` is Iosevka's vertical metrics at 1000 units per em, so an empty plan reproduces the font this project built before the plan existed. `tests/default.rs` pins that, along with a changed field reaching the tables.

The boundary format is JSON, because the page sends an object and `serde_json` is already in the tree. PLAN.md §4.1 asks for TOML through facet-toml and Iosevka's plan keys verbatim; the struct is the part that survives that change and the format is an adapter over it.

The CLI takes `--plan <file>` and the page has a text area holding the plan. Both hand it to the same `lemmata_plan::from_json`, and any key either omits takes its default, so a control only has to send what it changed.

## What it does not reach

Outlines. The shapes come from the recorded glyph capture, which is one face at one weight, so a plan changes the font's tables and not its drawing. Widths follow, for the same reason: they are recorded per glyph.

Reaching the shapes needs two things that do not exist yet. Iosevka derives its parameter set from a build plan through `param`, about a thousand lines, which is what turns a plan into the numbers a glyph definition reads. And the glyph source has to respond to those numbers, which is the strategy question in `docs/glyph-strategy.md`.

So this is the first slice: the plan reaches the font, verifiably, and the outlines are where the work continues.
