// Runs upstream's parameter layer over a set of shapes and writes the parameter
// sets it produces, so the Rust port of that layer can be compared against the
// real one.
//
// The layer is plain JavaScript rather than PTL, so unlike the glyph capture this
// needs no patched build: it imports the module and the tables and calls it.
//
// Usage, with a scratch copy of the pinned tree that has had `npm install` run
// in it:
//
//   IOSEVKA_SOURCE=/tmp/opencode/iosevka-src node oracle.mjs \
//     > ../../corpus/param.json

import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const source = process.env.IOSEVKA_SOURCE ?? "/tmp/opencode/iosevka-src";
const load = async (path) => (await import(pathToFileURL(path).href));

const param = await load(`${source}/packages/param/src/index.mjs`);
const tomlModule = await load(`${source}/node_modules/@iarna/toml/toml.js`);
const Toml = tomlModule.default ?? tomlModule;
const parse = (name) =>
  Toml.parse(readFileSync(`${source}/params/${name}`, "utf8"));

const data = Object.assign(
  {},
  parse("parameters.toml"),
  parse("shape-weight.toml"),
  parse("shape-width.toml"),
  parse("shape-slope.toml"),
);

// The plans a font is built from, as the shape each one names. A plan is these
// four numbers and the three switches, so this is the whole of what the layer
// reads.
// A plan names a weight and a width always, because the tables that carry them
// are only resolved when the plan does: a shape without one is not a plan a font
// is built from.
const SHAPES = [
  { weight: 400, width: 500 },
  { weight: 100, width: 500 },
  { weight: 200, width: 500 },
  { weight: 300, width: 500 },
  { weight: 500, width: 500 },
  { weight: 600, width: 500 },
  { weight: 700, width: 500 },
  { weight: 800, width: 500 },
  { weight: 900, width: 500 },
  { weight: 400, width: 720 },
  { weight: 700, width: 720 },
  { weight: 400, width: 500, slopeAngle: 15, slope: "italic" },
  { weight: 400, width: 500, slopeAngle: 15, slope: "oblique" },
  { weight: 400, width: 500, serifs: "slab" },
  { weight: 700, width: 500, serifs: "slab" },
  { weight: 400, width: 500, spacing: "quasi-proportional" },
  { weight: 400, width: 500, spacing: "term" },
  { weight: 400, width: 500, spacing: "fixed" },
  { weight: 700, width: 720, spacing: "quasi-proportional" },
  { weight: 700, width: 720, slopeAngle: 15, slope: "italic", serifs: "slab" },
];

const out = SHAPES.map((shape) => {
  const para = param.init(data, { shape, featureControl: {} });
  // The naming strings are not metrics, and a plan's naming is not this layer's
  // business, so they are dropped rather than carried.
  const metrics = {};
  for (const [key, value] of Object.entries(para)) {
    if (typeof value === "number" || typeof value === "boolean") {
      metrics[key] = value;
    }
  }

  return { shape, para: metrics };
});

process.stdout.write(`${JSON.stringify(out, null, 1)}\n`);
