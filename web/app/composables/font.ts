// The font the site is set in, read once and shared.
//
// The face is declared in `nuxt.config.ts` and built by the `lemmata-fonts`
// module at the start of the build, so nothing here compiles anything: it
// fetches the file the module serves, hashes it, and reads the readings out of
// the font's own tables rather than taking them on trust. The generator is the
// only page that compiles, and it does that with the wasm.
import {
  advance,
  cmap,
  glyphCount,
  ink,
  sha256Hex,
  unitsPerEm,
  type Reading,
} from "~/utils/font";

interface FontState {
  started: boolean;
  digest: string;
  map: Map<number, number> | null;
  metrics: Reading | null;
  ready: boolean;
  error: string;
  /// How long the font took to arrive and be read, in milliseconds. The site
  /// prints it, the way it prints everything else it measures.
  elapsed: number;
  /// How far along the font is, from 0 to 1. The download is the part that
  /// varies and the only part with a denominator, so it carries most of the
  /// range and the steps after it are increments.
  progress: number;
}

const state = reactive<FontState>({
  started: false,
  digest: "",
  map: null,
  metrics: null,
  ready: false,
  error: "",
  elapsed: 0,
  progress: 0.03,
});

/// Reads a response in chunks, counting them against the length the server
/// declared. A server that streams without a length leaves the meter on its
/// phase steps, which is the honest answer when the size is not known.
async function download(
  response: Response,
  upTo: number,
): Promise<Uint8Array> {
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

/// Where the family was declared, and what the page falls back to when it
/// cannot read the stylesheet.
const DECLARED = "/fonts/lemmata.ttf";

/// Where the stylesheet says a family is served from.
///
/// The fonts module treats the location of a font file as an implementation
/// detail rather than a URL contract, so the page asks the CSS instead of
/// assuming a path. The bytes hashed then are the bytes the browser is about to
/// render with, wherever the build decided to put them.
function servedUrl(family: string): string | null {
  if (!import.meta.client) return null;
  for (const sheet of document.styleSheets) {
    let rules: CSSRuleList | null = null;
    try {
      rules = sheet.cssRules;
    } catch {
      // A cross-origin sheet cannot be read. Ours is inlined in the head.
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
    // The file the module serves, which is the one the browser was given for the
    // preload, so the bytes hashed here are the bytes being rendered with.
    const url = servedUrl("Lemmata") ?? DECLARED;
    const response = await fetch(url);
    const bytes = await download(response, 0.78);
    state.progress = 0.82;

    state.digest = await sha256Hex(bytes);
    state.progress = 0.86;

    const map = cmap(bytes);
    state.map = map;
    state.metrics = {
      upem: unitsPerEm(bytes),
      drawn: glyphCount(bytes) ?? 0,
      mapped: map.size,
      xHeight: ink(map, bytes, "x")?.yMax ?? null,
      ascender: ink(map, bytes, "l")?.yMax ?? null,
      descender: ink(map, bytes, "p")?.yMin ?? null,
      advance: advance(map, bytes, "x"),
    };
    state.progress = 0.92;

    // The primary family, not the list: the module writes an optimized fallback
    // beside it whose source is a local family, and one face that cannot resolve
    // rejects the whole request even though this one loaded.
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
