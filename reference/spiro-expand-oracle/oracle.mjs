// Runs Iosevka's expansion over the synthetic skeletons in
// corpus/spiro-expand.json and prints, per case, the expanded knots and the
// Bezier strands of the knot lists that get drawn.
//
// The strands come from npm `spiro` directly, not from Iosevka's exported
// `DiSpiroGeometry`, so the comparison is against the same curve rather than
// Iosevka's Bezier fit of it.
//
// Usage:
//   IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs \
//     < ../../corpus/spiro-expand.json > ../../golden/spiro-expand.json

import { pathToFileURL } from "node:url";

const buildDir = process.env.IOSEVKA_BUILD_DIR ?? "/tmp/opencode/iosevka-build";
const geometry = pathToFileURL(`${buildDir}/packages/geometry/src/`).href;

const { BiKnot, SpiroExpander } = await import(`${geometry}spiro-expand.mjs`);
const { Transform } = await import(`${geometry}transform.mjs`);
const spiroModule = await import(pathToFileURL(`${buildDir}/node_modules/spiro/lib/index.js`).href);
const Spiro = spiroModule.spiroToBezierOnContext ? spiroModule : spiroModule.default;

/// Must match the delta the Rust test draws with.
const DELTA = 0.01;

const serializeKnot = (k) => ({ type: k.type, x: k.x, y: k.y, unimportant: !!k.unimportant });
const copyKnot = (k) => ({ type: k.type, x: k.x, y: k.y, unimportant: !!k.unimportant });

function makeKnots(item) {
  return item.knots.map((k) => {
    const bk = new BiKnot(k.type, k.x, k.y, k.d1, k.d2);
    if (k.unimportant) bk.unimportant = 1;
    if (k.proposedNormal) bk.proposedNormal = { x: k.proposedNormal.x, y: k.proposedNormal.y };
    return bk;
  });
}

function reverseMono(list) {
  const out = list.map(copyKnot);
  for (const k of out) {
    if (k.type === "left") k.type = "right";
    else if (k.type === "right") k.type = "left";
  }
  out.reverse();
  return out;
}

// Mirrors DiSpiroGeometry.toBezArcs: the knot list, or lists, that get drawn.
function drawnSides(expanded, closed) {
  const lhs = expanded.lhsUntransformed.map(copyKnot);
  const rhs = reverseMono(expanded.rhsUntransformed);
  if (closed) {
    return [lhs, rhs];
  }
  for (const list of [lhs, rhs]) {
    list[0].type = "corner";
    list[list.length - 1].type = "corner";
  }
  return [lhs.concat(rhs)];
}

function strandsFor(knots, gizmo) {
  const context = new Spiro.DefaultBezierContext();
  Spiro.spiroToBezierOnContext(knots, true, context, DELTA);
  for (const strand of context.strands) {
    for (const key of ["start", "c1", "c2", "end"]) {
      if (strand[key]) {
        const transformed = gizmo.apply(strand[key]);
        strand[key] = { x: transformed.x, y: transformed.y };
      }
    }
  }
  return context.strands;
}

let input = "";
for await (const chunk of process.stdin) input += chunk;
const corpus = JSON.parse(input);

const out = { oracle: "iosevka v34.9.0 geometry", cases: [] };

for (const item of corpus.cases) {
  const t = item.transform ?? { xx: 1, xy: 0, yx: 0, yy: 1, tx: 0, ty: 0 };
  const gizmo = new Transform(t.xx, t.xy, t.yx, t.yy, t.tx, t.ty);

  const expander = new SpiroExpander(gizmo, item.contrast, item.closed, makeKnots(item));
  expander.initializeNormals();
  for (let i = 0; i < 8; i++) {
    if (expander.iterateNormals() < 1e-8) break;
  }
  const expanded = expander.expand();

  out.cases.push({
    name: item.name,
    sides: drawnSides(expanded, item.closed).map((knots) => strandsFor(knots, gizmo)),
    expanded: {
      lhs: expanded.lhsUntransformed.map(serializeKnot),
      rhs: expanded.rhsUntransformed.map(serializeKnot),
    },
  });
}

process.stdout.write(JSON.stringify(out, null, 2) + "\n");
