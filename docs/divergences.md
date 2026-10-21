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
| `sqrt` (U+221A) | a drawn hook and a vertical stem against the advance, with a flat top, replacing the check mark | a renderer's vinculum starts at the advance and met the check mark's point at one spot; the stem gives the content a straight edge and the rule a flat top | `crates/lemmata-cli/tests/math.rs::the_radical_has_a_vertical_stem` |

The radical is the first upstream outline the build replaces. It is replaced in
`lemmata-build::font::radical_glyphs`, not in the ported source: the captured
`sqrt` remains the faithful drawing, and the math font gets the hook and stem a
renderer's vinculum needs. The size variants are replaced the same way.

## Interim drawings

The delimiter size variants and assemblies are composed from the same pieces: a
bottom hook, a straight bar of the stroke's cross-section, and a top hook. A
brace uses five parts, with the point between two bars.

The hooks and bars are drawn by `lemmata-build::delimiters`: the hook is a cubic
from the glyph's measured tip to its measured stem, with a round cap at the tip
and a vertical tangent where the bar begins. A paren's stem is its outer edge and
a brace's is its centre. The brace's point is still cut from the text outline at
the middle; it is a fixed part, never repeated, so the cut does not show as a
seam.

## Not divergences

- The family name. Generated fonts do not use Iosevka's reserved names.
- The MATH constants. The table is new, so its values are ours; the reference
  fonts are cited in `docs/math.md` where one was measured.
- Private-use encoding for derived glyphs. A subsetter keeps only reachable
  glyphs, so a form named only by the MATH table has to carry a code. That is
  packaging, not a shape.
