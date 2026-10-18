<script setup lang="ts">
// The generator, which is the one page that compiles a font. The compiler is a
// wasm module, it runs in a worker, and the button is the only thing that asks
// for it to run.
import { advance, cmap, glyphCount, ink } from "~/utils/font";

useHead({ title: "Generate" });

const { plan, status, state, download, downloadName, bytes, busy, run } =
  useBuild();

const specimen = ref<HTMLElement | null>(null);

const readings = computed(() => {
  const font = bytes.value;
  if (!font) return "Metrics appear once a font is built.";
  const map = cmap(font);

  return [
    `${glyphCount(font) ?? "?"} glyphs drawn, ${map.size} characters mapped`,
    `x ${ink(map, font, "x")?.yMax ?? "?"} · l ${ink(map, font, "l")?.yMax ?? "?"} · ` +
      `p ${ink(map, font, "p")?.yMin ?? "?"} · advance ${advance(map, font, "x") ?? "?"}`,
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
