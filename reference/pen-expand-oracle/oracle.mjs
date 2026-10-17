// Runs Iosevka's PenSpiroExpander over the synthetic profiles in
// corpus/pen-expand.json and prints, per case, the raw strips and the unioned
// Bezier output.
//
// Usage:
//   IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs \
//     < ../../corpus/pen-expand.json > ../../golden/pen-expand.json

import { pathToFileURL } from "node:url";

const buildDir = process.env.IOSEVKA_BUILD_DIR ?? "/tmp/opencode/iosevka-build";
const geometry = pathToFileURL(`${buildDir}/packages/geometry/src/`).href;

const { PenSpiroExpander, PenKnot } = await import(`${geometry}spiro-pen-expand.mjs`);
const { Init, SpiroPenGeometry } = await import(`${geometry}index.mjs`);
const { Transform } = await import(`${geometry}transform.mjs`);

await Init();

const point = (p) => ({ x: p.x, y: p.y });
const serializeShape = (shape) =>
  shape.map((contour) =>
    contour
      .filter((arc) => arc && arc.a && arc.b && arc.c && arc.d)
      .map((arc) => ({ a: point(arc.a), b: point(arc.b), c: point(arc.c), d: point(arc.d) })),
  );

function makeKnots(item) {
  return item.knots.map((k) => {
    const profile = k.profile ? k.profile.map((p) => ({ x: p.x, y: p.y })) : null;
    return new PenKnot(k.type, k.x, k.y, profile);
  });
}

let input = "";
for await (const chunk of process.stdin) input += chunk;
const corpus = JSON.parse(input);

const out = { oracle: "iosevka v34.9.0 geometry", cases: [] };

for (const item of corpus.cases) {
  const gizmo = Transform.Id();
  const profile = item.profile.map((p) => ({ x: p.x, y: p.y }));

  const expander = new PenSpiroExpander(gizmo, profile, item.closed, makeKnots(item));
  const raw = expander.getGeometry();

  const unioned = new SpiroPenGeometry(gizmo, profile, item.closed, makeKnots(item)).toBezArcs();

  out.cases.push({
    name: item.name,
    raw: raw.map((contour) => contour.map((p) => ({ x: p.x, y: p.y }))),
    output: serializeShape(unioned),
  });
}

process.stdout.write(JSON.stringify(out, null, 2) + "\n");
