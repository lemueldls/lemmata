# Pen expand oracle

Runs Iosevka's `PenSpiroExpander` and `PenSpiroGeometry` over `corpus/pen-expand.json`, so `lemmata-geom`'s pen expansion can be compared against both the raw strips and the unioned shape.

```
IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs \
  < ../../corpus/pen-expand.json > ../../golden/pen-expand.json
```

`IOSEVKA_BUILD_DIR` points at a scratch copy of the pinned Iosevka tree with `npm install` done. The cases use identity transforms.
