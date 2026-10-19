<script setup lang="ts">
// The ligature sets a coding face is expected to name, written as the sequences
// they stand for.
//
// This build writes no `liga` lookups, so the rows are labelled as the next
// piece rather than shown as joined forms. The sequence column still shows what
// the face does with each character today.
const SETS = [
  { name: "arrows", seq: "-> --> <- <-> =>", state: "next" },
  { name: "comparison", seq: "!= == <= >= =/", state: "next" },
  { name: "logic", seq: "&& || ^^ ~~ !!", state: "next" },
  { name: "marks", seq: "// /* */ ###", state: "next" },
  { name: "powerline", seq: ">_ |_ _|", state: "next" },
] as const;
</script>

<template>
  <div class="ligatures">
    <div v-for="set in SETS" :key="set.name" class="ligature">
      <span class="ligature__name">{{ set.name }}</span>
      <span class="ligature__seq">{{ set.seq }}</span>
      <span class="ligature__state" :data-state="set.state">
        {{ set.state }}
      </span>
    </div>
  </div>
  <p class="mono-note" style="margin-top: 12px">
    The plan names these sets. The current build writes no <code>liga</code> table, so the
    rows show the characters the face sets today.
  </p>
</template>
