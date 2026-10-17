// Runs a sequence of saves through upstream's GlyphSaveSink and GlyphStore from
// corpus/glyph-store.json, then prints the store it built, so the Rust port of
// the collection path can be compared against it.
//
// Usage:
//   IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs \
//     < ../../corpus/glyph-store.json > ../../golden/glyph-store.json

import { pathToFileURL } from "node:url";

const buildDir = process.env.IOSEVKA_BUILD_DIR ?? "/tmp/opencode/iosevka-build";
const geometryBase = pathToFileURL(`${buildDir}/packages/geometry/src/`).href;

const Geom = await import(`${geometryBase}index.mjs`);
const { MonoKnot } = await import(`${geometryBase}spiro-to-outline.mjs`);
const { Transform } = await import(`${geometryBase}transform.mjs`);
const { Glyph, GlyphProc } = await import(
  pathToFileURL(`${buildDir}/packages/glyph/src/glyph.mjs`).href
);
const { GlyphBuildExecutor, GlyphSaveSink, RecursiveBuildFilter } = await import(
  pathToFileURL(`${buildDir}/packages/glyph/src/block.mjs`).href
);
const { GlyphStore } = await import(
  pathToFileURL(`${buildDir}/packages/glyph/src/store.mjs`).href
);

await Geom.Init();

const read = async () => {
  const chunks = [];
  for await (const chunk of process.stdin) chunks.push(chunk);
  return JSON.parse(Buffer.concat(chunks).toString("utf8"));
};

const transform = (spec) => {
  const t = spec ?? { xx: 1, xy: 0, yx: 0, yy: 1, tx: 0, ty: 0 };
  return new Transform(t.xx, t.xy, t.yx, t.yy, t.tx, t.ty);
};

function geometry(spec) {
  switch (spec.kind) {
    case "spiro":
      return new Geom.SpiroGeometry(
        transform(spec.gizmo),
        spec.closed,
        spec.knots.map((k) => new MonoKnot(k.type, !!k.unimportant, k.x, k.y)),
      );
    case "combine":
      return new Geom.CombineGeometry(spec.parts.map(geometry));
    default:
      throw new Error(`unknown geometry ${spec.kind}`);
  }
}

function contents(spec) {
  return new GlyphProc((target) => {
    for (const part of spec.parts ?? []) target.includeGeometry(geometry(part));
  });
}

const corpus = await read();

// The sink is built the way a glyph module builds it: a store, the metrics it
// reads, and the build filter.
const store = new GlyphStore();
const filter = corpus.needed
  ? new RecursiveBuildFilter(new Set(corpus.needed), new Set())
  : null;
const exec = new GlyphBuildExecutor(filter);
const sink = new GlyphSaveSink(
  exec,
  { Width: 500, GlobalTransform: transform(corpus.transform) },
  store,
);

const saves = [];
for (const item of corpus.saves) {
  let target;
  if (item.target === "anonymous") target = null;
  else if (item.target === "codePoint") target = item.code;
  else target = [item.name, item.unicode ?? 0];

  let saved;
  let error = null;
  try {
    saved = item.force
      ? sink.saveFalse(target?.[0] ?? target, target?.[1], contents(item))
      : sink.save(target?.[0] ?? target, target?.[1], contents(item));
  } catch (caught) {
    error = String(caught.message ?? caught);
  }
  saves.push({
    name: item.name ?? null,
    error,
    saved: saved ? { width: saved.advanceWidth, name: saved.identifier ?? null } : null,
  });
}

const serializeArcs = (shape) =>
  shape.map((contour) =>
    contour
      .filter((arc) => arc && arc.a && arc.b && arc.c && arc.d)
      .map((arc) => ({
        a: { x: arc.a.x, y: arc.a.y },
        b: { x: arc.b.x, y: arc.b.y },
        c: { x: arc.c.x, y: arc.c.y },
        d: { x: arc.d.x, y: arc.d.y },
      })),
  );

const entries = [];
for (const [name, glyph] of store.namedEntries()) {
  entries.push({
    name,
    unicodes: [...(store.queryUnicodeOf(glyph) ?? [])],
    width: glyph.advanceWidth,
    arcs: serializeArcs(glyph.geometry.toBezArcs()),
  });
}

process.stdout.write(
  JSON.stringify({ oracle: "glyph-store", saves, size: store.size, entries }, null, 2) + "\n",
);
