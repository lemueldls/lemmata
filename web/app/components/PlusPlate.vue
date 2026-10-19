<script setup lang="ts">
// One numbered block. The bar names it, the corner marks show where the sheet
// begins and ends, and the body is whatever the page puts inside.
defineProps<{
  index: string;
  label: string;
  meta?: string;
  id?: string;
}>();
</script>

<template>
  <section :id="id" class="plate" data-key-section>
    <span class="plate__plus plate__plus--tl" aria-hidden="true">+</span>
    <span class="plate__plus plate__plus--tr" aria-hidden="true">+</span>
    <span class="plate__plus plate__plus--bl" aria-hidden="true">+</span>
    <span class="plate__plus plate__plus--br" aria-hidden="true">+</span>

    <div class="wrap">
      <header class="plate__bar">
        <span class="plate__index">{{ index }}</span>
        <span class="plate__label">{{ label }}</span>
        <span v-if="meta" class="plate__meta">{{ meta }}</span>
      </header>
      <div class="plate__body">
        <slot />
      </div>
    </div>
  </section>
</template>
