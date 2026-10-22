# Glyph strategy

How Lemmata gets from Iosevka's glyph definitions to outlines. Measured against the RefA plan on 17 October 2026, with the build cache cleared so the counts cover a full build.

## The two sizes

The glyph definitions are large and keep changing: 349 `.ptl` files, 62,560 lines, 2,669 `create-glyph` blocks (docs/inventory.md).

The geometry surface they call is small. A full RefA build (two faces) recorded these calls:

| Primitive | Calls |
| --- | --- |
| `CombineGeometry.toBezArcs` | 370,920 |
| `SpiroGeometry.toBezArcs` | 358,892 |
| `DiSpiroGeometry.toBezArcs` | 266,786 |
| `TransformedGeometry.toBezArcs` | 95,869 |
| `BooleanGeometry.toBezArcs` | 95,358 |
| `ReferenceGeometry.toBezArcs` | 86,769 |
| `SimplifyGeometry.toBezArcs` | 70,915 |
| `SimplifyGeometry.toContours` | 70,915 |
| `StrokeGeometry.toBezArcs` | 972 |
| `SpiroPenGeometry.toBezArcs` | 480 |
| `ContourSetGeometry.toBezArcs` | 306 |
| `RemoveHolesGeometry.toBezArcs` | 28 |

Twelve primitives, about 1.5 million calls over roughly 71,000 glyphs.

## What the numbers say

The primitive set is small enough to port. A dozen class methods cover everything these faces needed, and they are the same methods `lemmata-geom` is being built around.

The call volume is not small. Routing 1.5 million calls one by one across a JavaScript-to-Rust boundary would dominate a build, so the boundary has to be coarse. One crossing per glyph evaluates about 71,000 geometry graphs, or the graph can be handed to Rust once and walked there.

## Decision

Port the definitions by hand, file by file, and keep the host as the oracle that
gates them.

The definitions live in `lemmata-glyphs`, written against the Rust runtime
rather than translated from JavaScript. A file is read through the reader's
dump, which the pinned compiler emits as S-expressions, and `cargo xtask
expand` prints it with the macros applied. That makes a definition's semantics
explicit without compiling it: `include`, `glyph-proc` and the saving forms are
four lines of meaning behind one word, and the expansion is what a port follows.

The capture is the gate. A ported file is compared knot by knot against the
geometry the pinned build produced, so a mistake in a shape is a wrong number
rather than a wrong-looking letter. The hand port of `lower-a.ptl` was accepted
that way, and the capture has since held every variant it builds.

The host stays, as the second reader. It builds a reference plan on demand and
produces the capture the gate reads, and it is how a shape with no recorded
capture is checked.

## Port surface

A pilot on `letter/latin/lower-a.ptl` (473 lines, and the file behind the captured `a`) asked what a port needs beyond the twelve primitives.

The primitives are what a *host* shims. A *port* also needs the control-knot layer the definitions are written in, because that is what turns `corner`, `curl`, `flat`, `archv` and `widths.rhs` into the knots the primitives consume:

| Piece | Lines | Language |
| --- | --- | --- |
| `geometry/spiro-control.mjs` | 352 | JS |
| `font-kits/spiro-kit.mjs` | 549 | JS |
| `font-kits/letter-model.mjs` | 298 | JS |
| `font-kits/weight-control.mjs`, `boole-kit.mjs` | 122 | JS |
| `glyph/glyph.mjs`, `block.mjs`, `relation.mjs`, `store.mjs` | 1,161 | JS |
| `util` | 443 | JS |
| `meta/aesthetics` | ~544 | PTL |
| **Total** | **~3,470** | mostly plain JS |

That layer is one-time and shared, and the whole corpus uses **51 distinct names** from it. The twenty most used are `corner`, `heading`, `dispiro`, `flat`, `curl`, `g4`, `widths.rhs`, `widths.center`, `widths.lhs`, `spiro-outline`, `archv`, `Arch.lhs`, `Arch.rhs`, `Arch.adjust-x.bot`, `widths.heading`, `g4.down.mid`, `Arch.adjust-x.top`, `g4.up.start`, `g4.right.mid`, `g4.down.start`.

So the two strategies cost, in round numbers:

- **Host**: the twelve primitives, which `lemmata-geom` already implements, plus an engine.
- **Port**: ~3,470 lines of shared DSL, then the definitions per file, with the stage capture as an exact gate.

The second number is about twenty times the first, which is the correction this pilot makes to the Phase 1 reading. The definitions themselves are 60,020 lines across 349 files, and a single variant of one glyph is a small slice of its file: `a.doubleStoreySerifless` is the union of two shape functions, roughly 60 lines of PTL.

