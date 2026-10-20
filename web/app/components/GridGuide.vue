<script setup lang="ts">
// The page's column grid. Five guides cross the frame, and a plus sits where a
// guide meets the horizontal rule that ends a block.
//
// Every frame uses the same five positions, so the marks line up from the hero
// down to the footer instead of sitting wherever a section happens to end.
withDefaults(
  defineProps<{
    lines?: boolean;
    top?: boolean;
    bottom?: boolean;
  }>(),
  { lines: true, top: true, bottom: false },
);

const COLUMNS = [0, 25, 50, 75, 100];
</script>

<template>
  <div class="grid-guide" aria-hidden="true">
    <template v-if="lines">
      <span
        v-for="x in COLUMNS"
        :key="`line-${x}`"
        class="grid-guide__line"
        :style="{ left: `${x}%` }"
      />
    </template>

    <template v-if="top">
      <span
        v-for="x in COLUMNS"
        :key="`top-${x}`"
        class="grid-guide__cross grid-guide__cross--top"
        :style="{ left: `${x}%` }"
      >+</span>
    </template>

    <template v-if="bottom">
      <span
        v-for="x in COLUMNS"
        :key="`bottom-${x}`"
        class="grid-guide__cross grid-guide__cross--bottom"
        :style="{ left: `${x}%` }"
      >+</span>
    </template>
  </div>
</template>
