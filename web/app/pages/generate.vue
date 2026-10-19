<script setup lang="ts">
// The generator, in the order Iosevka puts a configurator in: the basics, the
// variants, the ligations, a specimen to try them on, and the config itself.
//
// The page keeps one plan, which the variant picker also writes to, so a choice
// made anywhere is the choice that gets built. The specimen is set with the line
// height the font itself declares, so the ascender and the descender show up in
// the page and not only in the numbers.
//
// Stubs: the family name is not a plan key yet, so that field does nothing; the
// ligature sets are the sequences, since the font writes no `liga`.
import { advance, cmap, glyphCount, ink } from "~/utils/font";
import { specimenSize } from "~/composables/hotkeys";
import { usePlan } from "~/composables/plan";

useHead({ title: "Generate" });

const { plan, capture, ensure, spacing, setSpacing } = usePlan();
const { status, state, download, downloadName, bytes, busy, run } = useBuild();

const specimen = ref<HTMLElement | null>(null);
const family = ref("Lemmata");

const BASICS = [
  { key: "capHeight", label: "cap height" },
  { key: "xHeight", label: "x height" },
  { key: "ascender", label: "ascender" },
  { key: "descender", label: "descender" },
  { key: "weight", label: "weight" },
  { key: "italicAngle", label: "italic angle" },
];

onMounted(() => {
  void ensure();
});

/// The plan as a form.
function value(key: string): number {
  try {
    return Number(JSON.parse(plan.value)[key] ?? 0);
  } catch {
    return 0;
  }
}

function set(key: string, next: number): void {
  try {
    const object = JSON.parse(plan.value);
    object[key] = next;
    plan.value = JSON.stringify(object, null, 2);
  } catch {
    // A plan that does not parse belongs to whoever wrote it, so the field
    // leaves it alone rather than replacing what they typed.
  }
}

async function build(): Promise<void> {
  // The outlines arrive with the plan, and a build asked for before they are here
  // would have nothing to draw from, so it waits rather than doing nothing.
  await ensure();
  if (capture.value) await run(plan.value, capture.value);
}

const readings = computed(() => {
  const font = bytes.value;
  if (!font) return "Numbers appear once a font is built.";
  const map = cmap(font);

  return [
    `${glyphCount(font) ?? "?"} glyphs drawn, ${map.size} characters`,
    `x ${ink(map, font, "x")?.yMax ?? "?"} · l ${ink(map, font, "l")?.yMax ?? "?"} · ` +
      `p ${ink(map, font, "p")?.yMin ?? "?"} · advance ${advance(map, font, "x") ?? "?"}`,
  ].join("\n");
});

watch(bytes, async (font) => {
  const target = specimen.value;
  if (!font || !target) return;
  // A family of its own, so the specimen is the font just built and never stands
  // in for the face the rest of the site is set in.
  const face = new FontFace("Lemmata Build", font.buffer as ArrayBuffer);
  await face.load();
  document.fonts.add(face);
  target.style.fontFamily = "Lemmata Build";
});
</script>

<template>
  <div class="wrap page">
    <h1>Generator</h1>
    <p class="lede">
      The plan is the font's metrics, and the fields below write it. Every key
      defaults, so this is the reference plan; change one and the tables follow.
    </p>

    <h2 class="plate-label">01 Basics</h2>
    <div class="fields">
      <label class="field">
        <span>family name</span>
        <input v-model="family" type="text" spellcheck="false" />
      </label>
      <label v-for="field in BASICS" :key="field.key" class="field">
        <span>{{ field.label }}</span>
        <input
          type="number"
          :value="value(field.key)"
          @input="set(field.key, Number(($event.target as HTMLInputElement).value))"
        />
      </label>
    </div>
    <div class="row">
      <span class="note">spacing</span>
      <div class="segmented">
        <button
          v-for="option in ['normal', 'quasi-proportional']"
          :key="option"
          type="button"
          :aria-pressed="spacing === option"
          @click="setSpacing(option)"
        >
          {{ option }}
        </button>
      </div>
    </div>

    <div class="rule-art"><i /></div>

    <h2 class="plate-label">02 Variants</h2>
    <p class="note">Most letters can be drawn more than one way.</p>
    <VariantPicker />

    <div class="rule-art"><i /></div>

    <h2 class="plate-label">03 Ligations</h2>
    <p class="note">The sets a coding face is expected to have.</p>
    <LigatureList />

    <div class="rule-art"><i /></div>

    <h2 class="plate-label">04 Try it</h2>
    <div class="row">
      <button class="btn btn--primary" id="build" type="button" :disabled="busy" @click="build">
        Build
      </button>
      <a class="btn" id="download" :href="download" :download="downloadName" :hidden="!download">
        Download
      </a>
      <label class="size">
        size
        <input v-model.number="specimenSize" type="range" min="12" max="64" step="1" />
        <b>{{ specimenSize }}</b>
      </label>
    </div>
    <pre id="status" :data-state="state">{{ status }}</pre>
    <p
      class="specimen"
      id="specimen"
      ref="specimen"
      :style="{ fontSize: `${specimenSize}px`, lineHeight: 'normal' }"
    >
      the quick brown fox jumps over the lazy dog<br >
      0123456789 ():;,."'*/%+#=&lt;&gt;
    </p>
    <pre id="metrics">{{ readings }}</pre>

    <div class="rule-art"><i /></div>

    <h2 class="plate-label">05 Your config</h2>
    <p class="note">The same object, as the builder takes it.</p>
    <textarea id="plan" v-model="plan" rows="10" spellcheck="false" />
  </div>
</template>
