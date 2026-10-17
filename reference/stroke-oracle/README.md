# Stroke oracle

Runs Iosevka's `strokeArcs` over `corpus/stroke.json`, plus a rebuild of the private `offsetContour` from the exported `OffsetCurve` and `RoundCapCurve`, so `lemmata-geom`'s stroke expansion can be compared against the fitted offsets and the unioned output.

```
IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs \
  < ../../corpus/stroke.json > ../../golden/stroke.json
```

`IOSEVKA_BUILD_DIR` points at a scratch copy of the pinned Iosevka tree with `npm install` done.
