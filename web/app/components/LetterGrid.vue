<script setup lang="ts">
// The character grid: every character the font has, out of its own map, drawn at
// a size worth looking at.
//
// A grid is the most honest picture of a monospace face, because the grid is the
// design: the same advance, over and over, with the drawing moving inside it.
import { useLemmataFont } from "~/composables/font";

const font = useLemmataFont();

const characters = computed(() =>
  font.map ? [...font.map.keys()].sort((left, right) => left - right) : [],
);

const label = (code: number): string =>
  `U+${code.toString(16).toUpperCase().padStart(4, "0")}`;
</script>

<template>
  <div class="letters">
    <figure
      v-for="code in characters"
      :key="code"
      class="letter"
      :class="{ 'letter--blank': code === 0x20 }"
    >
      <i v-if="code === 0x20" aria-label="space" />
      <i v-else>{{ String.fromCodePoint(code) }}</i>
      <figcaption>{{ label(code) }}</figcaption>
    </figure>
  </div>
</template>
