<script setup lang="ts">
// One numbered block. The bar names it, the column guides run through it and
// the corner marks sit on the frame's own rules.
defineProps<{
  index: string;
  label: string;
  meta?: string;
  id?: string;
  bottom?: boolean;
}>();
</script>

<template>
  <section :id="id" class="plate" data-key-section>
    <div class="wrap plate__frame">
      <GridGuide :bottom="bottom" />

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
