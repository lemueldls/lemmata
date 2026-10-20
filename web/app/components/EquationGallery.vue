<script setup lang="ts">
// Four equations from the corpus, set in the math face.
//
// The MathML is inlined, not fetched, so the figures render on the first paint.
// The same markup is the editable source on the equations page.
import corpus from "../../../corpus/equations.mathml?raw";

const blocks = corpus
  .replace(/<!--[\s\S]*?-->/g, "")
  .match(/<math[\s\S]*?<\/math>/g) ?? [];

const LABELS = [
  "addition",
  "mixed operators",
  "square of a sum",
  "fraction equality",
  "radical",
  "summation",
  "integral",
  "inequality",
  "nested radical",
  "fenced fraction",
  "greek letters",
  "matrix",
];

const PICKS = [3, 4, 5, 6, 11, 10];

const shown = computed(() =>
  PICKS.map((index) => ({
    index,
    label: LABELS[index] ?? `equation ${index + 1}`,
    math: blocks[index] ?? "",
  })).filter((entry) => entry.math),
);
</script>

<template>
  <div class="eq-grid">
    <figure
      v-for="equation in shown"
      :key="equation.label"
      class="eq-card"
    >
      <figcaption class="eq-card__bar">
        <span>{{ equation.label }}</span>
        <span>mathml / display</span>
      </figcaption>
      <div class="eq-card__body" v-html="equation.math" />
    </figure>
  </div>
</template>
