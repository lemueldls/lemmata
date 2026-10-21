// A hero word that redraws itself from the plan.
//
// The ported letters carry variants, and a plan selects one by moving its
// drawing onto the letter a codepoint maps to. So the word is rebuilt as a small
// preview font on a timer, rather than switched between glyphs in a fixed file:
// `preview_font` recomputes only the letters the word draws, which is a few
// milliseconds.
//
// The choices come from the registry rather than from the capture. A capture
// holds the drawings one build produced, which is the `a` family; the registry
// holds everything the port can draw, so `l` and `m` join in as soon as their
// definitions land.
import { preview_font, registry_names } from "lemmata-web";

import { usePlan } from "~/composables/plan";

const WORD = "lemmata";
const PERIOD = 3200;
const FALLBACK = "live from the font";

interface Choice {
  group: string;
  variant: string;
}

/// `doubleStoreySerifless` in the kebab case a plan writes.
function kebab(name: string): string {
  return name.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);
}

/// The variant choices the port draws for the letters of the word.
function choices(): Choice[] {
  const names = registry_names().split("\n");
  const out: Choice[] = [];
  for (const group of new Set(WORD)) {
    const prefix = `${group}.`;
    for (const name of names) {
      if (!name.startsWith(prefix)) continue;
      const suffix = name.slice(prefix.length);
      // A suffix is a variant name. The derived drawings (`mathbb/a`, `aCap`)
      // are other groups and never reach here.
      if (!/^[A-Za-z0-9]+$/.test(suffix)) continue;
      out.push({ group, variant: kebab(suffix) });
    }
  }

  return out;
}

export function useHeroVariants() {
  const { capture, ensure } = usePlan();
  const family = ref('"Lemmata", var(--built)');
  const label = ref(FALLBACK);
  /// The letters whose drawing just changed, so the page can animate them.
  const changed = ref<number[]>([]);
  /// Bumped on every swap; a key on the letter restarts its animation.
  const tick = ref(0);

  let face: FontFace | null = null;
  let timer: ReturnType<typeof setInterval> | null = null;
  let busy = false;
  let last = "";
  let options: Choice[] = [];

  async function swap(): Promise<void> {
    const bytes = capture.value;
    if (!bytes || busy || !options.length) return;
    // Pick the letter first, then its drawing. Picking a drawing directly would
    // drown `l` and `a` in the hundred `m` variants.
    const groups = [...new Set(options.map((option) => option.group))];
    const group = groups[Math.floor(Math.random() * groups.length)];
    // Never the same drawing twice running, so the change reads as a cycle
    // rather than a flicker.
    const pool = options.filter(
      (option) =>
        option.group === group && `${option.group}/${option.variant}` !== last,
    );
    const pick =
      pool[Math.floor(Math.random() * pool.length)] ??
      options.find((option) => option.group === group);
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
      changed.value = [...WORD]
        .map((letter, index) => (letter === pick.group ? index : -1))
        .filter((index) => index >= 0);
      tick.value += 1;
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
    options = choices();
    if (!options.length) return;
    await swap();
    timer = setInterval(() => void swap(), PERIOD);
  });

  onBeforeUnmount(() => {
    if (timer) clearInterval(timer);
    if (face) document.fonts.delete(face);
  });

  return { family, label, changed, tick };
}
