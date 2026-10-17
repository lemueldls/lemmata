# Reference builds

`private-build-plans.toml` holds the two plans used to produce the oracle fonts. Copy it into a scratch copy of `vendor/Iosevka` and build with `npm run build -- ttf-unhinted::RefA ttf-unhinted::RefB`. The full recipe and the pinned commit are in `docs/upstream.md`.

Build outputs do not belong in the repository. Copy the TTFs to `target/reference/` so `cargo test -p lemmata-oracle` picks them up.
