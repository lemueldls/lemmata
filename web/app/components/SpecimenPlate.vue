<script setup lang="ts">
// A specimen plate: a sample at a size, over the grid of the font's own cells,
// with the face and the size named under it and a way to see another one.
//
// This is the one thing every font site has, and it is worth having: the claim
// on this page is about a font, and the font is the only thing that can make it
// look true. The shuffle keeps it from being a fixed badge.
import { useLemmataFont } from "~/composables/font";

const props = withDefaults(
  defineProps<{ samples: string[]; face?: string; size?: number }>(),
  { face: "Lemmata Regular", size: 48 },
);

const font = useLemmataFont();
const index = ref(0);
const cell = ref("12px");

const sample = computed(() => props.samples[index.value % props.samples.length]);
const shuffle = (): void => {
  index.value = (index.value + 1) % props.samples.length;
};

// The cell is one advance at this size, so the grid is the grid the letters are
// drawn on rather than a decoration behind them.
watch(
  () => font.ready,
  (ready) => {
    const advance = font.metrics?.advance;
    if (!ready || advance === null || advance === undefined) return;
    cell.value = `${(advance / 1000) * props.size}px`;
  },
  { immediate: true },
);
</script>

<template>
  <div class="panel">
    <div class="plate" :style="{ '--cell': cell, '--plate': `${size}px` }">
      <p class="plate__sample">{{ sample }}</p>
      <div class="plate__foot">
        <span>{{ face }} / {{ size }}px</span>
        <button class="chip" type="button" @click="shuffle">
          Show another sample
        </button>
      </div>
    </div>
  </div>
</template>
