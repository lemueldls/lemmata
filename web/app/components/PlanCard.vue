<script setup lang="ts">
// The plan as the builder takes it, with a copy button.
//
// The object is content on the page, so it is in the prerendered HTML and reads
// with the scripts off. The colouring sits on top of that text.
const props = defineProps<{ plan: Record<string, number | string> }>();

const copied = ref(false);
const json = computed(() => JSON.stringify(props.plan, null, 2));

const html = computed(() =>
  json.value
    .split("\n")
    .map((line) => {
      const match = /^(\s*)"([^"]+)":\s*(.+?)(,?)$/.exec(line);
      if (!match) return line;
      const value = match[3] ?? "";
      const tone = value.startsWith('"') ? "s" : "n";

      return `${match[1]}<span class="k">"${match[2]}"</span>: <span class="${tone}">${value}</span>${match[4]}`;
    })
    .join("\n"),
);

const keys = computed(() => Object.keys(props.plan).length);

async function copy(): Promise<void> {
  try {
    await navigator.clipboard.writeText(json.value);
    copied.value = true;
    setTimeout(() => {
      copied.value = false;
    }, 1400);
  } catch {
    copied.value = false;
  }
}
</script>

<template>
  <div class="plan-card">
    <div class="plan-card__bar">
      <span>plan.json</span>
      <button class="btn btn--sm" type="button" @click="copy">
        {{ copied ? "copied" : "copy" }}
      </button>
    </div>
    <pre class="plan-card__code" v-html="html" />
    <div class="plan-card__foot">
      <span>reference plan</span>
      <span>{{ keys }} keys</span>
    </div>
  </div>
</template>