What the pilot did not measure is how fast an automated loop converges on one file with the oracle as its gate. That number, not the line counts, decides whether the definitions are portable by hand or only by machine.

## Port progress

The control layer is ported. `lemmata-ptl::control` carries the controls, the flattener and the collector; `lemmata-ptl::spiro_kit` carries the knot builders, the directed pairs, the amendments, the interpolators and the sink helpers. That is `spiro-control.mjs` and the builders in `spiro-kit.mjs`, about 900 of the 3,470 lines.

`reference/spiro-kit-oracle` runs upstream's own layer over `corpus/spiro-kit.json` and writes `golden/spiro-kit.json`; `crates/lemmata-glyphs/tests/spiro_kit.rs` builds the same sequences and compares. Sixteen cases and 69 knots, with every type, width, importance and normal exact and coordinates within 1e-9. The tolerance is for the arc interpolators, which go through `cos`, `sin` and `pow`: V8's implementations differ from Rust's by around 3e-14, so bit-exactness is not available there without matching V8's libm.

The metrics layer is ported too. `lemmata-ptl::metrics` carries `calculateMetrics` from `meta/aesthetics.ptl` and the stroke-width advice from `weight-control.mjs`: the frame, the stroke width, the arch depths, the overshoots, the balance offsets, the ess ratios, and the weight-control sigmoid. The capture writes the plan parameters the build used and the metrics it derived from them, and `tests/metrics.rs` compares all 105 fields. The worst deviation is 1.4e-14, from `tan`, `atanh` and `tanh` differing between V8 and Rust in the last bits.

The per-glyph frame is ported as well. `lemmata-ptl::letter_model` carries `DivFrame`: the division, the sidebearings, the middle, the stroke width, the arch depths, and the slicing and advice methods. The capture dumps eleven frames at several divisions plus the values they derive, and `tests/letter_model.rs` compares 275 of them. The worst deviation is 2.8e-14.

The composition layer is ported too. `lemmata-ptl::glyph` carries `Glyph` (the accumulating combine, the transforms, the width, the context tag), `GlyphProc` and `ForkGlyphProc`, `combineWith`, and the boolean procs from `boole-kit.mjs` (`union`, `intersection`, `difference`, `with-knockout`). `reference/glyph-oracle` builds compositions with upstream's own layer and `tests/glyph_compose.rs` compares thirteen of them: includes, transforms, widths, references, nested procs, tag ejection and booleans. Worst deviation 0.000.

The numeric vocabulary is ported as `lemmata-ptl::util`: `mix`, `barMixL`, `linreg`, `clamp`, `quantize`, `fallback`, `bez2`, `bez3`, `boole`, `boolePn`, `strokeOffset`, `slY`, `slX`, `distP` and the waveforms. `reference/util-oracle` runs upstream's helpers over a grid and `tests/numeric.rs` compares 124 results exactly. Two of them need JavaScript's arithmetic rather than Rust's: `quantize` rounds half toward positive infinity, and the square waveform returns zero for zero where Rust's `signum` returns one.

The collection path is ported as `lemmata-ptl::store`: `GlyphStore` and the save sink. A definition saves through the sink, which names the glyph, derives a `uniXXXX` name from a bare code point, refuses a duplicate, takes the advance width and the global transform from the metrics, and files the result. `reference/glyph-oracle/store-oracle.mjs` runs nine saves through upstream's own sink and store and `tests/glyph_store.rs` replays them, including a save the build filter skips, a forced save and a duplicate name. Exact.

## The first file

`letter/latin/lower-a.ptl` is ported whole. It builds thirteen `a` variants, a
half-height `aHalf` for each serifless double-storey one and a cap-height `aCap`
for each single-storey one, and all sixty are verified. The port is
`lemmata-ptl::shapes` (the helpers a definition calls) and
`lemmata-glyphs::letter::latin::lower_a` (the bodies and the variant table's
stroke advice), 1403 lines, plus `tests/lower_a.rs`.

`shapes.rs` carries `determineMixR`, `HookShape`, the `Arch` namespace including
`centerAt`, `flatside` in its four orientations, `HSerif.rt` and `.rb`, `VBar.l`
and `.r`, `ArcStartSerif`, `InwardSlabArcStart`, `bezControls` and the clamped
arch depths. `lower_a.rs` carries `DoubleStorey`'s three hook styles and seven
bodies, `SingleStorey`'s five bodies over five bars, and `CrossingBody`.

