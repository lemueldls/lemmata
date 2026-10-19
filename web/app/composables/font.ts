// The face the site is set in, read once and shared.
//
// The file is declared in `nuxt.config.ts` and built by the fonts module at the
// start of the site build. This fetches that file and reads its tables: `maxp`
// for the glyph count, `head` for the em, `cmap` for the characters, `glyf` for
// ink and `hmtx` for the advance.
import {
  advance,
  cmap,
  glyphCount,
  ink,
  isPublic,
  unitsPerEm,
  verticalMetrics,
  type Reading,
} from "~/utils/font";

interface FontState {
  started: boolean;
  map: Map<number, number> | null;
  metrics: Reading | null;
  ready: boolean;
  error: string;
  /// The size of the file the page loaded, in bytes.
  byteLength: number;
  /// How long the file took to arrive and be read, in milliseconds.
  elapsed: number;
  /// Download progress, from 0 to 1.
  progress: number;
}

const state = reactive<FontState>({
  started: false,
  map: null,
  metrics: null,
  ready: false,
  error: "",
  byteLength: 0,
  elapsed: 0,
  progress: 0.03,
});

/// Reads a response in chunks, counting them from the declared length.
async function download(response: Response, upTo: number): Promise<Uint8Array> {
  const declared = Number(response.headers.get("content-length") ?? 0);
  const reader = response.body?.getReader();

  if (!reader) return new Uint8Array(await response.arrayBuffer());

  const chunks: Uint8Array[] = [];
  let seen = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value);
    seen += value.length;
    if (declared > 0) state.progress = Math.min(upTo, (seen / declared) * upTo);
  }

  const bytes = new Uint8Array(seen);
  let at = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, at);
    at += chunk.length;
  }

  return bytes;
}

const DECLARED = "/fonts/lemmata.ttf";

/// The location the stylesheet gives a family.
///
/// The fonts module treats the file location as an implementation detail, so the
/// page asks the CSS rather than guessing a path.
function servedUrl(family: string): string | null {
  if (!import.meta.client) return null;
  for (const sheet of document.styleSheets) {
    let rules: CSSRuleList | null = null;
    try {
      rules = sheet.cssRules;
    } catch {
      continue;
    }
    for (const rule of rules ?? []) {
      if (!(rule instanceof CSSFontFaceRule)) continue;
      const name = rule.style.fontFamily.replace(/['"]/g, "");
      if (name !== family) continue;
      const match = /url\((['"]?)([^'")]+)\1\)/.exec(
        rule.style.getPropertyValue("src"),
      );
      if (match?.[2]) return match[2];
    }
  }

  return null;
}

async function load(): Promise<void> {
  const started = performance.now();
  try {
    const url = servedUrl("Lemmata") ?? DECLARED;
    const response = await fetch(url);
    const bytes = await download(response, 0.8);
    state.byteLength = bytes.length;
    state.progress = 0.84;

    const map = cmap(bytes);
    const vertical = verticalMetrics(bytes);
    state.map = map;
    state.metrics = {
      upem: unitsPerEm(bytes),
      drawn: glyphCount(bytes) ?? 0,
      mapped: [...map.keys()].filter(isPublic).length,
      xHeight: ink(map, bytes, "x")?.yMax ?? null,
      ascender: vertical.ascender,
      descender: vertical.descender,
      advance: advance(map, bytes, "x"),
    };
    state.progress = 0.93;

    await document.fonts.load('100px "Lemmata"', "lemmata");
    state.progress = 1;
    state.elapsed = performance.now() - started;
    state.ready = true;
  } catch (error) {
    state.error = error instanceof Error ? error.message : String(error);
  }
}

export function useLemmataFont(): FontState {
  onMounted(() => {
    if (state.started) return;
    state.started = true;
    void load();
  });

  return state;
}
