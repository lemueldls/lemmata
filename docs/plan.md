# The plan

What a caller chooses about the font, and how far that reaches today.

## What exists

`lemmata-plan` carries the values the writer puts in `head`, `hhea` and `OS/2`: the em size, the vertical metrics, the x-height and cap height, the underline and strikeout positions, the weight and width classes and the italic angle. `Plan::default` is Iosevka's vertical metrics at 1000 units per em, so an empty plan reproduces the font this project built before the plan existed. `tests/default.rs` pins that, along with a changed field reaching the tables.

The boundary format is JSON, because the page sends an object and `serde_json` is already in the tree. PLAN.md §4.1 asks for TOML through facet-toml and Iosevka's plan keys verbatim; the struct is the part that survives that change and the format is an adapter over it.

The CLI takes `--plan <file>` and the page has a text area holding the plan. Both hand it to the same `lemmata_plan::from_json`, and any key either omits takes its default, so a control only has to send what it changed.

## What it does not reach

Outlines. The shapes come from the recorded glyph capture, which is one face at one weight, so a plan changes the font's tables and not its drawing. Widths follow, for the same reason: they are recorded per glyph.

## The parameter layer

`lemmata-ptl::param` is the port of `@iosevka/param`'s parameter half: the walk over the style tree, where a style can inherit, multiply, add, append, remove, or interpolate a value across a range of grades, and the interpolator behind that last one (`monotonic-interpolate`, Fritsch and Carlson).

The tree is data, and the data is upstream's. `params/parameters.toml`, `shape-weight.toml`, `shape-width.toml` and `shape-slope.toml` are read verbatim, which is what `r[plan.iosevka-keys]` asks for. They need a TOML reader for the subset they use, which is `param::toml`: tables with numeric, negative and quoted keys, dotted keys, trailing comments, literal and triple-literal strings, integers, floats, booleans and arrays. `tests/param.rs` parses the real files rather than fixtures.

Verified against the pinned build rather than against fixtures: the reference shape, sans serifs at normal spacing and weight 400, produces **all 91 parameters the build derived**, none missing and none wrong.

What is not ported, and where it belongs:

| piece | why not, and where it goes |
| --- | --- |
| `variant.mjs` (571) with `variants.toml` (13,540) | names and orders glyph variants rather than measuring them. Layout work. |
| `ligation.mjs` (65) with `ligation-set.toml` (737) | ligature sets. Layout work. |
| `subset-filter.mjs` (52) | build-time subsetting, not parameters. |
| `metric-override.mjs` (244) | the plan's explicit per-metric overrides. Belongs here and is the next piece. |

## What is left before the shapes move

Three things, in order. `Para` from the parameter map, which is a name mapping over the keys the layer produces. Then `Metrics::calculate` over it, checked against the 105 metrics the build derived, which is the next oracle up. Then a glyph source that reads those numbers, which is the strategy question in `docs/glyph-strategy.md`.

The plan reaches the font today, verifiably. It does not reach the outlines, and the parameter layer is what stands between them.
