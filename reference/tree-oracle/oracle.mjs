// Builds Iosevka geometry trees from corpus/tree.json and prints their output.
//
// Each case names a tree node ("spiro", "diSpiro", "spiroPen", "stroke",
// "removeHoles", "simplify", "combine", "boolean", "transform") and whether to
// read "arcs" (toBezArcs) or "contours" (toContours).
//
// Usage:
//   IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs \
//     < ../../corpus/tree.json > ../../golden/tree.json

import { pathToFileURL } from "node:url";

const buildDir = process.env.IOSEVKA_BUILD_DIR ?? "/tmp/opencode/iosevka-build";
const geometry = pathToFileURL(`${buildDir}/packages/geometry/src/`).href;

const index = await import(`${geometry}index.mjs`);
const { MonoKnot } = await import(`${geometry}spiro-to-outline.mjs`);
const { BiKnot } = await import(`${geometry}spiro-expand.mjs`);
const { PenKnot } = await import(`${geometry}spiro-pen-expand.mjs`);
const { Transform } = await import(`${geometry}transform.mjs`);
const { Point } = await import(`${geometry}point.mjs`);
const typoModule = await import(pathToFileURL(`${buildDir}/node_modules/typo-geom/lib/index.js`).href);
const TypoGeom = typoModule.default ?? typoModule;

await index.Init();

const CLIP_TYPES = { union: "Union", intersection: "Intersection", difference: "Difference", xor: "Xor" };

const point = (p) => ({ x: p.x, y: p.y });
const transform = (spec) => {
  const t = spec ?? { xx: 1, xy: 0, yx: 0, yy: 1, tx: 0, ty: 0 };
  return new Transform(t.xx, t.xy, t.yx, t.yy, t.tx, t.ty);
};

function build(spec) {
  switch (spec.kind) {
    case "spiro":
      return new index.SpiroGeometry(
        transform(spec.gizmo),
        spec.closed,
        spec.knots.map((k) => new MonoKnot(k.type, !!k.unimportant, k.x, k.y)),
      );
    case "diSpiro":
      return new index.DiSpiroGeometry(
        transform(spec.gizmo),
        spec.contrast,
        spec.closed,
        spec.knots.map((k) => {
          const bk = new BiKnot(k.type, k.x, k.y, k.d1, k.d2);
          if (k.unimportant) bk.unimportant = 1;
          if (k.proposedNormal) bk.proposedNormal = { x: k.proposedNormal.x, y: k.proposedNormal.y };
          return bk;
        }),
      );
    case "spiroPen":
      return new index.SpiroPenGeometry(
        transform(spec.gizmo),
        spec.profile.map((p) => ({ x: p.x, y: p.y })),
        spec.closed,
        spec.knots.map((k) => {
          const profile = k.profile ? k.profile.map((p) => ({ x: p.x, y: p.y })) : null;
          return new PenKnot(k.type, k.x, k.y, profile);
        }),
      );
    case "stroke":
      return new index.StrokeGeometry(
        build(spec.geom),
        transform(spec.gizmo),
        spec.radius,
        spec.contrast,
        spec.inside,
      );
    case "removeHoles":
      return new index.RemoveHolesGeometry(build(spec.geom), transform(spec.gizmo));
    case "simplify":
      return new index.SimplifyGeometry(build(spec.geom), transform(spec.gizmo));
    case "combine":
      return new index.CombineGeometry(spec.parts.map(build));
    case "boolean":
      return new index.BooleanGeometry(
        TypoGeom.Boolean.ClipType[CLIP_TYPES[spec.op]],
        spec.operands.map(build),
      );
    case "transform":
      return new index.TransformedGeometry(transform(spec.tf), build(spec.geom));
    default:
      throw new Error(`unknown kind ${spec.kind}`);
  }
}

const serializeArcs = (shape) =>
  shape.map((contour) =>
    contour
      .filter((arc) => arc && arc.a && arc.b && arc.c && arc.d)
      .map((arc) => ({ a: point(arc.a), b: point(arc.b), c: point(arc.c), d: point(arc.d) })),
  );

let input = "";
for await (const chunk of process.stdin) input += chunk;
const corpus = JSON.parse(input);

const out = { oracle: "iosevka v34.9.0 geometry", cases: [] };

for (const item of corpus.cases) {
  const node = build(item.geom);
  if (item.output === "contours") {
    out.cases.push({
      name: item.name,
      contours: node
        .toContours()
        .map((contour) =>
          contour.map((p) => ({ quadratic: p.type === Point.Type.Quadratic, x: p.x, y: p.y })),
        ),
    });
  } else {
    out.cases.push({ name: item.name, arcs: serializeArcs(node.toBezArcs()) });
  }
}

process.stdout.write(JSON.stringify(out, null, 2) + "\n");
