# Tree oracle

Builds Iosevka geometry trees from `corpus/tree.json` and prints `toBezArcs` or `toContours`, so `lemmata-geom`'s tree classes can be compared against the real ones.

```
IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs \
  < ../../corpus/tree.json > ../../golden/tree.json
```

`IOSEVKA_BUILD_DIR` points at a scratch copy of the pinned Iosevka tree with `npm install` done.
