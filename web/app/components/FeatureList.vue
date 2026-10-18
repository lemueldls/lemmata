<script setup lang="ts">
// What the typeface carries, numbered, the way a mechanism is explained: one
// line for the name and one for what it does.
defineProps<{ rows: { name: string; note: string }[] }>();
</script>

<template>
  <div class="ledger">
    <div v-for="(row, index) in rows" :key="row.name" class="ledger__row">
      <span class="ledger__name">
        <b>{{ String(index + 1).padStart(2, "0") }}</b> {{ row.name }}
      </span>
      <span class="ledger__note">{{ row.note }}</span>
    </div>
  </div>
</template>
