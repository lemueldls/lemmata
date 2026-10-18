// The font the site is set in, read once and shared.
//
// The face is declared in `nuxt.config.ts` and built by `pnpm font`, so nothing
// here compiles anything: it fetches the file the module serves, hashes it, and
// reads the readings out of the font's own tables rather than taking them on
// trust. The generator is the only page that compiles, and it does that with the
// wasm.
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
}

const state = reactive<FontState>({
  started: false,
  digest: "",
  map: null,
  metrics: null,
  ready: false,
  error: "",
});

async function load(): Promise<void> {
  try {
    // The file the module serves, which is the one the browser was given for the
    // preload, so the bytes hashed here are the bytes being rendered with.
    const response = await fetch("/fonts/lemmata.ttf");
    const bytes = new Uint8Array(await response.arrayBuffer());
    state.digest = await sha256Hex(bytes);
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
    // The primary family, not the list: the module writes an optimized fallback
    // beside it whose source is a local family, and one face that cannot resolve
    // rejects the whole request even though this one loaded.
    await document.fonts.load('100px "Lemmata"', "lemmata");
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
