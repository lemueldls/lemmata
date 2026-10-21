// A hero word whose letters each redraw themselves from the plan.
//
// The ported letters carry variants, and a plan selects one by moving its
// drawing onto the letter a codepoint maps to. A font draws every occurrence of
// a letter the same way, so each occurrence gets its own face: the two `a`s and
// the two `m`s can then hold different drawings at once.
//
// The choices come from the registry rather than from the capture. A capture
// holds the drawings one build produced, which is the `a` family; the registry
// holds everything the port can draw, so `l` and `m` join in as soon as their
// definitions land.
import { preview_font, registry_names } from "lemmata-web";

import { usePlan } from "~/composables/plan";

const WORD = "lemmata";
const SWAP = 1900;
const FOLD = 240;
const FALLBACK = "live from the font";

interface Choice {
  group: string;
  variant: string;
}

/// `doubleStoreySerifless` in the kebab case a plan writes.
function kebab(name: string): string {
  return name.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);
}

export function useHeroVariants() {
  const { capture, ensure } = usePlan();
  const letter = [...WORD];
  const site = '"Lemmata", var(--built)';

  /// The face each letter is drawn in.
  const families = ref<string[]>(letter.map(() => site));
  /// Which half of the fold a letter is in: out, in, or at rest.
  const phases = ref<("out" | "in" | "")[]>(letter.map(() => ""));
  /// The accent mark under the letter that just changed.
  const notes = ref<boolean[]>(letter.map(() => false));
  const label = ref(FALLBACK);

  let options: Choice[] = [];
  let swappable: number[] = [];
  let timer: ReturnType<typeof setInterval> | null = null;
  let busy = false;
  let serial = 0;
  const current: (FontFace | null)[] = letter.map(() => null);
  const last: string[] = letter.map(() => "");

  /// The variant choices the port draws for a group.
  function choices(group: string): Choice[] {
    const prefix = `${group}.`;
    const out: Choice[] = [];
    for (const name of registry_names().split("\n")) {
      if (!name.startsWith(prefix)) continue;
      const suffix = name.slice(prefix.length);
      if (!/^[A-Za-z0-9]+$/.test(suffix)) continue;
      out.push({ group, variant: kebab(suffix) });
    }

    return out;
  }

  /// A face that draws one letter with one variant.
  async function makeFace(choice: Choice): Promise<FontFace | null> {
    const bytes = capture.value;
    if (!bytes) return null;
    const plan = JSON.stringify({
      variants: { [choice.group]: choice.variant },
    });
    const font = preview_font(bytes, plan, choice.group);
    if (!font.length) return null;
    const family = `Lemmata Hero ${serial++}`;
    const face = new FontFace(family, font.buffer as ArrayBuffer);
    await face.load();
    document.fonts.add(face);

    return face;
  }

  /// The letter folds edge-on, changes its drawing, and unfolds.
  async function swap(): Promise<void> {
    if (busy || !swappable.length) return;
    // A letter mid-fold waits for it to finish.
    const ready = swappable.filter((index) => phases.value[index] === "");
    if (!ready.length) return;
    const index = ready[Math.floor(Math.random() * ready.length)];
    if (index === undefined) return;
    const group = letter[index];
    if (!group) return;
    // Pick the drawing first, then fold towards it.
    const pool = options.filter(
      (option) => option.group === group && option.variant !== last[index],
    );
    const choice = pool[Math.floor(Math.random() * pool.length)];
    if (!choice) return;

    busy = true;
    try {
      const face = await makeFace(choice);
      if (!face) return;
      phases.value[index] = "out";
      notes.value[index] = true;
      label.value = `${group} · ${choice.variant}`;
      last[index] = choice.variant;
      window.setTimeout(() => {
        // The edge-on frame hides this swap, so the two drawings never show at
        // once.
        families.value[index] = `"${face.family}"`;
        if (current[index]) document.fonts.delete(current[index]);
        current[index] = face;
        phases.value[index] = "in";
        window.setTimeout(() => {
          if (phases.value[index] === "in") phases.value[index] = "";
        }, 360);
      }, FOLD);
      window.setTimeout(() => {
        notes.value[index] = false;
      }, 900);
    } catch {
      phases.value[index] = "";
    } finally {
      busy = false;
    }
  }

  onMounted(async () => {
    // A word that redraws itself is motion. Honour the setting that asks for
    // none, and leave the site face in place.
    if (matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    await ensure();
    for (const index of letter.keys()) {
      const group = letter[index];
      if (!group) continue;
      if (!options.some((option) => option.group === group)) {
        options.push(...choices(group));
      }
    }
    swappable = [...letter.keys()].filter((index) =>
      options.some((option) => option.group === letter[index]),
    );
    if (!swappable.length) return;
    await swap();
    timer = setInterval(() => void swap(), SWAP);
  });

  onBeforeUnmount(() => {
    if (timer) clearInterval(timer);
    for (const face of current) {
      if (face) document.fonts.delete(face);
    }
  });

  return { letter, families, phases, notes, label };
}