`tests/lower_a.rs` compares the bi-knots of every `dispiro` sequence against the
ones the build produced: 60 variants, 2057 knots, worst deviation 0. Every
coordinate, knot type, importance and normal matches, and so do the expansions
the solver consumes. That is the whole shape path, so the solver, the boolean
stage and the outline all follow from a match here. The outline's bounding box
is checked as well, because `OBarRight.shape` is `OBarLeft.shape` rotated half a
turn and a missing or wrong flip leaves every knot in the right place.

## The second file

`letter/latin/lower-b.ptl` (126 lines) followed. It builds five bowls over one
stem with four serif settings woven across them, plus `bStroke`, `bSlash`,
`latn/be`, `bHookTop` and `mathbb/b`. The capture holds 56 of them: the default
plan selects fewer combinations than the table names, and the store drops a
glyph the build filter does not want.

`lemmata-glyphs::letter::latin::lower_b` is 400 lines. `tests/lower_b.rs`
compares all 56 against the capture: 1869 knots, worst deviation 0. The first
result was exact, which is the point the hand port reached in one file and the
second reached in a few hours.

The shared layer grew to meet it. `OBarLeft` and `OBarRight` moved out of
`lower_a` into `lemmata_ptl::shapes`, followed by `VBar.l`/`.r`, `HSerif` in all
four orientations, `VerticalHook`, the letter-bar overlay, `OShapeOutline` and
`FlatHookDepth`. That is the cost the port surface measured, paid once: the
third letter should need less.

## The third file

`letter/latin/lower-d.ptl` (266 lines) followed, and the prediction held. The
file draws the same bowl and stem as `b` on the other side, so the shapes were
already there: `OBarRight`, `VBar.r`, `HSerif`, `VerticalHook`, the overlay and
`OShapeOutline` covered every body and serif. Two asymmetric pieces had to be
added, `OBarLeft.earlessCorner`/`.earlessRounded` and their flips
`OBarRight.toothlessCorner`/`.toothlessRounded`.

`tests/lower_d.rs` compares 63 of the 67 captured variants: `d`, `dCaron/base`
in a 0.95 frame, `dStroke`, `latn/de`, `dHookTop` and `mathbb/d`, 2128 knots,
worst deviation 0. The four it does not build are `dCurlyTail` and
`cyrl/djeKomi`, which need `CurlyTail` and the arc-start/arc-end pair.

One mistake the gate caught: `RightwardTailedBar` reads the build's
`AdviceStroke` and `SB`, not the frame's, so a variant in a narrower frame keeps
the same tail. The port had used the frame and was 1.4 units out on
`dCaron/base.tailedSerifless`.

## The first capital

`letter/latin/upper-f.ptl` (107 lines) is the first capital letter. It draws one
shape at two heights and overlays it three ways: `F`, `smcpF`, `FBar` and
`currency/frenchFrancSign`, sixteen captured glyphs, 184 knots, worst deviation
0.

It needed `VSerif.mr`, `Rect` and the `EF` crossbar helpers, all small. One
thing it did not need was tag ejection. `eject-contour 'crossBar'` upstream
compares the tag, and the port's `Geometry::Tagged` carries none, so ejecting
would also drop the tagged foot serif. The French franc sign builds the shape
without the crossbar instead, which is exact.

`letter/latin/upper-i.ptl` (109 lines) came next: one stem at cap height with
three serif settings, plus `ILonga`, `smcpI` and `mathbb/I`. Eight captured
glyphs, 46 knots, exact. It needed `VBar.m`, the centered `HSerif` and
`VSerif.ml`, and `BBBarCenter`.

`letter/latin/upper-d.ptl` (126 lines) added the first closed arc.
`DArcShapeT` draws the bowl either rounded or flat-sided, and `D`, `smcpD`,
`Eth`, `smcpEth` and `mathbb/D` build on it. Nine captured glyphs, 178 knots,
exact. It needed `flatside.rd`.

The flat-sided branch caught a mistake the knots alone would not: `flatside.rd`
had been written with `pair`, which emits two control knots, where upstream
writes a `flat` and a `curl`. The arc came out two knots long and the drift was
zero, because every knot it did emit sat where it belonged.

`letter/latin/upper-l.ptl` (130 lines) is the same idea in a simpler shape: a
stem, a foot bar and three serif grades, with `LSlash`, `smcpL`, `smcpLSlash`,
`LHighBar` and `mathbb/L` around it. Sixteen captured glyphs, 136 knots, exact.
It needed nothing new beyond the shapes already there.

