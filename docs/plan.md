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

`param::metric_override` is the rest of the layer: a plan may override any of twenty-seven metrics by name, with a number or with an expression over the defaults, the shape and a `blend` function. It carries a small recursive-descent evaluator, and its number literal accumulates in `f64` rather than parsing an integer, because the order changes the last bit. A field that cannot be overridden is skipped and reported, which is what upstream does.

`Para::from_named` turns the layer's keys into the structure the metrics code reads, and the capture reads through the same mapping rather than a second copy of it.

Both oracles are upstream's own, and both pass over the whole set:

| check | result |
| --- | --- |
| the parameter set, against the build's 91 parameters | 0 missing, 0 wrong |
| the metrics, from the plan's shape rather than from the capture | 105 fields, worst deviation 1.4e-14 |

The second is the whole chain: a shape, the parameter walk, `Para`, `Metrics::calculate`. It agrees with the pinned build to the last bits that `tan` and `tanh` differ between V8 and Rust, which is the same residual the capture-based check has.

What is not ported, and where it belongs:

| piece | why not, and where it goes |
| --- | --- |
| `variant.mjs` (571) with `variants.toml` (13,540) | names and orders glyph variants rather than measuring them. Layout work. |
| `ligation.mjs` (65) with `ligation-set.toml` (737) | ligature sets. Layout work. |
| `subset-filter.mjs` (52) | build-time subsetting, not parameters. |

## What is left before the shapes move

One thing: a glyph source that reads the numbers. A plan now reaches the parameter set and the metrics, verifiably; what it does not reach is the outlines, because those still come from the recorded capture. That is the strategy question in `docs/glyph-strategy.md`.
