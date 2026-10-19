<script setup lang="ts">
// The spellings of `a` that the outline set holds, as a grid of choices.
//
// The list comes from the file rather than from a table in this component, so
// it grows when the file does. The choice goes into the shared plan, which the
// build reads.
import { usePlan } from "~/composables/plan";

const plan = usePlan();
const mounted = ref(false);
const group = "a";

onMounted(async () => {
  await plan.ensure();
  mounted.value = true;
});

const options = computed(() => plan.variants(group));
const picked = computed(() => plan.chosen(group));

function choose(name: string): void {
  plan.choose(group, name);
}
</script>

<template>
  <div v-if="mounted">
    <div class="variant-head">
      <span class="variant-head__group"><b>a</b> group</span>
      <span class="status-line">
        <i />
        {{ picked === "" ? "outline drawing" : picked }}
      </span>
      <span class="mono-note">{{ options.length }} alternates</span>
    </div>

    <div class="variants">
      <button
        class="variant variant--default"
        type="button"
        :aria-pressed="picked === ''"
        @click="choose('')"
      >
        <span class="variant__glyph">a</span>
        <span class="variant__name">the outline's own drawing</span>
        <span class="variant__mark">{{ picked === "" ? "●" : "" }}</span>
      </button>

      <button
        v-for="name in options"
        :key="name"
        class="variant"
        type="button"
        :aria-pressed="picked === name"
        @click="choose(name)"
      >
        <span class="variant__glyph">a</span>
        <span class="variant__name">{{ name }}</span>
        <span class="variant__mark">{{ picked === name ? "●" : "" }}</span>
      </button>
    </div>

    <div class="form-row">
      <span>one group in this outline set</span>
      <button class="btn btn--sm" type="button" @click="choose('')">clear choice</button>
    </div>
  </div>

  <p v-else class="status-line" data-state="note">
    <i /> reading the outline set…
  </p>
</template>
