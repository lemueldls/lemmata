<script setup lang="ts">
// The ligature sets a coding face is expected to have, shown as the character
// sequences they stand for.
//
// Stub: ligatures. The font writes no `liga`, so these are the sequences and not
// the joined forms: the page can show which sets it means, and a reader sees the
// characters with the spacing the face gives them. Wire this to the plan when
// the feature lands, and the two columns become one.
const SETS = [
  { name: "arrows", text: "-> --> <- <-> =>" },
  { name: "comparison", text: "!= == <= >= =/" },
  { name: "logic", text: "&& || ^^ ~~ !!" },
  { name: "marks", text: "// /* */ ###" },
];
</script>

<template>
  <div class="ligatures">
    <div v-for="set in SETS" :key="set.name" class="ligature">
      <span>{{ set.text }}</span>
      <small>{{ set.name }}</small>
    </div>
  </div>
</template>
