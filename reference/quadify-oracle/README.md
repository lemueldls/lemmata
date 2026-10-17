# Quadify oracle

Runs Iosevka's `QuadifySink` over `corpus/quadify.json`, so `lemmata-geom`'s quadratic conversion can be compared against it.

```
IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs \
  < ../../corpus/quadify.json > ../../golden/quadify.json
```

`IOSEVKA_BUILD_DIR` points at a scratch copy of the pinned Iosevka tree with `npm install` done.
