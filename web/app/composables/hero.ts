// A hero word that redraws itself from the plan.
//
// The ported letters carry variants, and a plan selects one by moving its
// drawing onto the letter a codepoint maps to. So the word is rebuilt as a small
// preview font on a timer, rather than switched between glyphs in a fixed file:
// `preview_font` recomputes only the letters the word draws, which is a few
// milliseconds.
//
// With the capture the site ships, `a` is the letter with variants. The loop is
// written over the word's letters rather than over `a`, so a capture that grows
// more families lights the rest up on its own.
import { preview_font, variants_of } from "lemmata-web";

import { usePlan } from "~/composables/plan";

const WORD = "lemmata";
const PERIOD = 3200;
const FALLBACK = "live from the font";

interface Choice {
  group: string;
  variant: string;
}

/// The variant choices the capture holds for the letters of the word.
function choices(capture: Uint8Array | null): Choice[] {
  if (!capture) return [];
  const out: Choice[] = [];
  for (const group of new Set(WORD)) {
    let names: string[] = [];
    try {
      names = JSON.parse(variants_of(capture, group)) as string[];
    } catch {
      continue;
    }
    for (const variant of names) out.push({ group, variant });
  }

  return out;
}

export function useHeroVariants() {
  const { capture, ensure } = usePlan();
  const family = ref('"Lemmata", var(--built)');
  const label = ref(FALLBACK);

  let face: FontFace | null = null;
  let timer: ReturnType<typeof setInterval> | null = null;
  let busy = false;
  let last = "";

  async function tick(): Promise<void> {
    const bytes = capture.value;
    if (!bytes || busy) return;
    const options = choices(bytes);
    if (!options.length) return;
    // Never the same drawing twice running, so the change reads as a cycle
    // rather than a flicker.
    const pool = options.filter(
      (option) => `${option.group}/${option.variant}` !== last,
    );
    const pick =
      pool[Math.floor(Math.random() * pool.length)] ?? options[0];
    if (!pick) return;

    busy = true;
    try {
      const plan = JSON.stringify({
        variants: { [pick.group]: pick.variant },
      });
      const font = preview_font(bytes, plan, WORD);
      if (!font.length) return;
      if (face) document.fonts.delete(face);
      face = new FontFace("Lemmata Hero", font.buffer as ArrayBuffer);
      await face.load();
      document.fonts.add(face);
      family.value = '"Lemmata Hero", var(--built)';
      label.value = `${pick.group} · ${pick.variant}`;
      last = `${pick.group}/${pick.variant}`;
    } catch {
      // The last good drawing stays on screen.
    } finally {
      busy = false;
    }
  }

  onMounted(async () => {
    // A word that redraws itself is motion. Honour the setting that asks for
    // none, and leave the site face in place.
    if (matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    await ensure();
    if (!choices(capture.value).length) return;
    void tick();
    timer = setInterval(() => void tick(), PERIOD);
  });

  onBeforeUnmount(() => {
    if (timer) clearInterval(timer);
    if (face) document.fonts.delete(face);
  });

  return { family, label };
}
