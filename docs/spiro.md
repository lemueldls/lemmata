# Spiro

Glyph curves come out of a Spiro solver, the same implementation family Iosevka uses. This note records provenance, because "spiro" names two lineages with different licenses.

## Licenses

| Source | License | Role |
| --- | --- | --- |
| [raphlinus/spiro](https://github.com/raphlinus/spiro) | Apache-2.0 | Raph Levien's original implementation. |
| [be5invis/spiro-js](https://github.com/be5invis/spiro-js), npm `spiro` 3.0.1 | Apache-2.0 or MIT | The package Iosevka's build imports. `crates/lemmata-geom/src/spiro.rs` is a port of it. |
| [MFEK/spiro.rlib](https://github.com/MFEK/spiro.rlib) | Apache-2.0 | Rust transpile of the C original, kept as an alternative. |
| [fontforge/libspiro](https://github.com/fontforge/libspiro) | GPL-3.0 | Not used and not usable here. |

The GPL belongs to libspiro, a separate maintained project. Nothing on the chosen path touches it.

## Port

`crates/lemmata-geom/src/spiro.rs` follows spiro-js 3.0.1 function by function, including the constants (four integration steps, finite-difference reciprocal 2e6, at most 32 subdivision stops) and the band solver.

Two JavaScript behaviors are reproduced on purpose:

- Writes that land past the 11-slot band row hit array slots no reader visits. The port drops them.
- The solver runs in `f64` with the same iteration cap (10 rounds, stop when the norm drops below 1e-12).

Iosevka passes `GEOMETRY_PRECISION = 1/4` as the subdivision delta, and the corpus uses the same value.

## Verification

`corpus/spiro-knots.json` holds nine knot sets: open and closed paths, every knot type, two to six knots. `reference/spiro-oracle` runs npm `spiro` over them and writes `golden/spiro-cases.json`, tagged with the oracle version. `cargo test -p lemmata-geom` compares the port against the goldens within 1e-9 font units, and the current corpus passes.

Regeneration steps are in `reference/spiro-oracle/README.md`.
