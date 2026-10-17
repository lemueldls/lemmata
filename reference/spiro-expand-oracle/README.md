# Spiro expand oracle

Runs Iosevka's own `DiSpiroGeometry` over the skeletons in `corpus/spiro-expand.json`, so `lemmata-geom`'s contrast expander can be compared against it.

```
IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs \
  < ../../corpus/spiro-expand.json > ../../golden/spiro-expand.json
```

`IOSEVKA_BUILD_DIR` points at a scratch copy of the pinned Iosevka tree with `npm install` done. The cases use identity transforms apart from one translated square; transforms are supported by the port but not exercised broadly yet.
