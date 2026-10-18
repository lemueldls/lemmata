<script setup lang="ts">
// Every character the font carries, out of its own character map, drawn in the
// font it came from. The sheet is a reading rather than a decoration: it can
// only show what the font has.
import { useLemmataFont } from "~/composables/font";

const font = useLemmataFont();

const characters = computed(() =>
  font.map ? [...font.map.keys()].sort((left, right) => left - right) : [],
);

const label = (code: number): string =>
  `U+${code.toString(16).toUpperCase().padStart(4, "0")}`;
</script>

<template>
  <div class="glyphs">
    <figure
      v-for="code in characters"
      :key="code"
      class="glyph"
      :class="{ 'glyph--blank': code === 0x20 }"
    >
      <i v-if="code === 0x20" aria-label="space" />
      <i v-else>{{ String.fromCodePoint(code) }}</i>
      <figcaption>{{ label(code) }}</figcaption>
    </figure>
  </div>
</template>