`letter/latin/upper-e.ptl` (138 lines) followed: a stem and three bars with four
serif settings, plus `smcpE`, `smcpEBar` and the mirrored `revE`. Seventeen
captured glyphs, 226 knots, exact. It is the same `EF` machinery as `F`, read
from the other side. The outline budget is now six units: `mathbb/E`'s boolean
quadifies into two more points than upstream's and drifts 5.5, while its knots
are exact.

`letter/latin/lower-p.ptl` (173 lines) is the first file to read `SerifFrame`,
ported whole into `lemmata_ptl::serif`. The frame describes where the serifs a
stem can carry sit and how far each form reaches, which is what `lower-e`,
`lower-h`, `lower-m`, `lower-n` and the rest of the lower-case serifed variants
need. Twenty-one captured glyphs, 658 knots, exact; the `thorn` families and
four of the `p` table's combinations are not selected by the plan.

`letter/latin/lower-h.ptl` (175 lines) followed, with `nShoulder` in
`lemmata_ptl::shoulder` as its one new shape: a flat side, an arch and a flat
across, which `h`, `n`, `m` and `b` share. `h`, `hStroke`, `hHookTop`, `heng`,
`hengStroke`, `hengHookTop` and `mathbb/h`: twenty captured glyphs, 559 knots,
exact. Two mistakes the knots caught: `Serifless` was still drawing the
top-left serif, and the bottom serifs are `HSerif.mb` rather than `lb`.

`letter/latin/upper-b.ptl` (560 lines) gave the first two-bowl capital. Four
bodies differ in the bar's position and in whether a gap interrupts the stem;
`B`, `BStroke` and `smcpB` are thirty captured glyphs, 1074 knots, exact. It
needed `BowlXDepth`, the turn depth the two arcs share. `BHookLeft`, `BBar`, the
currency signs, `latn/Beta` and the Cyrillic ve shapes are not ported.

`letter/latin/lower-n.ptl` (403 lines) is the first file to use the shoulder's
earless forms: four bodies, two terminals and four serif settings, twenty-four
captured glyphs, 485 knots, exact. The `HTB` bodies, the Yat serifs and the
hooks are not ported. Two defaults the capture settled: the shoulder's `leftY0`
is `(top - ada) - TINY` rather than zero, and the eared body starts its shoulder
one stroke in so it does not overlap the stem.

`letter/latin/upper-q.ptl` added the first tailed ring: `OShape` with a straight,
crossing or detached tail, for `Q` and `smcpQ`, eight captured glyphs, 244
knots, exact. It needed `OShape` as a di-spiro beside the outline, and
`DiagCor`/`ExtLineCenter`.

`letter/latin/o.ptl` gave the ring itself: `O`, `smcpO`, `o`, the two slashes,
the two barred forms and the two blackboard ones, nine captured glyphs, 268
knots, exact. The Cyrillic, Polish, numeral and horn forms are not ported.

`letter/latin/upper-h.ptl` is two stems and a crossbar with seven serif grades
and an optional tail: `H` and `smcpH`, thirteen captured glyphs, 156 knots,
exact. It added `Design::h_bar_pos`. The turned and half forms, the hooks and
the descenders are not ported.

`letter/latin/upper-p.ptl` is a stem and a bowl that closes on the crossbar,
with two openness settings and three serif settings: `P` and `smcpP`, twelve
captured glyphs, 248 knots, exact. The open forms cut a gap in the stem with a
`difference`. The Greek rho, the hook, the stroke variants and the ruble sign
are not ported.

`letter/latin/upper-n.ptl` is two straight strands joined by a diagonal, in a
symmetric or asymmetric body. The slab setting follows `para.slab`, so the port
now carries it: `Para` and `Metrics` gained `refJut` and the serif frame gained
`enough_space_for_full_serifs`. Twelve captured glyphs, 128 knots, exact.

`letter/latin/lower-il.ptl` is one table for `i` and `l`. Three of its
thirty-two variants are ported, the ones that need only the stem and the
horizontal serif: `serifless`, `serifed` and `hookyBottom`, across `dotlessi`,
`dotlessiCap`, `l` and `cyrl/palochka`. Nine captured glyphs, 36 knots, exact.
The hooks, tails, decompressed and phonetic forms need `CurlyTail`, `DiagTail`
or `HalfRingDescender`.

`upper-m.ptl` and `lower-m.ptl` are the two `m`s. The capital is complete for
`M` and `smcpM`: two side bars, two middle diagonals and an optional middle
stem, across sixteen suffixes, 28 captured glyphs, 448 knots, exact. It also
settled the amended-`flat` question: the width amendment belongs on the knot,
not in a separate `Af` item before it, and once placed there the bars agreed.

