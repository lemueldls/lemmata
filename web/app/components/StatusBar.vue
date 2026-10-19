<script setup lang="ts">
// The bottom rail. It reports the face, the page, the keys and a way back to
// the top, in the shape of an editor status line.
import { useRoute } from "vue-router";
import { useLemmataFont } from "~/composables/font";
import { specimenSize } from "~/composables/hotkeys";

const route = useRoute();
const font = useLemmataFont();

const state = computed(() => {
  if (font.error) return { label: "font failed", tone: "bad" };
  if (!font.ready) return { label: `loading ${Math.round(font.progress * 100)}%`, tone: "note" };

  return { label: "font ready", tone: "ok" };
});

const page = computed(() => {
  if (route.path.startsWith("/generate")) return "02 / configurator";
  if (route.path.startsWith("/math")) return "03 / equations";

  return "01 / specimen";
});

function top(): void {
  scrollTo({ top: 0, behavior: "smooth" });
}
</script>

<template>
  <div class="statusbar">
    <span class="statusbar__state" :data-state="state.tone">
      <i /> {{ state.label }}
    </span>
    <span class="statusbar__page">{{ page }}</span>
    <span class="statusbar__keys">
      <span><kbd>?</kbd> keys</span>
      <span><kbd>↑</kbd><kbd>↓</kbd> blocks</span>
      <span><kbd>t</kbd> palette</span>
      <span><kbd>g</kbd> build</span>
      <span><kbd>e</kbd> math</span>
      <span>size {{ specimenSize }}</span>
    </span>
    <button class="statusbar__top" type="button" @click="top">top ↑</button>
  </div>
</template>
