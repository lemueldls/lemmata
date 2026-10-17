// Builds glyphs with upstream's composition layer from corpus/glyph-compose.json
// and prints the arcs they resolve to, so the Rust port of that layer can be
// compared against it.
//
// Usage:
//   IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs \
//     < ../../corpus/glyph-compose.json > ../../golden/glyph-compose.json

import { pathToFileURL } from "node:url";

const buildDir = process.env.IOSEVKA_BUILD_DIR ?? "/tmp/opencode/iosevka-build";
const geometry = pathToFileURL(`${buildDir}/packages/geometry/src/`).href;

const Geom = await import(`${geometry}index.mjs`);
const { MonoKnot } = await import(`${geometry}spiro-to-outline.mjs`);
const { BiKnot } = await import(`${geometry}spiro-expand.mjs`);
const { Transform } = await import(`${geometry}transform.mjs`);
const { Glyph, GlyphProc } = await import(
  pathToFileURL(`${buildDir}/packages/glyph/src/glyph.mjs`).href
);
const booleKit = await import(
  pathToFileURL(`${buildDir}/packages/font-kits/src/boole-kit.mjs`).href
);

await Geom.Init();

const identity = new Transform(1, 0, 0, 1, 0, 0);
const boole = booleKit.SetupBuilders({ globalTransform: identity });

const read = async () => {
  const chunks = [];
  for await (const chunk of process.stdin) chunks.push(chunk);
  return JSON.parse(Buffer.concat(chunks).toString("utf8"));
};

const transform = (spec) => {
  const t = spec ?? { xx: 1, xy: 0, yx: 0, yy: 1, tx: 0, ty: 0 };
  return new Transform(t.xx, t.xy, t.yx, t.yy, t.tx, t.ty);
};

function buildGeometry(spec) {
  switch (spec.kind) {
    case "spiro":
      return new Geom.SpiroGeometry(
        transform(spec.gizmo),
        spec.closed,
        spec.knots.map((k) => new MonoKnot(k.type, !!k.unimportant, k.x, k.y)),
      );
    case "diSpiro":
      return new Geom.DiSpiroGeometry(
        transform(spec.gizmo),
        spec.contrast,
        spec.closed,
        spec.knots.map((k) => {
          const knot = new BiKnot(k.type, k.x, k.y, k.d1, k.d2);
          if (k.unimportant) knot.unimportant = 1;
          if (k.proposedNormal) knot.proposedNormal = { x: k.proposedNormal.x, y: k.proposedNormal.y };
          return knot;
        }),
      );
    case "combine":
      return new Geom.CombineGeometry(spec.parts.map(buildGeometry));
    case "boolean": {
      const clip = { union: "Union", intersection: "Intersection", difference: "Difference", xor: "Xor" };
      return new Geom.BooleanGeometry(clip[spec.op], spec.operands.map(buildGeometry));
    }
    case "transform":
      return new Geom.TransformedGeometry(transform(spec.tf), buildGeometry(spec.geom));
    default:
      throw new Error(`unknown geometry ${spec.kind}`);
  }
}

function applyOps(glyph, ops) {
  for (const op of ops ?? []) {
    switch (op.op) {
      case "includeGeometry":
        glyph.includeGeometry(buildGeometry(op.geom));
        break;
      case "includeProc":
        glyph.include(new GlyphProc((target) => applyOps(target, op.proc.ops)));
        break;
      case "includeGlyph":
        glyph.include(glyphFrom(op.glyph));
        break;
      case "applyTransform":
        glyph.applyTransform(transform(op.tf));
        break;
      case "setWidth":
        glyph.setWidth(op.w);
        break;
      case "ctxTag":
        glyph.ctxTag = op.tag;
        break;
      case "ejectTagged":
        glyph.ejectTagged(op.tag);
        break;
      case "boole": {
        const operator = op.operator;
        const operands = op.operands.map(
          (operand) => new GlyphProc((target) => applyOps(target, operand.ops)),
        );
        glyph.include(boole[operator](...operands));
        break;
      }
      default:
        throw new Error(`unknown op ${op.op}`);
    }
  }
}

function glyphFrom(spec) {
  const glyph = spec.identifier ? new Glyph(spec.identifier) : new Glyph();
  applyOps(glyph, spec.ops);
  return glyph;
}

const serializeArcs = (shape) =>
  shape.map((contour) =>
    contour
      .filter((arc) => arc && arc.a && arc.b && arc.c && arc.d)
      .map((arc) => ({
        a: { x: arc.a.x, y: arc.a.y },
        b: { x: arc.b.x, y: arc.b.y },
        c: { x: arc.c.x, y: arc.c.y },
        d: { x: arc.d.x, y: arc.d.y },
      })),
  );

const corpus = await read();
const out = { oracle: "glyph-compose", cases: [] };
for (const item of corpus.cases) {
  const glyph = glyphFrom(item);
  out.cases.push({
    name: item.name,
    width: glyph.advanceWidth,
    arcs: serializeArcs(glyph.geometry.toBezArcs()),
  });
}

process.stdout.write(JSON.stringify(out, null, 2) + "\n");
