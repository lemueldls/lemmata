// Runs Iosevka's QuadifySink over the contours in corpus/quadify.json and prints
// the quadratic contours it produces.
//
// Usage:
//   IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs \
//     < ../../corpus/quadify.json > ../../golden/quadify.json

import { pathToFileURL } from "node:url";

const buildDir = process.env.IOSEVKA_BUILD_DIR ?? "/tmp/opencode/iosevka-build";
const geometry = pathToFileURL(`${buildDir}/packages/geometry/src/`).href;

const { QuadifySink } = await import(`${geometry}quadify.mjs`);
const { Point } = await import(`${geometry}point.mjs`);

const typoModule = await import(pathToFileURL(`${buildDir}/node_modules/typo-geom/lib/index.js`).href);
const TypoGeom = typoModule.default ?? typoModule;
const { Arcs, ShapeConv } = TypoGeom;

// `SimplifyGeometry.toContours` passes GEOMETRY_PRECISION as the tolerance.
const GEOMETRY_PRECISION = 1 / 4;

const point = (p) => ({ x: p.x, y: p.y });

let input = "";
for await (const chunk of process.stdin) input += chunk;
const corpus = JSON.parse(input);

const out = { oracle: "iosevka v34.9.0 geometry", cases: [] };

for (const item of corpus.cases) {
  const shape = item.contours.map((contour) =>
    contour.map((arc) => new Arcs.Bez3(point(arc.a), point(arc.b), point(arc.c), point(arc.d))),
  );

  const sink = new QuadifySink();
  ShapeConv.transferGenericShape(shape, sink, GEOMETRY_PRECISION);

  out.cases.push({
    name: item.name,
    contours: sink.contours.map((contour) =>
      contour.map((p) => ({
        quadratic: p.type === Point.Type.Quadratic,
        x: p.x,
        y: p.y,
      })),
    ),
  });
}

process.stdout.write(JSON.stringify(out, null, 2) + "\n");
