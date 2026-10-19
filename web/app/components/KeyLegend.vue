<script setup lang="ts">
// The keys, and where they are written down.
//
// The legend is part of the chrome rather than a page's copy, because the keys
// work everywhere and a reader who does not find them will not guess them.
import { useHotkeys } from "~/composables/hotkeys";

const { keys } = useHotkeys();

const label: Record<string, string> = {
  "digit": "sections",
  "up": "up",
  "down": "down",
  "+": "bigger",
  "-": "smaller",
  "m": "theme",
  "r": "reset",
  "g": "generator",
  "e": "equations",
};
</script>

<template>
  <div class="legend">
    <span v-for="key in keys" :key="key">
      <kbd>{{ key === "digit" ? "1-9" : key }}</kbd>
      {{ label[key] ?? key }}
    </span>
  </div>
</template>
