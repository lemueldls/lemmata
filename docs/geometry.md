# Geometry core

`lemmata-geom` holds the primitives that glyph definitions call. All of it is ported from Iosevka's `packages/geometry`, which is JavaScript.

## Spiro

`crates/lemmata-geom/src/spiro.rs` evaluates spiro curves. Provenance is in `docs/spiro.md`.

- `tests/spiro_golden.rs` replays `corpus/spiro-knots.json` against npm `spiro` and matches within 1e-9 font units.
- `tests/glyph_spiro.rs` replays 24 knot sets captured from a real Iosevka build against the output that build produced. Worst deviation is 0.85 font units, under the 1 unit budget. The residual is Iosevka's own Bezier conversion.

## Contrast expansion

`crates/lemmata-geom/src/expand.rs` ports `spiro-expand.mjs` and the `DiSpiroGeometry` driver from `index.mjs`: the two normal passes, the linear interpolation of unimportant knots, and the draw step that reverses the right-hand side and sets corner ends on open skeletons. Transforms go through `kurbo::Affine`.

`tests/spiro_expand.rs` covers nine synthetic skeletons in `corpus/spiro-expand.json`:

- expanded knots match to 1e-9,
- drawn strands match npm `spiro` to 1e-3, the slack being one interpolation case where a 1e-9 knot difference becomes about 1e-5 inside the solver.

One case carries a translated skeleton. Deeper transforms are supported but not covered yet.

Worth knowing before tightening these numbers: Iosevka's exported Bezier drawing sits 0.6 to 1.3 font units from the spiro curve it approximates, so a comparison against that output measures Iosevka's fitter rather than this port. The fixture therefore compares against npm `spiro` directly.

## Pen expansion

`crates/lemmata-geom/src/pen.rs` ports `spiro-pen-expand.mjs`: each profile edge traces the skeleton, unimportant knots are interpolated per edge, and adjacent traces are joined into strips. `SpiroPenGeometry.toBezArcs` unions the strips with typo-geom; `pen_spiro_paths` unions them with linesweeper instead.

`tests/pen_expand.rs` covers five synthetic profiles in `corpus/pen-expand.json`:

- raw strips match Iosevka to 1e-6, which is the port,
- our union's boundary stays within 2.12 font units of those strips,
- against Iosevka's union the gap reaches 33 font units on one three-edge closed profile. That is two boolean engines disagreeing about near-tangential overlaps, not a difference in the strips, which match exactly. Overlap removal has its own tests in `boolean.rs`.

## Stroke expansion

`crates/lemmata-geom/src/stroke.rs` ports `stroke.mjs` with `OffsetCurve` and `RoundCapCurve` from `curve-util.mjs`, and the cubic fitter from typo-geom's `shape-to-bez3`. Each contour is offset to both sides, gaps between offset arcs become round caps, non-cubic arcs are fitted to cubics, and the sides are unioned. With `inside`, the result is intersected with the original shape. `boolean.rs` supplies the union and intersection.

`tests/stroke_expand.rs` covers four contours in `corpus/stroke.json` (rectangle outside and inside, a circle, a triangle with sharp corners):

- the fitted offsets match Iosevka to 1e-6,
- the unioned and clipped shape matches to 4.2e-5 font units, measured as point-to-curve distance.

## Spiro outline

`crates/lemmata-geom/src/outline.rs` ports `spiro-to-outline.mjs`, the body of `SpiroGeometry.toBezArcs`: solve the skeleton, group arcs into runs that end at important knots, drop runs shorter than a hair, fit the remaining arcs to cubics, and apply the gizmo. A shared fitter in `fit.rs` serves this and stroke expansion.

`tests/outline.rs` covers six skeletons in `corpus/outline.json` (open and closed, unimportant knots, a translated gizmo). Arc counts and control points match `SpiroGeometry` at 1e-6.

One subtlety: typo-geom's `convertShapeToBez3` handles a combined arc by recursing into its segments rather than fitting the combination, so runs are flattened before fitting. Fitting the combination instead gave different splits, which is how that was found.

## Fairization

`crates/lemmata-geom/src/fairize.rs` ports `fairizeBezierShape` from typo-geom: corner classification, splitting at extrema, canonical start selection, and straight-run reduction into `FairArc::Straight` or a length-parameterized `CombinedArc`. It carries the quadratic root solver (with its compensated discriminant), the 16-point Gauss-Legendre length integral, and typo-geom's `RootSink`.

One upstream quirk is reproduced on purpose: when a cubic's extrema quadratic is degenerate on one axis, `ClampedRootSink.degenerated` sets its counter to `-1`, the next accepted root is written to a negative array index, and that root is lost. A cubic with two extrema on the other axis is therefore split at only one of them. `docs/geometry.md`'s test relies on matching it, because the drift target is Iosevka's actual output.

`tests/fairize.rs` covers five shapes in `corpus/fairize.json` (rectangle, circle, wave, a rectangle split into colinear cubics, and a four-cubic flower). Arc kinds, counts and coordinates match upstream at 1e-9.

## Quadratic conversion

`crates/lemmata-geom/src/quadify.rs` ports `quadify.mjs` and the `Quadify.auto` fitter from typo-geom: cubic contours become quadratics, then the cleanup pipeline aligns coordinates, drops occurrent points, removes colinear segments, rounds to integers, and rotates the start point. JavaScript's `Math.round` rounds half toward positive infinity, so the port uses `floor(x + 0.5)`.

`tests/quadify.rs` covers five shapes in `corpus/quadify.json` (rectangle, rectangle with colinear points, circle, wave, two contours). Every contour matches upstream at 1e-9.

## Overlap removal

`crates/lemmata-geom/src/boolean.rs` wraps linesweeper for union, intersection and self-overlap removal on kurbo paths. Iosevka uses typo-geom on an integer grid for the same job, so the two agree within that grid's resolution rather than exactly.

## Not ported yet

- The geometry tree classes in `packages/geometry/src/index.mjs` that wire spiro, expansion, transforms and fairization together.

Everything ported above is verified against upstream on its own inputs.

## Captured fixtures

`corpus/glyph-spiro.json` and `golden/glyph-spiro.json` came from an instrumented build, described in `docs/glyph-strategy.md`. They are regenerated by re-running the counter wrapper, not by a checked-in script. The expansion fixtures come from `reference/spiro-expand-oracle`.