The lower-case file is a hundred-entry table across five bodies. All five are
ported, with both leg heights, both terminals and all five serif settings.
Forty of them are in the capture, 612 di-spiro knots, exact. The arches
shoulders are closed spiro outlines, so the test walks the `Spiro` nodes as well
and compares their knots one for one. The plan selects the arches and flat-top
variants and leaves the three earless ones unbuilt, so those are drawn but
ungated.
`mCrossedTail`, the Cyrillic forms and the turned family are not ported.

`lower-f.ptl` has three bodies: the standard stem, the narrow one, and the
flat-hook body that turns the stem's top into a hook. Each takes five counter
ends and three crossbar heights, and the flat-hook body also has four CLC
settings that widen the hook and the crossbar. The standard and narrow bodies
build the stem at the reference bar position and then translate it, which the
tree keeps as a `Transformed` node. The diagonal-tailed ends use `DiagTail.L`:
a superellipse turn sampled by `DiagTail.FArc`, with the superellipse's
derivative giving the depth direction. `HCrossBar.top` and `HookStart` are the
shared helpers; the counter serif, the down extension, the down hook and the
diagonal tail are local to this file. The table names 105 glyphs, and
`fPalatalHook` derives 45 more by hanging the palatal hook from the `overlay`
anchor. Where the counter end sets `palatalHookPos` the hook moves out to it,
and where it does not, the part below the baseline and left of the stem is cut
away. The capture holds 90 of the 210, the standard, narrow and flat-hook
bodies at every hook and bar for both forms, 3087 knots, exact. The four CLC
settings are the left halves of the `f` ligatures. They are built on request by
`ligature_glyphs`, not listed as registry variants, and are ungated because
upstream's cleaned store keeps only their ligature instances.

`symbol/punctuation/asterisk.ptl` builds the asterisk family from one
generator: a ring of petals, each a two-knot di-spiro tapering from the centre
to the tip. Nine rows vary the side count, the rotation and the stroke, and the
`high`, `slashTop`, `halfXH` and `slashBot` forms translate the same drawing.
`dualAsterisk` stacks two copies, and `asterism` stacks three at the width kind
it is built for, scaled about the symbol's middle. `slavonicAsterisk` is the one
drawing the generator does not make: it is the difference of two six-pointed
stars, each the union of two three-corner spiro outlines. The module also builds
the copies the character map selects, so `asterisk/slashBot`, `dualAsterisk`,
`flower`, `opAsterisk` and `slavonicAsterisk` carry ported outlines under their
own names. Forty-two of the forty-six glyphs the module builds are in the
capture, 736 knots, exact. The four half-height forms are not built by the
plan.

`symbol/math/dots-and-colons.ptl` builds `therefore`, `because`, `mathcolon`,
`coloncolon` and `mathCDot` from a shared dot radius, in the round and square
variants. Ten of the twelve glyphs are in the capture, and their outlines match
point for point. `mathCDotInner` has no codepoint and is not captured.

`upper-y.ptl` builds the capital from two `HalfXStrand`s. The strand lives in
`x.ptl` and is the only part of that file ported so far: either a straight pair
of knots for the straight body, or a quadratic curve whose control knots come
from `quadControls`. The serifed settings cut the strands with
`WithSerifOverflowMask`, an intersection with two boxes that split at the
middle of the body. All eight configs are ported, the curly and straight bodies
across no, base, motion and all serifs. The capture holds all eight, 268 knots,
exact.

`lower-j.ptl` builds the dotless bodies: five shapes (bent hook, narrow bent
hook, straight, flat hook and diagonal tail), three top serifs, the barred form
of each, and two curly-tail forms. The barred form adds `LetterBarOverlay.m`,
and the curly tail reuses `CurlyTail` with a straight body. The
`dotlessjBarHookTop` forms need `TopHook.toRight.mBarOuter`, which is not
ported. The composed `j` is the dotless body plus the tittle, and the tittle's
placement is still open, so only the dotless bodies are registered. The capture
holds 22 of the 32 built, 414 knots, exact.

`v.ptl` has two standard bodies (the straight bar and the curly V) over four
serif settings, plus the two cursive forms. The serifs come from `SerifFrame`
and the auto setting draws none at slab level 0. The capture holds 14 of the
18, 226 knots, exact. The hooked and looped forms are not ported: they eject a
tagged contour, and the port's tags carry no name to eject by.

`upper-j.ptl` has five bodies: the bent hook, the flat hook, the descending
bent hook and the two descending flat hooks, over the top serifs they select.
`FlatHookDotlessJShape` moved into the shared shapes layer because both `j`
files draw it. The capture holds all 14 configs, 296 knots, exact.
`JCurlyTail` and `mathbb/J` are not ported.

