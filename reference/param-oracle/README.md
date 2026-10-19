# Parameter oracle

Runs upstream's parameter layer over a set of shapes and writes the parameter
sets it produces, so the Rust port of that layer can be compared against the real
one.

Unlike the glyph capture this needs no patched build: the layer is plain
JavaScript rather than PTL, so the oracle imports `@iosevka/param` and the tables
and calls it. It needs a scratch copy of the pinned tree with `npm install` run
in it, which is the same copy the other oracles read.

```
IOSEVKA_SOURCE=/tmp/opencode/iosevka-src node oracle.mjs > ../../corpus/param.json
```

The shapes are twenty: the reference, the weight grades from 100 to 900, width
720, the two slopes, slab serifs, three spacings, and two combinations. A plan
names a weight and a width always, because the tables that carry them are only
resolved when the plan does.

The output is `corpus/param.json`. `crates/lemmata-ptl/tests/param.rs` compares
the port against it, and pins the values that disagree.
