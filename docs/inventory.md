# Upstream inventory

First pass over `vendor/Iosevka` at v34.9.0. Counts come from `grep` over the source tree, so treat them as magnitudes, not exact call graphs.

## PTL source

| Measure | Count |
| --- | --- |
| `.ptl` files | 349 |
| `.ptl` lines | 62,560 |
| Files in `font-glyphs` | 337 |
| Files in `font-otl` | 12 |

Macro occurrences:

| Macro | Occurrences |
| --- | --- |
| `include` | 8,582 |
| `glyph-proc` | 3,313 |
| `create-glyph` | 2,669 |
| `glyph-block` | 2,181 |
| `glyph-module` | 658 |

## Shape vocabulary

Word counts inside `font-glyphs` PTL files:

| Word | Count |
| --- | --- |
| `stroke` | 2,610 |
| `union` | 685 |
| `spiro` | 355 |
| `difference` | 310 |
| `transform` | 270 |
| `translate` | 195 |
| `rect` | 169 |
| `circle` | 60 |
| `skew` | 44 |
| `rotate` | 15 |
| `polygon` | 8 |
| `oval` | 6 |

## Geometry package

The primitives live in `packages/geometry` as JavaScript, not PTL. Modules include `spiro-control`, `spiro-expand`, `spiro-pen-expand`, `spiro-to-outline`, `stroke`, `quadify`, `segment`, `transform`, `box`, `anchor`, `curve-util`, `point`, `encoding`.

Exported classes and helpers include `SpiroGeometry`, `DiSpiroGeometry`, `SpiroExpander`, `PenSpiroExpander`, `SpiroFlattener`, `QuadifySink`, `OffsetCurve`, `BooleanGeometry`, `CombineGeometry`, `SimplifyGeometry`, `RemoveHolesGeometry`, `RoundCapCurve`, and the `Bez3`/`BiKnot`/`PenKnot` machinery. The boolean operations are `union` and `difference`, both under `BooleanGeometry` and `CombineGeometry`.

## Reference font facts

| Measure | Value |
| --- | --- |
| Outlines per reference face | 48,725 (`RefA-Regular`, `RefB-Bold`), 48,728 (`RefA-Extended`, `RefB-ExtendedBold`) |
| Extraction time for all four faces | about 1.9 s |
| Extraction reproducibility | bit-identical across repeated runs (`extraction_is_deterministic`) |

## Still missing

The distinct primitive count and call volume need a run of the compiled glyph code with stubbed geometry calls. That measurement has not been made yet.
