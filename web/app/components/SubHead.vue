<script setup lang="ts">
// A part inside a section, with its own rule and its own heading.
defineProps<{ title: string }>();
</script>

<template>
  <div class="subhead">
    <h3>{{ title }}</h3>
    <slot />
  </div>
</template>
