# Corpora

Versioned inputs for the test suites.

- `prose.txt`: running text, terminal output, mixed punctuation.
- `code.txt`: source in several languages with long identifiers and alignment.
- `ligatures.txt`: every ligature trigger the layout tables define.
- `equations.typ`: Typst source for the math corpus.
- `equations.mathml`: MathML torture page for the browser suite.

The JSON files are the geometry corpora: minimal reproductions of the shapes and
arcs each stage of `lemmata-geom` has to handle, pinned against upstream's own
output.

A corpus is what a suite runs, so it is written in what the font carries. The
coverage capture holds the lower-case letters, the lining digits and the
punctuation the site's font needs, and the shaping suite names the characters a
corpus needs that are not there rather than letting them shape `.notdef`:
`code.txt` wants `V`, `[`, `]`, `{`, `|`, `}` and `ligatures.txt` wants `|`.
That set is the coverage gap, and it closes when the capture grows.

Fingerprints belong in `golden/`.
