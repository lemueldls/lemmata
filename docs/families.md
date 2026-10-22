# Families

A plan builds one face. A family document builds the faces a type family needs:
Regular, Bold, Italic, Bold Italic, and any width or weight grades in between.
The document is TOML, or JSON with the same keys.

```toml
[family]
name = "Lemmata"
version = "Version 1.000"
vendor = "LEMM"
out = "dist/lemmata"
formats = ["ttf", "woff2", "css"]
policy = "mixed"
capture = "web/public/glyphs.bin"

[family.base]
capHeight = 735
xHeight = 520
ascender = 965
descender = -215
lineGap = 70
width = 600
spacing = "normal"
serifs = "sans"

[family.cjk]
source = "target/cjk/SarasaMonoSC-Regular.ttf"
scale = 2.0
ranges = "U+4E00-U+9FFF"
label = "CN"

[family.instances.regular]
weight = 400
slope = 0
capture = "web/public/glyphs.bin"

[family.instances.bold]
weight = 700
slope = 0

[family.instances.italic]
weight = 400
slope = 12

[family.instances.bold-italic]
weight = 700
slope = 12
```

`family.cjk` is optional. It bundles one CJK source into every face, with the
advance scaled by `scale` times the Latin cell. `label` appends a charset label
to the family name, which is what a build split by charset uses. `docs/cjk.md`
covers the source and the widths.

Build it with:

```sh
IOSEVKA_BUILD_DIR=/path/to/Iosevka lemmata build --family families/lemmata.toml
```

The build directory is a scratch copy of `vendor/Iosevka` with `npm install`
run in it. An instance with a `capture` uses that file. An instance without one
asks the host to capture its weight, width, slope, serifs and spacing. The
capture is cached under `target/host` by shape and codepoint set.

`out` is relative to the document. `--out` overrides it, and is relative to the
working directory. `--only bold,italic` builds a subset of the instances;
`--subset U+0020-U+007E` restricts every face's character set.

`capture` at the top level is the base capture. The build reads its codepoints
when `codepoints` is absent; it does not use the base drawing for another face.
`codepoints` is a comma-separated list of `U+XXXX` or `U+XXXX-U+YYYY` ranges.

## Names

The instance fields `legacy_family`, `legacy`, `typographic_family`,
`typographic`, and `postscript` override the names the build derives.

The default names follow the RIBBI rules:

| weight and slope | legacy family | legacy subfamily | typographic subfamily |
| --- | --- | --- | --- |
| 400 upright | Lemmata | Regular | Regular |
| 700 upright | Lemmata | Bold | Bold |
| 400 slanted | Lemmata | Italic | Italic |
| 700 slanted | Lemmata | Bold Italic | Bold Italic |
| 600 upright | Lemmata SemiBold | Regular | SemiBold |
| 400 at width class 7 | Lemmata Expanded | Regular | Expanded |
| 700 at width class 7 | Lemmata Expanded | Bold | Expanded Bold |

The builder sets `head.macStyle`, `OS/2.fsSelection`, `OS/2.usWeightClass`,
`OS/2.usWidthClass`, `post.italicAngle`, the caret slope, and the typographic
name pair. Duplicate legacy names or PostScript names stop the build.

## Output

The build writes one TTF and WOFF2 per face, plus `lemmata.json` and
`lemmata.css`. `lemmata.json` records the names, the shape of every face, the
files, and how many glyphs came from the port and how many from the capture.

`lemmata.css` pairs the faces with `font-weight`, `font-style` and
`font-stretch`:

```css
@font-face {
  font-family: "Lemmata";
  src: url("./Lemmata-Bold.woff2") format("woff2"),
       url("./Lemmata-Bold.ttf") format("truetype");
  font-weight: 700;
  font-style: normal;
  font-stretch: 100%;
  font-display: swap;
}
```

## Coverage

`policy` decides what happens when the port does not draw a glyph:

- `mixed` builds the face and records the counts.
- `strict` stops the build when a glyph comes from the capture or is missing.
- `port-only` stops the build when any glyph comes from the capture.

The capture is at the face's shape when the host built it. A frozen capture has
no shape sidecar, so the manifest marks it `file-unverified`. That distinction
matters: a per-shape capture keeps an unported glyph at the right weight and
slope, while one base capture reused for a bold face does not.

The browser still draws only the ported glyphs at a new shape. A family built
from the page needs a capture per shape, or faces built natively and served as
files. `lemmata build --family` is the native path.

Variable fonts are not part of this. The output is a set of static faces.
