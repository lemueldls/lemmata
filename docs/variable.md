# Variable fonts

Variable output is not part of the current build. This note records what it
would take and what the port already rules out.

## What the writer has

`write-fonts` 0.54.1 writes the tables a variable font needs: `fvar`, `gvar`,
`avar`, `STAT`, `HVAR` and `VVAR`. The missing part is not the file format.

## The compatibility problem

A `gvar` delta is a list of point displacements. Every master of a glyph must
have the same point count and the same contour order, or the deltas address the
wrong points.

The port does not guarantee that today. It draws a serif only where the shape
asks for one, and a heavier weight can add a point or drop one. Measured at
400 and 700 over the full capture:

| Result | Glyphs |
| --- | --- |
| Same contour count and point count | 418 |
| Different point count | 486 |
| Total compared | 904 |

The differing glyphs are not a rounding difference. `B.standardSerifless` has
61 points at 400 and 57 at 700. `B.standardBilateralSerifed` goes from 65 to 61.
A `gvar` built from those two masters would be wrong for 486 glyphs.

The captured glyphs are a second limit. They are one drawing at one weight, so
they have no second master at all. A variable face that varies the 418
compatible ported glyphs and leaves the captured ones fixed would change weight
inconsistently across the alphabet.

## What a first slice needs

1. **A compatibility contract.** Either the port keeps one topology at every
   weight, or a matching step pairs points between masters before `gvar` is
   written. The first is a constraint on every glyph definition. The second is
   a geometry problem with its own failure modes.
2. **A test that fails on an incompatible pair.** The measurement above is a
   useful gate: it should run for every weight pair the axis names, not only
   400 and 700.
3. **Two masters and a weight axis.** `fvar` with `wght` 400 to 700, `gvar`
   deltas for the compatible glyphs, `HVAR` for the advances, and `STAT` for the
   style names.
4. **A decision about the capture.** The unported glyphs need masters or the
   variable face is a hybrid. That points at finishing the port before the
   axis, or at a variable build that only covers the ported set and says so.

Width and slope axes come after weight. CJK adds a third problem: the source
font has to be variable itself, and its `gvar` has to be merged rather than
rebuilt.

## The plan

Variable fonts stay a non-goal until the compatibility contract exists. The
next useful measurement is a per-glyph list of point counts at every weight the
port can draw, with the differing glyphs named. That list is the work queue for
either the contract or the matcher.
