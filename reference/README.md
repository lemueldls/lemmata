# Reference builds

`private-build-plans.toml` holds the two plans used to produce the oracle fonts. Copy it into a scratch copy of `vendor/Iosevka` and build with `npm run build -- ttf-unhinted::RefA ttf-unhinted::RefB`. The full recipe and the pinned commit are in `docs/upstream.md`.

Build outputs do not belong in the repository. Copy the TTFs to `target/reference/` so `cargo test -p lemmata-oracle` picks them up.

`spiro-kit-oracle/` runs upstream's spiro control layer and knot builders over `corpus/spiro-kit.json`, so the Rust port of that layer can be compared against it.

`glyph-oracle/` builds glyph compositions with upstream's `Glyph`, `GlyphProc` and `boole-kit` over `corpus/glyph-compose.json`, so the Rust port of the composition layer can be compared against it.

`glyph-capture/` patches the scratch tree instead of importing from it, because per-glyph geometry is not reachable any other way. It writes a glyph set's geometry trees and converted contours for `lemmata-glyphs` and `lemmata-cli` to read.
