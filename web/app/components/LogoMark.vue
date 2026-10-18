<script setup lang="ts">
// The mark: three squares in a triangle.
//
// Drawn rather than typeset, because the font does not carry `∴`, `⁂` or any
// other mark that would do: the capture is letters, digits and ASCII
// punctuation, so a glyph outside that set would fall back to whatever the
// reader has installed and the mark would change shape per machine.
//
// Squares because that is the shape the font gives a full stop, and three of
// them in a triangle because that reads two ways at once: as `∴`, the mark that
// finishes a derivation, and as the grid this typeface is drawn on with three of
// its cells marked.
defineProps<{ size?: number }>();
</script>

<template>
  <svg
    :width="size ?? 15"
    :height="size ?? 15"
    viewBox="0 0 16 16"
    fill="currentColor"
    aria-hidden="true"
  >
    <rect x="5.75" y="1.5" width="4.5" height="4.5" />
    <rect x="1.5" y="10" width="4.5" height="4.5" />
    <rect x="10" y="10" width="4.5" height="4.5" />
  </svg>
</template>
