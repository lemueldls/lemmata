<script setup lang="ts">
// A numbered section: the label in the left column, everything else in the
// right one. The numbers are the order of the argument, which is a proof.
defineProps<{ number: string; label: string; id?: string }>();
</script>

<template>
  <section class="section" :id="id">
    <div class="wrap section__grid">
      <p class="label">{{ number }} / {{ label }}</p>
      <div class="section__body">
        <slot />
      </div>
    </div>
  </section>
</template>
