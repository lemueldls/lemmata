// Runs typo-geom's fairizeBezierShape over the contours in corpus/fairize.json
// and prints the fairized arcs.
//
// Usage:
//   IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs \
//     < ../../corpus/fairize.json > ../../golden/fairize.json

import { pathToFileURL } from "node:url";

const buildDir = process.env.IOSEVKA_BUILD_DIR ?? "/tmp/opencode/iosevka-build";

const typoModule = await import(pathToFileURL(`${buildDir}/node_modules/typo-geom/lib/index.js`).href);
const TypoGeom = typoModule.default ?? typoModule;
const { Arcs, Fairize } = TypoGeom;

const point = (p) => ({ x: p.x, y: p.y });
const bez3 = (arc) => ({ a: point(arc.a), b: point(arc.b), c: point(arc.c), d: point(arc.d) });

let input = "";
for await (const chunk of process.stdin) input += chunk;
const corpus = JSON.parse(input);

const out = { oracle: "iosevka v34.9.0 geometry", cases: [] };

for (const item of corpus.cases) {
  const shape = item.contours.map((contour) =>
    contour.map((arc) => new Arcs.Bez3(point(arc.a), point(arc.b), point(arc.c), point(arc.d))),
  );

  const fairized = Fairize.fairizeBezierShape(shape);

  out.cases.push({
    name: item.name,
    contours: fairized.map((contour) =>
      contour.map((arc) =>
        arc instanceof Arcs.StraightSegment
          ? { kind: "straight", a: point(arc.a), b: point(arc.b) }
          : { kind: "combined", segments: arc.segments.map(bez3) },
      ),
    ),
  });
}

process.stdout.write(JSON.stringify(out, null, 2) + "\n");
