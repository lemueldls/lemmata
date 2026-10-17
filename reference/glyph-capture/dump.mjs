// Appended by lemmata: dump per-glyph geometry specs and reference contours.
//
// Enabled by LEMMATA_GLYPH_DUMP (binary output path) and LEMMATA_GLYPH_SET
// (comma-separated glyph names). The reference contours land next to the binary
// as `<path>.json`.

import { writeFileSync } from "node:fs";
import * as LemmaCurveUtil from "@iosevka/geometry/curve-util";
import { SpiroExpander as LemmaSpiroExpander } from "@iosevka/geometry/spiro-expand";
import * as LemmaSpiroJs from "spiro";
import * as LemmaTypoGeom from "typo-geom";

class LemmaWriter {
  constructor() {
    this.parts = [];
    this.length = 0;
  }
  u8(value) {
    const buffer = Buffer.alloc(1);
    buffer.writeUInt8(value >>> 0);
    this.parts.push(buffer);
    this.length += 1;
  }
  u32(value) {
    const buffer = Buffer.alloc(4);
    buffer.writeUInt32LE(value >>> 0);
    this.parts.push(buffer);
    this.length += 4;
  }
  f64(value) {
    const buffer = Buffer.alloc(8);
    buffer.writeDoubleLE(value);
    this.parts.push(buffer);
    this.length += 8;
  }
  str(value) {
    const buffer = Buffer.from(value, "utf8");
    this.u32(buffer.length);
    this.parts.push(buffer);
    this.length += buffer.length;
  }
  bytes() {
    return Buffer.concat(this.parts, this.length);
  }
}

const lemmaPoint = (p) => ({ x: p.x, y: p.y });
const lemmaTransform = (w, tf) => {
  w.f64(tf.xx);
  w.f64(tf.xy);
  w.f64(tf.yx);
  w.f64(tf.yy);
  w.f64(tf.tx);
  w.f64(tf.ty);
};
const lemmaKnotType = (w, type) => w.str(String(type));

