<script setup lang="ts">
// The mark: a glyph cell with three guides and one filled square.
//
// The font has no logo glyph, and a mark that falls back to the reader's system
// face would change shape per machine. This one is drawn.
defineProps<{ size?: number }>();
</script>

<template>
  <svg
    :width="size ?? 20"
    :height="size ?? 20"
    viewBox="0 0 20 20"
    fill="none"
    aria-hidden="true"
  >
    <path d="M1 1h18v18H1z" stroke="currentColor" stroke-width="1.2" />
    <path
      d="M7 1v18M13 1v18M1 7h18M1 13h18"
      stroke="currentColor"
      stroke-width="0.7"
      opacity="0.42"
    />
    <rect x="7" y="7" width="6" height="6" fill="currentColor" />
  </svg>
</template>
