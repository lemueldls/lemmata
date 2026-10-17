# Spiro oracle

The npm `spiro` package (3.0.1) is the implementation upstream uses, so its output is the reference for `lemmata-geom`'s port.

```
npm install
node oracle.mjs < ../../corpus/spiro-knots.json > ../../golden/spiro-cases.json
```

Run the regeneration after any change to the corpus, and keep the corpus and goldens in the same commit.