`upper-t.ptl` builds the three `T` configs: serifless, motion-serifed and
serifed. The crossbar is two `HBar.t` halves, the top serifs are `VSerif.dr`
and `.dl`, and the bottom serif is `HSerif.mb`. The capture holds all three, 28
knots, exact. The hooked, Cyrillic and turned forms are not ported.

`u.ptl` has five bodies (toothed, tailed, flat bottom, toothless corner and
toothless rounded) over the serif settings each case selects. The bowl is a
`dispiro` with `Arch.rhs` or `Arch.lhs` resolved through the port's `arch`
helper; the flat bottom is the corner bowl with its rise set to zero. The
capture holds 36 of the 50 built, 670 knots, exact. The hooked, short-leg,
tailed-stem, Greek and Cyrillic forms are not ported.

`z.ptl` has the straight and curly bodies over four serif settings, for the
capital and the lower case. The straight middle is a four-corner spiro
outline; the curly one is `XStrand`; the lower-case straight middle is an
`ExtLineRhsToLhs`. The capture holds all 16 built, 536 knots, exact. The
cursive body, the crossbar overlays, the hooked and swash forms, `Z.PLK` and
the Cyrillic forms are not ported.

`x.ptl` builds the straight and curly bodies over the two heights and the
serif settings, for both cases. Each diagonal is an `XStrand`; the full and
Greek-chi settings cut the strands with `WithSerifOverflowMask`, which compares
the body box plus the serif's overflow. The mask moved into the shared shapes
layer because `upper-y.ptl` uses it too. The capture holds all 36 built, 2232
knots, exact. The chancery, semi-chancery and cursive forms are not ported.

`symbol/punctuation/percentages.ptl` is ported for the three percent forms:
the dot form, the continuous-slash ring form and the segmented-slash ring
form. The plan selects the segmented one for `%`, so the capture holds only
that drawing; it matches the port at 60 knots, zero deviation. The dots and
continuous forms are built but not gated. Permille, base point, the commercial
minus sign and the connected-bar forks are not ported.

`symbol/punctuation/dashes.ptl` supplies `hyphen`, the three underscore
settings, `figureDash`, `enDash`, `dblHyphen` and `overline`. The capture holds
only `hyphen`, which matches at 2 knots. The open boxes, the Armenian hyphen,
the em dash widths, the spear and the derived compositions are not ported.

`symbol/punctuation/bullets.ptl` supplies the round bullet, the triangular
bullet and the white bullet. `symbol/punctuation/interpuncts.ptl` supplies the
round and square interpuncts. The capture holds the round bullet and the round
interpunct, which match the port; the other three are built but ungated.

The o family is wired into the registry: `O`, `smcpO`, `o`, `OSlash`, `oSlash`,
`OBar`, `oBar` and the two blackboard forms, all nine against the capture at
268 knots.

The lowercase `w` is ported in its straight serifless form and matches the
capture at 8 knots. Its four strokes share the `DiagCorDs` correction, a
four-step fixpoint over `DiagCor`; the inner pair meets at 0.72 of the
x-height. The remaining bodies and their serifs are not ported.

The lowercase `y` is ported in its straight serifless form and matches the
capture at 8 knots. The right half and the left half are clipped by split masks
whose high side depends on the half; swapped masks pass the knot check but the
mask comparison catches it. The curly body, the turn and loop bottoms, the
serifs and the cursive forms are not ported.

The lowercase `k` is ported in its straight serifless form and matches the
capture at 6 knots. The legs are one arm plus a lower leg cut from the same
mask; the lower leg was missing on the first pass and the gate counted the
nodes.

The lowercase `l` is gated through the existing `lower-il` table: the plan's
serifed form matches the capture at 6 knots. The lowercase `r` is ported in its
standard serifless form and matches at 70 knots; the flat-top, earless,
hookless, corner-hooked and narrow bodies are not.

The lowercase `c`, `e`, `g`, `s` and `t` are ported in the forms the sans plan
selects: serifless for `c` and `s`, flat-crossbar for `e`, the single-storey
bent hook for `g`, and the balanced bent hook at the ascender for `t`. They
match the capture at 52, 43, 57, 55 and 29 knots. The `s` neck showed
that upstream delays the two outer `alsoThru` knots so they resolve against the
middle one; resolving all three against the same pair put them in the wrong
places. The serifed forms, the reversed letters and the accented compositions
are not ported. Their tests compare the rendered capture tree to the port's
outline rather than the side contour file, which is produced by a later pass
and trims the open ends of these glyphs. For `c` the two boolean engines trim
the caps two units apart, so the shipped face keeps the captured outline until
that difference closes.

