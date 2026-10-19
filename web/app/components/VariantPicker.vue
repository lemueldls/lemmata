<script setup lang="ts">
// The spellings of `a` the glyph source names, as a row of choices.
//
// The list is read out of the capture, so it follows the glyphs rather than a
// table written here, and the choice goes into the shared plan: the generator
// builds the one that was picked.
import { usePlan } from "~/composables/plan";

const plan = usePlan();

const group = "a";
// The list comes from the glyphs, which arrive after the page does, so it is
// rendered once the page is running: a first paint would print an empty row and
// the next one would disagree with it.
const mounted = ref(false);
onMounted(async () => {
  await plan.ensure();
  mounted.value = true;
});

const options = computed(() => plan.variants(group));
const picked = computed(() => plan.chosen(group));

// The group's own drawing is what a plan gets by saying nothing, so it is a
// choice of its own rather than a missing one.
const clear = (): void => plan.choose(group, "");
</script>

<template>
  <div v-if="mounted" class="variants">
    <button
      class="variant"
      type="button"
      :aria-pressed="picked === ''"
      @click="clear"
    >
      <b>a</b>
      <span>the capture's own drawing</span>
    </button>
    <button
      v-for="name in options"
      :key="name"
      class="variant"
      type="button"
      :aria-pressed="picked === name"
      @click="plan.choose(group, name)"
    >
      <b>a</b>
      <span>{{ name }}</span>
    </button>
  </div>
  <StatusLine v-if="mounted" state="ok">
    {{ picked === "" ? "the capture's own drawing" : picked }} ·
    {{ options.length }} spellings the glyph source names
  </StatusLine>
</template>
