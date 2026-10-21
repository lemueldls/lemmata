<script setup lang="ts">
// The ligature sets a coding face is expected to name, written as the sequences
// they stand for.
//
// The build has Iosevka's chaining rules for `calt`, so a row whose glyphs
// the font holds draws joined. A sequence with no rule stays as typed. The
// logic and powerline rows need a pipe the page's capture does not hold yet,
// so they stay labelled rather than pretending.
const SETS = [
  { name: "arrows", seq: "-> --> <- <-> =>", state: "on" },
  { name: "comparison", seq: "!= == <= >=", state: "on" },
  { name: "logic", seq: "&& || ^^ ~~ !!", state: "next" },
  { name: "marks", seq: "// /* */ ###", state: "on" },
  { name: "powerline", seq: ">_ |_ _|", state: "next" },
] as const;

defineProps<{ preview?: string }>();
</script>

<template>
  <div class="ligatures">
    <div v-for="set in SETS" :key="set.name" class="ligature">
      <span class="ligature__name">{{ set.name }}</span>
      <span class="ligature__seq" :style="preview ? { fontFamily: preview } : undefined">
        {{ set.seq }}
      </span>
      <span class="ligature__state" :data-state="set.state">
        {{ set.state }}
      </span>
    </div>
  </div>
  <p class="mono-note" style="margin-top: 12px">
    The build has the chaining rules Iosevka uses for <code>calt</code>, so a row draws
    with the font the page just built. The logic and powerline rows wait on a pipe the
    capture does not hold.
  </p>
</template>