`symbol/punctuation/ampersand.ptl` is ported for the closed form, which the
default sans plan selects. It matches the capture at 33 knots. The lead, its
two masks and the upper loop are all present; the open, tailed and et forms are
not ported.

`symbol/punctuation/small.ptl` is ported for the period and the colon in their
round and square forms. The plan selects the round drawings for `.` and `:`,
which the capture holds; both match, and the square forms are built but
ungated. The comma, semicolon, raised and stacked forms and the three-dot
punctuation are not ported.

`symbol/punctuation/quotes-and-primes.ptl` is ported for the straight ASCII
single quote and the double quote that repeats it either side of the middle.
The capture holds both, 9 knots, exact. The curly, low, reversed and prime
forms, the selectors and the stress marks are not ported.

`symbol/punctuation/slashes-and-number-sign.ptl` is ported for `slash`,
`backslash`, the two joining halves and `doubleSlash`. The capture holds
`slash`, which matches at 0 knots (a closed outline). The number sign, the big
forms, the overbars and the reverse halves are not ported.

The number sign in the same file is ported upright: closed and open bars at
the normal and tall heights. The plan selects the upright form, which matches
the capture at 8 knots. The slanted configs, the joining form and the cap
height selector are not ported.

`symbol/math/relation.ptl` is ported for `equal`, `less` and `greater`. The
capture holds all three, 12 knots, exact. `less` and `greater` are the two
strokes `LessShape`/`GreaterShape` union, with the expansion factor that bends
each end. The negated, subset and ligature forms are not ported.

`symbol/punctuation/small.ptl` is ported for the period, colon, comma and
semicolon in their round and square forms. The plan selects the round comma and
semicolon, which match the capture; the whole module now covers 4 of its 10
cases and 74 knots. The comma's fill and tail are one `union`, and the tail's
quadratic segment uses 12 samples. The raised, stacked, reversed and multi-dot
forms are not ported.

`symbol/punctuation/emotion.ptl` is ported for the question mark with the
smooth hook and the exclamation, each in the round and square dot forms. The
plan selects the round dot, which matches the capture at 29 knots for the
question and 2 for the exclamation. The straight and hooked question hooks, the
reversed and down questions, the interrobang and the turned exclamations are
not ported.

`symbol/punctuation/at.ptl` is ported for the normal-height threefold form,
which the plan selects. It matches the capture at 46 knots. The tall,
solid-inner, fourfold and compact forms are not ported. The arch and centered
arch take their stroke widths from different defaults upstream, and the gate
caught the mix-up.

`symbol/math/arith.ptl` is ported for `plus`, which matches the capture at 4
knots. Its vertical bar runs from `PlusTop` to `PlusBot`, the opposite order
from most bars, which flips the side its widths attach to. The minus, the
Hebrew plus, the enclosures and the derived signs are not ported.

Two things the layer did not have and the port needed:

- `Metrics::stroke_width_blend`, which the `SmoothA`/`SmoothB` advice calls.
- `FunctionInterpolator`. Upstream's `HookStart` and `Arch.lhs`/`Arch.rhs` are
  interpolators that read the knots either side of them when the flattener runs.
  A definition writes those neighbours itself, one line above and one below, so
  the port passes them in directly and returns the resolved knots. The arc
  blends the arch emits stay interpolators for the flattener. This avoids
  teaching the flattener to let an interpolator move its neighbours, which
  upstream's `HookShape` does: it shifts the terminal knot it attaches to and
  gives it a heading. The port returns that knot and the caller substitutes it.

The capture is the oracle for this layer. PTL functions cannot be imported into
JavaScript the way the geometry and kit modules can, so the only way to run
upstream's own code is a full build, and the capture already is one.

Getting there turned up four traps worth recording, all in `docs/glyphs.md`:
the four heading constants in `meta/aesthetics.ptl` point along the stroke, not
along the axes, so `Upward` is `(-HVContrast, 0)` and `Leftward` is `(0, -1)`;
`archv` and `arcvh` are blends rather than knots, so writing a bare `g4` where
one belongs keeps the item count and loses knots; the captured knots for the
earless corner are the *unflipped* corner inside a `Transformed`; and PTL's
indented arguments are siblings, so the tailed bar's `Math.max` takes three.

What remains of the port surface is the dependency manager's block half and
`relation.mjs`, which order glyphs and drive subsetted rebuilds rather than
outlines. After that the definitions themselves, which pull from the shared PTL
libraries (`common/shapes.ptl` 913, `common/derivatives.ptl` 382,
`letter/shared.ptl` 1,389 and its neighbours), part of the 60,020 and amortized
across glyphs.

