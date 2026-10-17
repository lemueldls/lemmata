// Runs Iosevka's strokeArcs over the synthetic contours in
// corpus/stroke.json and prints, per case, the fitted left and right offsets and
// the unioned output.
//
// The offsets are rebuilt here from the exported pieces, because `offsetContour`
// is private to stroke.mjs. The rebuild is a line-for-line copy of it.
//
// Usage:
//   IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs \
//     < ../../corpus/stroke.json > ../../golden/stroke.json

import { pathToFileURL } from "node:url";

const buildDir = process.env.IOSEVKA_BUILD_DIR ?? "/tmp/opencode/iosevka-build";
const geometry = pathToFileURL(`${buildDir}/packages/geometry/src/`).href;

const { Init } = await import(`${geometry}index.mjs`);
const { strokeArcs } = await import(`${geometry}stroke.mjs`);
const { OffsetCurve, RoundCapCurve, OCCURRENT_PRECISION, GEOMETRY_PRECISION } = await import(
  `${geometry}curve-util.mjs`
);

const typoModule = await import(pathToFileURL(`${buildDir}/node_modules/typo-geom/lib/index.js`).href);
const TypoGeom = typoModule.default ?? typoModule;
await Init();

const { Arcs, ShapeConv } = TypoGeom;

const point = (p) => ({ x: p.x, y: p.y });

function offsetContour(arcs, distance, contrast) {
  const fReverse = distance < 0;
  const offsetArcs = [];
  let prevOffsetedArc = new OffsetCurve(arcs[arcs.length - 1], distance, contrast);
  for (let i = 0; i < arcs.length; i++) {
    const current = arcs[i];
    const currentOffsetedArc = new OffsetCurve(current, distance, contrast);

    const prevEnd = prevOffsetedArc.eval(1);
    const currentStart = currentOffsetedArc.eval(0);
    if (
      Math.abs(prevEnd.x - currentStart.x) > OCCURRENT_PRECISION ||
      Math.abs(prevEnd.y - currentStart.y) > OCCURRENT_PRECISION
    ) {
      offsetArcs.push(
        new RoundCapCurve(
          distance < 0,
          contrast,
          prevOffsetedArc.bone.eval(1),
          prevEnd,
          currentOffsetedArc.bone.eval(0),
          currentStart,
        ),
      );
    }

    offsetArcs.push(currentOffsetedArc);
    prevOffsetedArc = currentOffsetedArc;
  }

  if (fReverse) {
    offsetArcs.reverse();
    for (let i = 0; i < offsetArcs.length; i++) {
      offsetArcs[i] = new Arcs.Reverted(offsetArcs[i]);
    }
  }
  return offsetArcs;
}

const serializeShape = (shape) =>
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
  const contours = item.contours.map((contour) =>
    contour.map((arc) => new Arcs.Bez3(point(arc.a), point(arc.b), point(arc.c), point(arc.d))),
  );
  const contour = contours[0];

  const left = ShapeConv.convertShapeToBez3([offsetContour(contour, -item.radius, item.contrast)], GEOMETRY_PRECISION);
  const right = ShapeConv.convertShapeToBez3([offsetContour(contour, item.radius, item.contrast)], GEOMETRY_PRECISION);
  const unioned = strokeArcs(contours, item.radius, item.contrast, item.inside);

  out.cases.push({
    name: item.name,
    left: serializeShape(left),
    right: serializeShape(right),
    output: serializeShape(unioned),
  });
}

process.stdout.write(JSON.stringify(out, null, 2) + "\n");
