<script setup lang="ts">
// The machine output line: a status, a state, and a dot that takes the accent
// when the state is good.
defineProps<{ state?: string }>();
</script>

<template>
  <p class="status" :data-state="state">
    <slot />
  </p>
</template>
