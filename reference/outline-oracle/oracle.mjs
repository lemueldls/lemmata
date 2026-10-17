// Runs Iosevka's SpiroGeometry over the skeletons in corpus/outline.json and
// prints the Bezier outline it produces.
//
// Usage:
//   IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs \
//     < ../../corpus/outline.json > ../../golden/outline.json

import { pathToFileURL } from "node:url";

const buildDir = process.env.IOSEVKA_BUILD_DIR ?? "/tmp/opencode/iosevka-build";
const geometry = pathToFileURL(`${buildDir}/packages/geometry/src/`).href;

const { SpiroGeometry } = await import(`${geometry}index.mjs`);
const { MonoKnot } = await import(`${geometry}spiro-to-outline.mjs`);
const { Transform } = await import(`${geometry}transform.mjs`);

const point = (p) => ({ x: p.x, y: p.y });
const bez3 = (arc) => ({ a: point(arc.a), b: point(arc.b), c: point(arc.c), d: point(arc.d) });

let input = "";
for await (const chunk of process.stdin) input += chunk;
const corpus = JSON.parse(input);

const out = { oracle: "iosevka v34.9.0 geometry", cases: [] };

for (const item of corpus.cases) {
  const knots = item.knots.map((k) => new MonoKnot(k.type, !!k.unimportant, k.x, k.y));
  const t = item.transform ?? { xx: 1, xy: 0, yx: 0, yy: 1, tx: 0, ty: 0 };
  const gizmo = new Transform(t.xx, t.xy, t.yx, t.yy, t.tx, t.ty);

  const shape = new SpiroGeometry(gizmo, item.closed, knots).toBezArcs();

  out.cases.push({
    name: item.name,
    contours: shape.map((contour) => contour.filter((arc) => arc.a).map(bez3)),
  });
}

process.stdout.write(JSON.stringify(out, null, 2) + "\n");
