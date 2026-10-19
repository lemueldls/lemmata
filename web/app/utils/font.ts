// Readers for the font the site loads.
//
// Nothing here measures with a canvas. A character the font lacks would be
// handed to a fallback face, and the fallback would answer with its own numbers.
// Every value below comes from the font's own tables.

/// The ink box of one glyph, as `glyf` stores it in the glyph header.
export interface Ink {
  xMin: number;
  yMin: number;
  xMax: number;
  yMax: number;
}

/// What the font's tables answer. `null` is a question the font does not
/// answer, which is different from a zero.
export interface Reading {
  upem: number | null;
  drawn: number;
  mapped: number;
  xHeight: number | null;
  ascender: number | null;
  descender: number | null;
  advance: number | null;
}

const viewOf = (bytes: Uint8Array): DataView =>
  new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);

const tag = (name: string): number =>
  name
    .split("")
    .reduce((value, character) => (value << 8) | character.charCodeAt(0), 0) >>>
  0;

/// The offset of a table, from the directory at the front of the font.
function table(view: DataView, name: string): number | null {
  const count = view.getUint16(4);

  for (let index = 0; index < count; index += 1) {
    const record = 12 + index * 16;
    if (view.getUint32(record) === tag(name)) return view.getUint32(record + 8);
  }

  return null;
}

/// The number of glyphs the font holds, including every derived glyph.
export function glyphCount(bytes: Uint8Array): number | null {
  const view = viewOf(bytes);
  const offset = table(view, "maxp");

  return offset === null ? null : view.getUint16(offset + 4);
}

/// The em the font is drawn at.
export function unitsPerEm(bytes: Uint8Array): number | null {
  const view = viewOf(bytes);
  const offset = table(view, "head");

  return offset === null ? null : view.getUint16(offset + 18);
}

/// Which character maps to which glyph, out of the format 4 subtable. Every font
/// this project writes has one, and it is the only place that says what the font
/// can set.
export function cmap(bytes: Uint8Array): Map<number, number> {
  const view = viewOf(bytes);
  const map = new Map<number, number>();
  const directory = table(view, "cmap");

  if (directory === null) return map;

  const records = view.getUint16(directory + 2);
  let subtable: number | null = null;

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
    if (platform === 3 || platform === 0) subtable ??= at;
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
      let glyph: number;
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

/// Which characters a page should show. The private-use block holds the derived
/// size variants, which are not characters a reader types.
export function isPublic(code: number): boolean {
  return (
    code >= 0x20 &&
    code !== 0x7f &&
    !(code >= 0xe000 && code <= 0xf8ff)
  );
}

/// The ascender and descender from `hhea`. These are the line metrics, not the
/// ink of one letter.
export function verticalMetrics(bytes: Uint8Array): {
  ascender: number | null;
  descender: number | null;
} {
  const view = viewOf(bytes);
  const hhea = table(view, "hhea");

  if (hhea === null) return { ascender: null, descender: null };

  return {
    ascender: view.getInt16(hhea + 4),
    descender: view.getInt16(hhea + 6),
  };
}

/// The offset of one glyph's outline in `glyf`, out of `loca`.
function outline(view: DataView, id: number): number | null {
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

/// The ink extents of one character. A glyph with no outline, like the space,
/// has none.
export function ink(
  map: Map<number, number>,
  bytes: Uint8Array,
  character: string,
): Ink | null {
  const code = character.codePointAt(0);
  const id = code === undefined ? undefined : map.get(code);
  if (id === undefined) return null;
  const view = viewOf(bytes);
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
export function advance(
  map: Map<number, number>,
  bytes: Uint8Array,
  character: string,
): number | null {
  const code = character.codePointAt(0);
  const id = code === undefined ? undefined : map.get(code);
  const view = viewOf(bytes);
  const hmtx = table(view, "hmtx");
  const hhea = table(view, "hhea");
  if (id === undefined || hmtx === null || hhea === null) return null;
  const metered = view.getUint16(hhea + 34);

  return view.getUint16(hmtx + Math.min(id, metered - 1) * 4);
}

/// Which characters of a string the font does not have. The count is of the
/// distinct characters, since a font either has a character or does not.
export function coverage(
  map: Map<number, number>,
  text: string,
): { total: number; missing: string[] } {
  const unique = [...new Set([...text])].filter(
    (character) => !/\s/.test(character),
  );
  const missing = unique.filter((character) => {
    const code = character.codePointAt(0);

    return code === undefined || !map.has(code);
  });

  return { total: unique.length, missing };
}
