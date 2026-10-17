// Builds Iosevka's spiro control sequences from corpus/spiro-kit.json and prints
// the knots the collector ends up with, so the Rust port of the control layer
// and the builders can be compared against the real ones.
//
// Usage:
//   IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs \
//     < ../../corpus/spiro-kit.json > ../../golden/spiro-kit.json

import { pathToFileURL } from "node:url";

const buildDir = process.env.IOSEVKA_BUILD_DIR ?? "/tmp/opencode/iosevka-build";
const geometry = pathToFileURL(`${buildDir}/packages/geometry/src/`).href;

const control = await import(`${geometry}spiro-control.mjs`);
const { BiKnotCollector } = await import(`${geometry}spiro-expand.mjs`);
const kits = await import(pathToFileURL(`${buildDir}/packages/font-kits/src/spiro-kit.mjs`).href);
const { Transform } = await import(`${geometry}transform.mjs`);

const read = async () => {
  const chunks = [];
  for await (const chunk of process.stdin) chunks.push(chunk);
  return JSON.parse(Buffer.concat(chunks).toString("utf8"));
};

const corpus = await read();
const out = { oracle: "spiro-kit", cases: [] };

const point = (spec) => ({ x: spec.x, y: spec.y });

for (const item of corpus.cases) {
  const bindings = {
    Stroke: item.stroke,
    Superness: item.superness ?? 1,
    Contrast: item.contrast,
    TINY: item.tiny ?? 0.125,
    GlobalTransform: new Transform(1, 0, 0, 1, 0, 0),
  };
  const kit = kits.SetupBuilders(bindings);

  const amendment = (spec) => {
    if (!spec) return undefined;
    switch (spec.kind) {
      case "widths":
        return kit.widths(spec.l, spec.r);
      case "widths.lhs":
        return kit.widths.lhs(spec.w);
      case "widths.rhs":
        return kit.widths.rhs(spec.w);
      case "widths.center":
        return kit.widths.center(spec.w);
      case "widths.heading":
        return kit.widths.heading(spec.l, spec.r, point(spec));
      case "widths.lhs.heading":
        return kit.widths.lhs.heading(spec.w, point(spec));
      case "widths.rhs.heading":
        return kit.widths.rhs.heading(spec.w, point(spec));
      case "widths.center.heading":
        return kit.widths.center.heading(spec.w, point(spec));
      case "heading":
        return kit.heading(point(spec));
      case "unimportant":
        return kit.unimportant;
      case "important":
        return kit.important;
      case "disable-contrast":
        return kit["disable-contrast"]();
      default:
        throw new Error(`unknown amendment ${spec.kind}`);
    }
  };

  const build = (spec) => {
    const f = amendment(spec.af);
    switch (spec.kind) {
      case "g4":
      case "g2":
      case "corner":
      case "flat":
      case "curl":
        return kit[spec.kind](spec.x, spec.y, f);
      case "virt":
        return kit.virt(spec.x, spec.y, f);
      case "close":
        return kit.close(f);
      case "end":
        return kit.end(f);
      case "archv":
        return kit.archv(spec.samples, spec.superness);
      case "arcvh":
        return kit.arcvh(spec.samples, spec.superness);
      case "alsoThru":
        return kit.alsoThru(spec.rx, spec.ry, f);
      case "alsoThruThem": {
        const entries = spec.rs.map(([rx, ry, rt]) => [rx, ry, rt]);
        return kit.alsoThruThem(entries, f, kit[spec.ty ?? "g2"]);
      }
      case "decor":
        return kit["decor@"](spec.items.map(build));
      default: {
        // Directional builders, e.g. "g4.up" or "g4.right.start".
        const [base, direction, position] = spec.kind.split(".");
        const builder = kit[base]?.[direction];
        if (!builder) throw new Error(`unknown builder ${spec.kind}`);
        return position ? builder[position](spec.x, spec.y, f) : builder(spec.x, spec.y, f);
      }
    }
  };

  const flattener = new control.SpiroFlattener();
  for (const spec of item.controls) flattener.add(build(spec));
  flattener.flatten();

  const collector = new BiKnotCollector(item.contrast);
  flattener.pipe(collector);

  out.cases.push({
    name: item.name,
    closed: collector.closed,
    contrast: collector.contrast,
    knots: collector.knots.map((knot) => ({
      type: knot.type,
      x: knot.x,
      y: knot.y,
      d1: knot.d1,
      d2: knot.d2,
      unimportant: knot.unimportant,
      proposedNormal: knot.proposedNormal
        ? { x: knot.proposedNormal.x, y: knot.proposedNormal.y }
        : null,
    })),
  });
}

process.stdout.write(JSON.stringify(out, null, 2) + "\n");
