<script setup lang="ts">
// The face at work: an editor window with one short file, a line of chrome and
// an inspector for a single glyph.
//
// The sample avoids capitals and brackets on purpose. The capture covers
// lowercase, digits and the punctuation around them, so a bracket would call a
// fallback face and the window would stop showing the typeface it claims.
import { useLemmataFont } from "~/composables/font";

const font = useLemmataFont();

const SOURCE = `# place each drawing inside one cell
def stem(width, side):
    if width > side:
        return width - side
    return side

for pair in grid:
    stem(pair.left, 2)`;

const LINES = SOURCE.split("\n").length;

const { data: html } = await useAsyncData("workbench-code", async () => {
  if (!import.meta.client) {
    const { highlight } = await import("~/utils/highlight");

    return highlight(SOURCE, "python");
  }

  return null;
});

const rows = computed(() => [
  ["advance", font.metrics?.advance ?? "—"],
  ["x height", font.metrics?.xHeight ?? "—"],
  ["ascender", font.metrics?.ascender ?? "—"],
  ["descender", font.metrics?.descender ?? "—"],
]);
</script>

<template>
  <div class="workbench">
    <div class="workbench__bar">
      <span class="workbench__lights"><i /><i /><i /></span>
      <span class="workbench__tab">specimen.py</span>
      <span class="workbench__tab workbench__tab--idle">plan.json</span>
      <span class="workbench__meta">utf-8 / lf / python</span>
    </div>

    <div class="workbench__body">
      <ol class="workbench__gutter">
        <li v-for="line in LINES" :key="line">{{ line }}</li>
      </ol>
      <div v-if="html" class="workbench__code" v-html="html" />
      <pre v-else class="workbench__code">{{ SOURCE }}</pre>

      <aside class="workbench__inspector">
        <p class="workbench__label">selected glyph</p>
        <span class="workbench__glyph">g</span>
        <dl>
          <div v-for="[name, value] in rows" :key="name">
            <dt>{{ name }}</dt>
            <dd>{{ value }}</dd>
          </div>
        </dl>
      </aside>
    </div>

    <div class="workbench__status">
      <span>python 3</span>
      <span>lemmata regular / 14 px</span>
      <span>{{ font.metrics?.drawn ?? "—" }} glyphs in the face</span>
      <span>ln 4, col 12</span>
    </div>
  </div>
</template>
