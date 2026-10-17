# Upstream oracle

Iosevka v34.9.0 is the reference. Recorded here so every drift report can name it.

| | |
| --- | --- |
| Upstream | https://github.com/be5invis/Iosevka |
| Tag | v34.9.0, released 2026-09-26 |
| Commit | `2487637bb71f41b2a66134afd9faa6c316e5a438` |
| Checkout | `vendor/Iosevka`, a shallow submodule pinned to the tag |

The source tree is unmodified. Private build plans and build outputs stay outside the submodule.

## Reference fonts

Built from a copy of the pinned tree in a scratch directory:

1. Copy `vendor/Iosevka` to a scratch directory.
2. `npm install`.
3. Add `private-build-plans.toml` with two plans that differ in weight and serif style.
4. `npm run build -- ttf-unhinted::RefA ttf-unhinted::RefB`.

Unhinted target: `ttfautohint` is not installed on this machine, and hinting is not part of the comparison.

The four resulting TTFs (`RefA-Regular`, `RefA-Extended`, `RefB-Bold`, `RefB-Extended`) were copied to `target/reference/`, which is outside version control. `cargo test -p lemmata-oracle` reads them from there, or from `LEMMATA_REFERENCE_DIR`.
