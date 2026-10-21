# Divergences from upstream

The glyph definitions are ports. A ported glyph is expected to reproduce the
capture exactly: `crates/lemmata-glyphs/tests/registry.rs` compares the knots,
`tests/arcs.rs` compares each stage, and a deviation is a bug rather than a
design choice.

The font also carries glyphs upstream does not. This file lists them, so a
reader can tell a new drawing from a port that drifted. The rule is to add
rather than replace: a new form may sit beside an upstream one, but an upstream
drawing is not changed without an entry here and a test that pins the change.

## New glyphs

| glyphs | upstream | ours | described in |
| --- | --- | --- | --- |
| MATH constants, per-glyph information and stretchy constructions | no MATH table | written or derived in `lemmata-build` | `docs/math.md` |
| stretchy size variants and assembly pieces | none | derived from the base outlines | `docs/math.md` |
| horizontal arrow variants and assemblies | none | derived from the drawn arrows | `docs/math.md` |
| `ssty` alternates | none | two x-scaled forms per covered glyph | `docs/math.md` |
| `flac` accents | none | `.mathcap` forms, 0.9 high and 1.1 wide | `docs/math.md` |
| `dtls` dotless forms | none | derived by removing the tittle | `docs/math.md` |
| texture forms | upstream's `TXTR` feature | redrawn per form, not scaled | `docs/math.md` |
| corner kerns | none | derived from the top-corner overhang | `docs/math.md` |

## Modified outlines

| glyph | change | reason | pinned by |
| --- | --- | --- | --- |
| `sqrt` (U+221A) | a bar of the rule thickness at the top, from the advance to the ink's right edge | a renderer's vinculum starts at the advance and met the check mark's point at one spot, leaving a notch | `crates/lemmata-cli/tests/math.rs::the_radical_has_a_flat_vinculum_edge` |

The radical is the first upstream outline the build changes. It is changed in
`lemmata-build::font::radical_vinculum`, not in the ported source: the ported
`sqrt` remains the faithful drawing, and the build adds the edge a math
renderer needs.

## Interim drawings

The delimiter assemblies currently repeat a straight bar derived from the base
outline. Its width now comes from the stroke's cross-section at the band's
midpoint rather than the band's bounding box, which removed the extra weight
the bar carried. It is still a cut, not a drawn join: the text paren has no
straight stem, so the bar cannot be tangent to the curve at both ends, and a
curve that is only tangent at one point cannot give a long straight middle.

The replacement is to draw the pieces at U+239B to U+23AD (upper hook,
extension, lower hook, and the brace's point and bars) and compose both the
size variants and the assembly from them. Those pieces do not exist upstream,
so they will appear here as new drawings rather than as a modification.

## Not divergences

- The family name. Generated fonts do not use Iosevka's reserved names.
- The MATH constants. The table is new, so its values are ours; the reference
  fonts are cited in `docs/math.md` where one was measured.
- Private-use encoding for derived glyphs. A subsetter keeps only reachable
  glyphs, so a form named only by the MATH table has to carry a code. That is
  packaging, not a shape.
