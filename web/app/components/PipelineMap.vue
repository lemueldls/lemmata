<script setup lang="ts">
// The path from a plan to a file, as five boxes.
const STEPS = [
  { name: "plan", note: "one object, seven keys" },
  { name: "parameters", note: "Iosevka's tables read as written" },
  { name: "outlines", note: "drawings placed in the cell" },
  { name: "tables", note: "glyf, cmap, hmtx, MATH" },
  { name: "font", note: "ttf or woff2, ready to install" },
];
</script>

<template>
  <div class="pipeline" aria-label="Build path">
    <div v-for="(step, index) in STEPS" :key="step.name" class="pipeline__step">
      <span class="pipeline__num">{{ String(index + 1).padStart(2, "0") }}</span>
      <span class="pipeline__name">{{ step.name }}</span>
      <span class="pipeline__note">{{ step.note }}</span>
    </div>
  </div>
</template>