## How the measurement was taken

The scratch build's `packages/geometry/src/index.mjs` was wrapped with counters and JSON capture, the build cache was removed, and `npm run build -- ttf-unhinted::RefA` was run with `LEMMATA_DUMP` set. The same run produced the captured skeletons that `crates/lemmata-geom/tests/glyph_spiro.rs` replays. The instrumented tree is disposable and lives outside the repository.

## The host runs

The half that turns parameters into outlines now exists for the native path. `lemmata-host` writes upstream's build plan for one shape, runs that build through `npm`, and reads back the capture it leaves. Everything downstream is unchanged, because what crosses the boundary is the same capture the frozen ones were; the plan is what makes it a seam rather than an artifact.

The gate is a shape nothing in the repository holds a capture for. RefB is slab serifs at bold, and 24 letters of the capture agree with upstream's own `RefB-Bold.ttf` within two units of ink. The tolerance is sharp rather than generous: the bold `m` is 482 units wide against the regular one's 398, so a capture of the wrong shape fails on the first glyph. The test skips unless `LEMMATA_HOST_BUILD` is set, because it starts an external build.

`lemmata-cli build` takes `--weight`, `--width`, `--slope`, `--serifs` and `--spacing`, and names a shape that way. The first build of a shape takes tens of seconds and lands under `target/host`; the next one reads the cache. A build is tens of seconds and the shape is the whole of the cache key, so a cached build does not re-run the dump and the host clears its output directory before each run rather than trusting it.

The host is the reference path, not the product. The browser has none, and it
needs none: the ported definitions are Rust, so a page builds from them the same
way the native binary does. What is left for the host is the shape as a plan
field rather than a set of flags, and the capture it regenerates when the pinned
tag moves.

## The site draws from the port

The registry is the seam between the definitions and the site. `lemmata_glyphs::registry::build` takes the metrics and the design and returns every glyph the port draws, by the name the build gives it. Each definition exposes `glyphs()` for its own variant table, so a glyph that exists in a module cannot be missing from the list.

`lemmata-source` derives the metrics from the plan through `lemmata_ptl::param` and the four parameter files, builds the registry, and swaps a glyph's contours for the capture's when the name is in both. Everything else keeps the capture. Both front ends call it: the page for its wasm surface and the CLI for its build. That is what keeps a drawn texture form the same in both, and the parity pair the check that they have not drifted. The built font is still byte-identical to the recorded parity pair for the default plan, which is the test: 376 glyphs in the registry, 269 of them in the capture, 7539 knots, zero deviation, and the same font bytes the fingerprint records.

What that buys today is the `a` family, which is what the page's fixture holds. A visitor who changes the plan's weight, width or slope now moves those outlines, and the variant picker moves the drawing it selects. The other glyphs stay as drawn until they are ported.

## Ligatures are contextual alternates

A reference build with `inherits = "default-calt"` settles what a ligature is in this source: not one glyph replacing a sequence, but each character replaced by a joined form that meets its neighbour. `->` shapes to two glyphs at full advance, not one glyph. The GSUB lookups are chaining contextual rules applying single substitutions, which is why the first extraction attempt found `.join-r` intermediates and no type-4 entries.

`lemmata_layout` now carries both shapes: `Effect::Contextual` with one coverage per position, which is what the ligation rules need, and `Effect::Ligature` for a type-4 lookup a future definition may prefer. `Rule`'s backtrack and lookahead are per position rather than one flattened coverage, so a two-character context is expressible.

The fixture is embedded now. `reference/ligatures.json` holds the rules and the
outlines of every target, and `lemmata_build::ligatures` adds both to the font
under `calt`. The extractor writes one chaining rule per changed position, and
each later rule names the joined form the earlier one produced, so `<=`, `>=`,
`->` and `=>` join at both ends. A rule that kept the plain context drew the
left glyph's extension over an unchanged right glyph.

The outlines come from the reference build, so they are fixed at its plan.
`symbol/ligation.ptl` is ported for the arrow bars: `plus`, `hyphen`, `equal`
and `ident`, both notch settings, across every left and right join class.
`reference/ligature-sources.json` maps the targets whose captured outline
matches a candidate exactly (21 of the 143) back to the source name, and
`lemmata-source` draws those at the plan's metrics, so a bold or slanted plan
gets a matching rod. The comparison angles, the arrow heads, the waves and the
bracket joins are separate shapes and stay on the fixture until they are
ported.
