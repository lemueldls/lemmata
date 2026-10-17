# Fairize oracle

Runs typo-geom's `fairizeBezierShape` over `corpus/fairize.json`, so `lemmata-geom`'s fairization can be compared against it.

```
IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs \
  < ../../corpus/fairize.json > ../../golden/fairize.json
```

`IOSEVKA_BUILD_DIR` points at a scratch copy of the pinned Iosevka tree with `npm install` done.
