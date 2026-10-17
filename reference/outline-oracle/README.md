# Spiro outline oracle

Runs Iosevka's `SpiroGeometry` over `corpus/outline.json`, so `lemmata-geom`'s outline path (solve, simplify, fit) can be compared against it.

```
IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs \
  < ../../corpus/outline.json > ../../golden/outline.json
```

`IOSEVKA_BUILD_DIR` points at a scratch copy of the pinned Iosevka tree with `npm install` done.