function lemmaWriteNode(w, geom, depth) {
  if (depth > 40 || !geom) {
    w.u8(0);
    return;
  }
  if (geom instanceof Geom.ContourSetGeometry) {
    w.u8(1);
    const arcs = LemmaCurveUtil.convertShapeToArcs(geom.m_contours);
    w.u32(arcs.length);
    for (const contour of arcs) {
      w.u32(contour.length);
      for (const arc of contour) {
        for (const p of [arc.a, arc.b, arc.c, arc.d]) {
          w.f64(p.x);
          w.f64(p.y);
        }
      }
    }
    return;
  }
  if (geom instanceof Geom.SpiroGeometry) {
    w.u8(2);
    lemmaTransform(w, geom.m_gizmo);
    w.u8(geom.m_closed ? 1 : 0);
    w.u32(geom.m_knots.length);
    for (const knot of geom.m_knots) {
      lemmaKnotType(w, knot.type);
      w.f64(knot.x);
      w.f64(knot.y);
      w.u8(knot.unimportant ? 1 : 0);
    }
    return;
  }
  if (geom instanceof Geom.DiSpiroGeometry) {
    w.u8(3);
    lemmaTransform(w, geom.m_gizmo);
    w.f64(geom.m_contrast);
    w.u8(geom.m_closed ? 1 : 0);
    w.u32(geom.m_biKnots.length);
    for (const knot of geom.m_biKnots) {
      lemmaKnotType(w, knot.type);
      w.f64(knot.x);
      w.f64(knot.y);
      w.f64(knot.d1);
      w.f64(knot.d2);
      w.u8(knot.unimportant ? 1 : 0);
      if (knot.proposedNormal) {
        w.u8(1);
        w.f64(knot.proposedNormal.x);
        w.f64(knot.proposedNormal.y);
      } else {
        w.u8(0);
      }
    }
    return;
  }
  if (geom instanceof Geom.SpiroPenGeometry) {
    w.u8(4);
    lemmaTransform(w, geom.m_gizmo);
    w.u8(geom.m_closed ? 1 : 0);
    w.u32(geom.m_penProfile.length);
    for (const p of geom.m_penProfile) {
      w.f64(p.x);
      w.f64(p.y);
    }
    w.u32(geom.m_knots.length);
    for (const knot of geom.m_knots) {
      lemmaKnotType(w, knot.type);
      w.f64(knot.x);
      w.f64(knot.y);
      if (knot.profile) {
        w.u8(1);
        w.u32(knot.profile.length);
        for (const p of knot.profile) {
          w.f64(p.x);
          w.f64(p.y);
        }
      } else {
        w.u8(0);
      }
    }
    return;
  }
  if (geom instanceof Geom.StrokeGeometry) {
    w.u8(5);
    lemmaTransform(w, geom.m_gizmo);
    w.f64(geom.m_radius);
    w.f64(geom.m_contrast);
    w.u8(geom.m_fInside ? 1 : 0);
    lemmaWriteNode(w, geom.m_geom, depth + 1);
    return;
  }
  if (geom instanceof Geom.RemoveHolesGeometry) {
    w.u8(6);
    lemmaTransform(w, geom.m_gizmo);
    lemmaWriteNode(w, geom.m_geom, depth + 1);
    return;
  }
  if (geom instanceof Geom.SimplifyGeometry) {
    w.u8(7);
    lemmaTransform(w, geom.m_gizmo);
    lemmaWriteNode(w, geom.m_geom, depth + 1);
    return;
  }
  if (geom instanceof Geom.CombineGeometry) {
    w.u8(8);
    w.u32(geom.m_parts.length);
    for (const part of geom.m_parts) lemmaWriteNode(w, part, depth + 1);
    return;
  }
  if (geom instanceof Geom.BooleanGeometry) {
    w.u8(9);
    w.str(String(geom.m_operator));
    w.u32(geom.m_operands.length);
    for (const operand of geom.m_operands) lemmaWriteNode(w, operand, depth + 1);
    return;
  }
  if (geom instanceof Geom.TransformedGeometry) {
    w.u8(10);
    lemmaTransform(w, geom.m_transform);
    lemmaWriteNode(w, geom.m_geom, depth + 1);
    return;
  }
  if (geom instanceof Geom.TaggedGeometry) {
    w.u8(11);
    lemmaWriteNode(w, geom.m_geom, depth + 1);
    return;
  }
  if (geom instanceof Geom.RadicalGeometry) {
    w.u8(12);
    lemmaWriteNode(w, geom.m_geom, depth + 1);
    return;
  }
  if (geom instanceof Geom.ReferenceGeometry) {
    w.u8(13);
    lemmaWriteNode(w, geom.unwrap(), depth + 1);
    return;
  }
  w.u8(0);
}




// The fairized stage of the glyph-level conversion.
function lemmaFairized(geom, gizmo) {
  const arcs = new Geom.SimplifyGeometry(geom, gizmo).toBezArcs();
  const fairized = LemmaTypoGeom.Fairize.fairizeBezierShape(arcs);
  const p = (z) => ({ x: z.x, y: z.y });
  return fairized.map((contour) =>
    contour.map((arc) =>
      arc instanceof LemmaTypoGeom.Arcs.StraightSegment
        ? { kind: "straight", a: p(arc.a), b: p(arc.b) }
        : {
            kind: "combined",
            segments: arc.segments.map((s) => ({ a: p(s.a), b: p(s.b), c: p(s.c), d: p(s.d) })),
          },
    ),
  );
}

