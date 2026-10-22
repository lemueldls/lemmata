# CJK

The CJK glyphs come from a built font rather than from a ported definition.
Sarasa Gothic is a composite, and so is Source Han Sans before it. Reading a
source font is the smaller job and the one that keeps the regional cuts
available.

## Where the glyphs come from

Sarasa Gothic is a composite of three projects:

| Part | Source |
| --- | --- |
| CJK ideographs, kana, hangul | Source Han Sans, by region: SC, TC, HC, J, K, plus half-width cuts |
| Classical Chinese | Shanggu Sans TC, used for the `CL` faces |
| Latin, Greek, Cyrillic, proportional | Inter |
| Latin, Greek, Cyrillic, monospaced | Iosevka |
| Punctuation | Sarasa's own passes, combining the Latin and CJK sources |

So the CJK characters are Source Han Sans. Sarasa adds the merge, the
punctuation and the regional selection. The first Lemmata CJK build uses
`SarasaMonoSC-Regular.ttf` from the Sarasa release, which is already a TrueType
font.

All four upstream projects are OFL 1.1. A generated font needs a new family
name, the OFL text, and the copyright notices of the sources it used. Sarasa's
license carries Adobe's reserved name "Source", which a generated family must
not use.

## The 2:1 width

Sarasa Mono is 2:1 because Iosevka's Latin cell is 500 units and Source Han
Sans's CJK cell is 1000. The same relationship at Lemmata's 600-unit cell is
1200.

`--cjk-scale` names how many Latin cells a full-width CJK glyph occupies. The
default is 2.0. A glyph is scaled horizontally by `target_cell * scale /
source_upem`, and its advance is the source advance scaled by the same factor.
Vertical scale is `units_per_em / source_upem`, which is 1 for Sarasa. A
half-width CJK glyph keeps half the cell, so punctuation does not become a full
ideograph. Combining marks keep their zero advance.

The source's em is the CJK cell. That is what makes a 1000-unit ideograph fill
two 600-unit cells after the 1.2 horizontal scale.

## Command line

Bundle CJK glyphs into one face:

```sh
lemmata build \
  --cjk target/cjk/SarasaMonoSC-Regular.ttf \
  --cjk-ranges U+4E00-U+9FFF \
  --cjk-label CN \
  --out LemmataCN-Regular.ttf
```

`--cjk-ranges` takes the same syntax as `--subset`. Without it the CN set is
used: radicals, CJK punctuation, Extension A, the unified ideographs and the
full-width forms. Extension B is left out by default because its 42,000 glyphs
plus the rest would pass the 65,535 glyphs a `maxp` table can count. Name the
range when the source and the output can hold it.

`--cjk-label` appends a label to the family name. It is how a build that splits
by charset becomes `Lemmata CN` while a Latin-only build stays `Lemmata`. The
label is separate from the width label, so a 600-unit face is still
`Lemmata CN Expanded`.

## Family documents

A family shares one CJK source across its faces:

```toml
[family.cjk]
source = "target/cjk/SarasaMonoSC-Regular.ttf"
scale = 2.0
ranges = "U+4E00-U+9FFF"
label = "CN"
```

The source path is resolved relative to the family document. Every instance
gets the same CJK glyphs. The `label` changes the legacy and typographic family
names the same way `--cjk-label` does.

## What is not there yet

- **CFF and CFF2 sources.** The writer is `glyf` only, and the reader rejects
  cubic outlines. Sarasa's TTF is quadratic, so the first source works. Source
  Han Sans's own OTF would need a cubic-to-quadratic step.
- **Regional layout.** A face is built for one region at a time. `locl`,
  `vert`, `vrt2`, `palt` and `halt` are not written.
- **Extension B by default.** The CN range stops at the unified ideographs. The
  glyph count guard is in `lemmata-cjk`.
- **Variable fonts.** The output is static, like the rest of the family build.

## Tests

`cargo test -p lemmata-cjk` builds a tiny TrueType source with one ideograph and
checks the 2:1 scale without needing Sarasa.

`cargo test -p lemmata-cli --test cjk` reads
`target/cjk/SarasaMonoSC-Regular.ttf` when it is present, bundles U+4E00 through
U+4E0F, and shapes `a一a` to three cells. Set `LEMMATA_CJK_SOURCE` to use a
source at another path. The test skips when the file is absent, like the other
reference tests.

## The web and releases

The browser does not read a 14 MB source. CJK faces are built natively, and the
site offers prebuilt specimens and downloads. The full archives belong on
GitHub Releases and a Cloudflare R2 mirror. The site's `/cjk` page loads a small
WOFF2 subset for the specimen and links the full release. `NUXT_PUBLIC_CJK_BASE`
points the page at the R2 mirror; without it the page shows the GitHub release
link only.

The sample is built from the site capture and a short character list:

```sh
lemmata build   --glyphs web/public/glyphs.bin   --cjk target/cjk/SarasaMonoSC-Regular.ttf   --cjk-ranges 'U+4E00-U+4E1F,U+4E2D,U+6587,U+6392,U+7248,U+4E8C,U+4E09,U+56DB,U+4E94,U+516D,U+4E03,U+516B,U+4E5D,U+5341,U+3001,U+3002,U+FF0C,U+FF01,U+FF1F,U+FF1A,U+FF1B'   --name 'Lemmata CJK Sample'   --out web/public/fonts/lemmata-cjk-sample.woff2
```

The sample's notice is beside it. The source itself is not committed; it is a
release download from Sarasa.
