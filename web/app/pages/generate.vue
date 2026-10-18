<script setup lang="ts">
// The generator, which is the one page that compiles a font. The compiler is a
// wasm module, it runs in a worker, and the button is the only thing that asks
// for it to run.
import { advance, cmap, glyphCount, ink } from "~/utils/font";

useHead({ title: "Generate" });

const { plan, status, state, download, downloadName, bytes, busy, run } =
  useBuild();
const { spacing: currentSpacing, setSpacing } = usePlan();

// The two spacings are two builds, so this picks the outlines the generator
// hands the builder. The plan records the choice, which is why it is set there
// rather than held here.
const SPACINGS = ["normal", "quasi-proportional"];

const specimen = ref<HTMLElement | null>(null);

const readings = computed(() => {
  const font = bytes.value;
  if (!font) return "Metrics appear once a font is built.";
  const map = cmap(font);

  // The advances are here because the spacing is a choice: `x` is the same cell
  // either way, and `i` and `m` are what say which face was built.
  return [
    `${glyphCount(font) ?? "?"} glyphs drawn, ${map.size} characters mapped`,
    `x ${ink(map, font, "x")?.yMax ?? "?"} · l ${ink(map, font, "l")?.yMax ?? "?"} · ` +
      `p ${ink(map, font, "p")?.yMin ?? "?"}`,
    `advance i ${advance(map, font, "i") ?? "?"} · m ${advance(map, font, "m") ?? "?"} · ` +
      `x ${advance(map, font, "x") ?? "?"}`,
  ].join("\n");
});

watch(bytes, async (font) => {
  const target = specimen.value;
  if (!font || !target) return;
  // Registers under a family of its own, so the specimen is the font just built
  // and never stands in for the reference face the rest of the site is set in.
  const face = new FontFace("Lemmata Build", font.buffer as ArrayBuffer);
  await face.load();
  document.fonts.add(face);
  target.style.fontFamily = "Lemmata Build";
});
</script>

<template>
  <div class="wrap page">
    <h1>Generate</h1>
    <p>
      The plan below is the font's metrics. Every key defaults, so this is the
      reference plan; change one and the tables follow. The specimen under it is
      the font it builds, and the readings under that come from the font's own
      tables.
    </p>
    <div class="row">
      <button class="btn btn--primary" id="build" type="button" :disabled="busy" @click="run">
        Build a font
      </button>
      <a class="btn" id="download" :href="download" :download="downloadName" :hidden="!download">
        Download
      </a>
    </div>
    <div class="row">
      <button
        v-for="name in SPACINGS"
        :key="name"
        class="btn"
        type="button"
        :aria-pressed="currentSpacing === name"
        @click="setSpacing(name)"
      >
        {{ name }}
      </button>
    </div>
    <textarea id="plan" v-model="plan" rows="8" spellcheck="false" />
    <pre id="status" :data-state="state">{{ status }}</pre>

    <h2>Specimen</h2>
    <p class="specimen" id="specimen" ref="specimen">
      the quick brown fox jumps over the lazy dog<br >
      0123456789 ():;,."'*/%+#=&lt;&gt;
    </p>
    <pre id="metrics">{{ readings }}</pre>
  </div>
</template>