// The raw solver output for one side, sampled.
function lemmaRawArcs(knots, closed) {
  const arcs = [];
  const sink = {
    beginShape() {},
    endShape() {},
    moveTo() {},
    arcTo(arc) {
      const sample = (t) => {
        const z = arc.eval(t);
        return [z.x, z.y];
      };
      arcs.push([
        sample(0), sample(0.25), sample(0.5), sample(0.75), sample(1),
        [arc.deriveX0, arc.deriveY0, arc.deriveX1, arc.deriveY1, arc.arcLength, arc.bend],
      ]);
    },
  };
  LemmaSpiroJs.spiroToArcsOnContext(knots, closed, sink);
  return arcs;
}

// Pre-order walk collecting each di-spiro node's expanded sides.
function lemmaCollectDiSpiro(geom, out) {
  if (!geom) return;
  if (geom instanceof Geom.DiSpiroGeometry) {
    const expander = new LemmaSpiroExpander(
      geom.m_gizmo,
      geom.m_contrast,
      geom.m_closed,
      geom.m_biKnots,
    );
    expander.initializeNormals();
    for (let r = 0; r < 8; r++) {
      if (expander.iterateNormals() < 1e-8) break;
    }
    const expanded = expander.expand();
    const side = (knots) =>
      knots.map((k) => [k.x, k.y, String(k.type), k.unimportant ? 1 : 0]);
    const lhsU = expanded.lhsUntransformed;
    const rhsU = expanded.rhsUntransformed;
    let raw = null;
    try {
      const lhs = lhsU.map((k) => k.clone());
      const rhs = rhsU.map((k) => k.clone());
      for (const k of rhs) k.reverseType();
      rhs.reverse();
      if (geom.m_closed) {
        raw = { lhs: lemmaRawArcs(lhs, true), rhs: lemmaRawArcs(rhs, true) };
      } else {
        if (lhs.length) {
          lhs[0].type = "corner";
          lhs[lhs.length - 1].type = "corner";
        }
        if (rhs.length) {
          rhs[0].type = "corner";
          rhs[rhs.length - 1].type = "corner";
        }
        raw = { open: lemmaRawArcs(lhs.concat(rhs), true) };
      }
    } catch (_e) {
      raw = null;
    }
    out.push({
      lhs: side(lhsU),
      rhs: side(rhsU),
      raw,
    });
    return;
  }
  if (geom instanceof Geom.CombineGeometry) {
    for (const part of geom.m_parts) lemmaCollectDiSpiro(part, out);
  } else if (geom instanceof Geom.BooleanGeometry) {
    for (const operand of geom.m_operands) lemmaCollectDiSpiro(operand, out);
  } else if (geom instanceof Geom.TransformedGeometry) {
    lemmaCollectDiSpiro(geom.m_geom, out);
  } else if (geom instanceof Geom.RemoveHolesGeometry || geom instanceof Geom.SimplifyGeometry) {
    lemmaCollectDiSpiro(geom.m_geom, out);
  } else if (geom instanceof Geom.StrokeGeometry) {
    lemmaCollectDiSpiro(geom.m_geom, out);
  } else if (geom instanceof Geom.TaggedGeometry || geom instanceof Geom.RadicalGeometry) {
    lemmaCollectDiSpiro(geom.m_geom, out);
  } else if (geom instanceof Geom.ReferenceGeometry) {
    lemmaCollectDiSpiro(geom.unwrap(), out);
  }
}

