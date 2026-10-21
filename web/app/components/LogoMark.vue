<script setup lang="ts">
// The mark: three stops in the proof triangle.
//
// A1 from the mark studies: 12-unit dots on a 48-unit square, with the top stop
// centered over the pair below. The margin is seven units on every side.
defineProps<{ size?: number }>();
</script>

<template>
  <svg
    :width="size ?? 20"
    :height="size ?? 20"
    viewBox="0 0 48 48"
    fill="currentColor"
    aria-hidden="true"
  >
    <rect x="18" y="8" width="12" height="12" rx="1.4" />
    <rect x="8" y="28" width="12" height="12" rx="1.4" />
    <rect x="28" y="28" width="12" height="12" rx="1.4" />
  </svg>
</template>
