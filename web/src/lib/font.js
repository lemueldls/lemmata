// Reads the font a page has just built.
//
// Nothing here measures with a canvas. A character the font lacks is measured
// through a fallback, and the fallback reports its own number, which is how a
// page ends up printing Arial's cap height for a font it built. So every value
// comes from the font's own tables: `maxp` for the glyph count, `head` for the
// units per em, `cmap` for the characters, `glyf` for the ink, `hmtx` for the
// advance.

const viewOf = (bytes) =>
  new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);

const tag = (name) =>
  name
    .split("")
    .reduce((value, character) => (value << 8) | character.charCodeAt(0), 0) >>>
  0;

/// The offset of a table, from the directory at the front of the font.
function table(view, name) {
  const count = view.getUint16(4);

  for (let index = 0; index < count; index += 1) {
    const record = 12 + index * 16;
    if (view.getUint32(record) === tag(name)) return view.getUint32(record + 8);
  }

  return null;
}

/// The number of glyphs the font carries, which is the count after every
/// derived glyph has been added.
export function glyphCount(bytes) {
  const view = viewOf(bytes);
  const offset = table(view, "maxp");

  return offset === null ? null : view.getUint16(offset + 4);
}

/// The em the font is drawn at.
export function unitsPerEm(bytes) {
  const view = viewOf(bytes);
  const offset = table(view, "head");

  return offset === null ? null : view.getUint16(offset + 18);
}

/// Which character maps to which glyph, out of the format 4 subtable. Every
/// font this project writes carries one, and it is the only place that says
/// what the font can set, as opposed to what a browser will substitute.
export function cmap(bytes) {
  const view = viewOf(bytes);
  const map = new Map();
  const directory = table(view, "cmap");

  if (directory === null) return map;

  const records = view.getUint16(directory + 2);
  let subtable = null;

  for (let index = 0; index < records; index += 1) {
    const record = directory + 4 + index * 8;
    const platform = view.getUint16(record);
    const encoding = view.getUint16(record + 2);
    const at = directory + view.getUint32(record + 4);
    if (view.getUint16(at) !== 4) continue;
    if (platform === 3 && encoding === 10) {
      subtable = at;
      break;
    }
    if (platform === 3 || platform === 0) subtable = subtable ?? at;
  }

  if (subtable === null) return map;

  const segments = view.getUint16(subtable + 6) / 2;
  const ends = subtable + 14;
  const starts = ends + segments * 2 + 2;
  const deltas = starts + segments * 2;
  const ranges = deltas + segments * 2;

  for (let segment = 0; segment < segments; segment += 1) {
    const end = view.getUint16(ends + segment * 2);
    const start = view.getUint16(starts + segment * 2);
    const delta = view.getInt16(deltas + segment * 2);
    const range = view.getUint16(ranges + segment * 2);
    for (let code = start; code <= end && code !== 0xffff; code += 1) {
      let glyph;
      if (range === 0) {
        glyph = (code + delta) & 0xffff;
      } else {
        const at = ranges + segment * 2 + range + (code - start) * 2;
        glyph = at + 1 < view.byteLength ? view.getUint16(at) : 0;
        if (glyph !== 0) glyph = (glyph + delta) & 0xffff;
      }
      if (glyph !== 0) map.set(code, glyph);
    }
  }

  return map;
}

/// The byte range of one glyph in `glyf`, out of `loca`.
function outline(view, id) {
  const loca = table(view, "loca");
  const glyf = table(view, "glyf");
  const head = table(view, "head");

  if (loca === null || glyf === null || head === null) return null;

  const long = view.getInt16(head + 50) === 1;
  const at = long ? loca + id * 4 : loca + id * 2;
  const start = long ? view.getUint32(at) : view.getUint16(at) * 2;
  const end = long ? view.getUint32(at + 4) : view.getUint16(at + 2) * 2;

  return start === end ? null : glyf + start;
}

/// The ink extents of one character, from the bounding box `glyf` stores in
/// the glyph's header. A glyph with no outline, like the space, has none.
export function ink(map, bytes, character) {
  const view = viewOf(bytes);
  const id = map.get(character.codePointAt(0));
  if (id === undefined) return null;
  const at = outline(view, id);

  return at === null
    ? null
    : {
        xMin: view.getInt16(at + 2),
        yMin: view.getInt16(at + 4),
        xMax: view.getInt16(at + 6),
        yMax: view.getInt16(at + 8),
      };
}

/// The advance of one character, from `hmtx`.
export function advance(map, bytes, character) {
  const view = viewOf(bytes);
  const id = map.get(character.codePointAt(0));
  const hmtx = table(view, "hmtx");
  const hhea = table(view, "hhea");
  if (id === undefined || hmtx === null || hhea === null) return null;
  const metered = view.getUint16(hhea + 34);

  return view.getUint16(hmtx + Math.min(id, metered - 1) * 4);
}

/// Which characters of a string the font does not carry. The count is of the
/// distinct characters, since a font either has a character or does not.
export function coverage(map, text) {
  const unique = [...new Set([...text])].filter(
    (character) => !/\s/.test(character),
  );
  const missing = unique.filter(
    (character) => !map.has(character.codePointAt(0)),
  );

  return { total: unique.length, missing };
}

export async function sha256Hex(bytes) {
  const digest = await crypto.subtle.digest("SHA-256", bytes);

  return [...new Uint8Array(digest)]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

export const short = (hash, length = 16) => `${hash.slice(0, length)}…`;

/// Registers a built font under a family name and hands the face back.
export async function registerFace(bytes, family) {
  const face = new FontFace(family, bytes.buffer);

  await face.load();
  document.fonts.add(face);

  return face;
}