function lemmataDumpGlyphs(glyphStore, para) {
  const wantedWidth = process.env.LEMMATA_GLYPH_WIDTH
    ? Number(process.env.LEMMATA_GLYPH_WIDTH)
    : null;
  if (wantedWidth !== null && Math.round(para?.width ?? NaN) !== wantedWidth) return;
  const output = process.env.LEMMATA_GLYPH_DUMP;
  const names = (process.env.LEMMATA_GLYPH_SET ?? "").split(",").filter(Boolean);
  const hasCodepoints = (process.env.LEMMATA_GLYPH_CODEPOINTS ?? "").length > 0;
  if (!output || (!names.length && !hasCodepoints)) return;

  const wanted = new Set(names);
  const codepoints = new Set(
    (process.env.LEMMATA_GLYPH_CODEPOINTS ?? "").split(",").filter(Boolean).map(Number),
  );
  writeFileSync(`${output}.names`, Array.from(glyphStore.glyphs()).map((g) => String(g._m_identifier)).join("\n"));
  const glyphs = [];
  for (const glyph of glyphStore.glyphs()) {
    const name = glyph._m_identifier;
    const unicodes = Array.from(glyphStore.queryUnicodeOf(glyph) ?? []);
    if (!wanted.has(name) && !unicodes.some((u) => codepoints.has(u))) continue;
    const contours = new Geom.SimplifyGeometry(glyph.geometry, glyph.gizmo)
      .toContours()
      .map((contour) =>
        contour.map((z) => ({ quadratic: z.type === Point.Type.Quadratic, x: z.x, y: z.y })),
      );
    let treeArcs = [];
    try {
      treeArcs = glyph.geometry.toBezArcs();
    } catch (_e) {
      treeArcs = [];
    }
    let arcs = [];
    try {
      arcs = new Geom.SimplifyGeometry(glyph.geometry, glyph.gizmo).toBezArcs();
    } catch (e) {
      if (process.env.LEMMATA_GLYPH_DEBUG) console.error("lemmata arcs threw", name, String(e));
      arcs = [];
    }
    glyphs.push({
      name,
      unicodes,
      advance: glyph.advanceWidth,
      gizmo: glyph.gizmo,
      geometry: glyph.geometry,
      contours,
      arcs,
      treeArcs,
      fairized: (() => {
        try {
          return lemmaFairized(glyph.geometry, glyph.gizmo);
        } catch (_e) {
          return [];
        }
      })(),
      expansions: (() => {
        const out = [];
        try {
          lemmaCollectDiSpiro(glyph.geometry, out);
        } catch (_e) {}
        return out;
      })(),
    });
  }

  const writer = new LemmaWriter();
  writer.u32(glyphs.length);
  for (const glyph of glyphs) {
    writer.str(glyph.name);
    writer.f64(glyph.advance);
    writer.u32(glyph.unicodes.length);
    for (const u of glyph.unicodes) writer.u32(u);
    lemmaTransform(writer, glyph.gizmo);
    lemmaWriteNode(writer, glyph.geometry, 0);
  }
  writeFileSync(output, writer.bytes());
  writeFileSync(
    `${output}.fairized.json`,
    JSON.stringify(glyphs.map((glyph) => ({ name: glyph.name, contours: glyph.fairized }))),
  );
  writeFileSync(
    `${output}.expansions.json`,
    JSON.stringify(
      glyphs.map((glyph) => ({ name: glyph.name, expansions: glyph.expansions })),
    ),
  );
  writeFileSync(
    `${output}.tree-arcs.json`,
    JSON.stringify(
      glyphs.map((glyph) => ({
        name: glyph.name,
        arcs: glyph.treeArcs.map((contour) =>
          contour.map((arc) => [arc.a, arc.b, arc.c, arc.d].map((p) => [p.x, p.y])),
        ),
      })),
    ),
  );
  writeFileSync(
    `${output}.arcs.json`,
    JSON.stringify(
      glyphs.map((glyph) => ({
        name: glyph.name,
        arcs: glyph.arcs.map((contour) =>
          contour.map((arc) => [arc.a, arc.b, arc.c, arc.d].map((p) => [p.x, p.y])),
        ),
      })),
    ),
  );
  writeFileSync(
    `${output}.json`,
    JSON.stringify(
      glyphs.map((glyph) => ({
        name: glyph.name,
        unicodes: glyph.unicodes,
        advance: glyph.advance,
        contours: glyph.contours,
      })),
    ),
  );
  console.log(`lemmata: dumped ${glyphs.length} glyphs to ${output}`);
}
