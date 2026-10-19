<script setup lang="ts">
// What the face holds, as a spec sheet rather than a feature card.
defineProps<{
  rows: { name: string; note: string; state: "ready" | "next" }[];
}>();
</script>

<template>
  <div class="matrix">
    <div v-for="(row, index) in rows" :key="row.name" class="matrix__row">
      <span class="matrix__num">{{ String(index + 1).padStart(2, "0") }}</span>
      <span class="matrix__name">{{ row.name }}</span>
      <span class="matrix__note">{{ row.note }}</span>
      <span class="matrix__state" :data-state="row.state">
        {{ row.state === "ready" ? "ready" : "next" }}
      </span>
    </div>
  </div>
</template>
