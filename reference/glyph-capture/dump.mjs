// Appended by lemmata: dump per-glyph geometry specs and reference contours.
//
// Enabled by LEMMATA_GLYPH_DUMP (binary output path). Selection is by
// LEMMATA_GLYPH_SET (glyph names) or LEMMATA_GLYPH_CODEPOINTS, and
// LEMMATA_GLYPH_WIDTH pins the advance width. The reference contours and arcs
// land next to the binary as `<path>.json` and `<path>.arcs.json`.
//
// This is appended to packages/font/src/otd-conv/geometry.mjs, so `Geom` and
// `Point` come from that file's imports.

import { writeFileSync } from "node:fs";
import * as LemmaCurveUtil from "@iosevka/geometry/curve-util";

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
