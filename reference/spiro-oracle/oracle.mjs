// Reads a knot corpus on stdin, prints the spiro strands as JSON on stdout.
//
// Usage: node oracle.mjs < corpus/spiro-knots.json > golden/spiro-cases.json
//
// The corpus carries the same delta Iosevka passes (GEOMETRY_PRECISION, 1/4),
// so the strand subdivision matches the build pipeline.

import { DefaultBezierContext, spiroToBezierOnContext } from "spiro";
import { readFileSync } from "node:fs";

const manifest = JSON.parse(readFileSync(new URL("./package.json", import.meta.url)));

let input = "";
for await (const chunk of process.stdin) input += chunk;

const corpus = JSON.parse(input);
const out = { oracle: { spiro: manifest.dependencies.spiro }, cases: [] };

for (const item of corpus.cases) {
  const context = new DefaultBezierContext();
  spiroToBezierOnContext(item.knots, item.closed, context, item.delta);
  out.cases.push({ name: item.name, strands: context.strands });
}

process.stdout.write(JSON.stringify(out, null, 2) + "\n");
