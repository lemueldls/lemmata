<script setup lang="ts">
// The spellings of `a` the glyph source names, as a row of choices.
//
// Stub: variant.picker. The capture carries one spelling per glyph, so this sets
// the selection and reports it; wire it to the worker once a plan can name the
// variant it wants, and the specimen can show the one that was picked.
const VARIANTS = ["a", "a.doubleStoreySerifed", "a.singleStoreySerifless"];

const chosen = ref(VARIANTS[0]);
</script>

<template>
  <div class="variants">
    <button
      v-for="name in VARIANTS"
      :key="name"
      class="variant"
      type="button"
      :aria-pressed="chosen === name"
      @click="chosen = name"
    >
      <b>a</b>
      <span>{{ name }}</span>
    </button>
  </div>
  <StatusLine state="ok">{{ chosen }} · the spelling this build draws</StatusLine>
</template>
