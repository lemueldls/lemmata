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

/// A dev tool that restarts under a page can leave a request open forever. The
/// read is small, so a timeout is cheaper than a stuck status line.
const FILE_TIMEOUT = 10_000;
const FACE_TIMEOUT = 5_000;

function after<T>(work: Promise<T>, ms: number, message: string): Promise<T> {
  return Promise.race([
    work,
    new Promise<never>((_, reject) => {
      setTimeout(() => reject(new Error(message)), ms);
    }),
  ]);
}

async function fetchFont(url: string): Promise<Uint8Array> {
  const once = async (): Promise<Uint8Array> => {
    const response = await after(fetch(url), FILE_TIMEOUT, "the font file did not arrive");
    if (!response.ok) throw new Error(`the font file answered ${response.status}`);
    const buffer = await after(
      response.arrayBuffer(),
      FILE_TIMEOUT,
      "the font file stopped arriving",
    );

    return new Uint8Array(buffer);
  };

  try {
    return await once();
  } catch {
    // One retry covers a page opened while the dev tool is restarting.
    await new Promise((resolve) => setTimeout(resolve, 500));

    return once();
  }
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
    const bytes = await fetchFont(url);
    state.byteLength = bytes.length;
    state.progress = 0.6;

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
    state.progress = 0.85;

    // A face that has not painted yet should not hold the page at "loading".
    // The tables above are already read, and the CSS will swap the face in when
    // it is ready.
    await after(
      document.fonts.load('100px "Lemmata"', "lemmata"),
      FACE_TIMEOUT,
      "the face took too long to paint",
    ).catch(() => {});

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
