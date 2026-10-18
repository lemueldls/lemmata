<script setup lang="ts">
// The plan, as the object the builder defaults to, with a copy button.
//
// It is content on the page rather than something fetched, so it is visible with
// the scripts off, and the highlighting is done on top of the text that is
// already there.
const props = defineProps<{ plan: Record<string, number> }>();

const pre = ref<HTMLElement | null>(null);
const copied = ref<"copy" | "copied" | "select it">("copy");
const json = computed(() => JSON.stringify(props.plan, null, 2));

const highlight = (): string =>
  json.value
    .split("\n")
    .map((line) => {
      const match = /^(\s*)"([^"]+)":\s*(.+?)(,?)$/.exec(line);
      if (!match) return line;

      return `${match[1]}<span class="k">"${match[2]}"</span>: <span class="n">${match[3]}</span>${match[4]}`;
    })
    .join("\n");

async function copy(): Promise<void> {
  try {
    await navigator.clipboard.writeText(json.value);
    copied.value = "copied";
  } catch {
    copied.value = "select it";
  }
  setTimeout(() => {
    copied.value = "copy";
  }, 1400);
}

onMounted(() => {
  if (pre.value) pre.value.innerHTML = highlight();
});
</script>

<template>
  <div class="panel">
    <div class="panel__head">
      <span>plan.json</span>
      <button class="chip" type="button" @click="copy">{{ copied }}</button>
    </div>
    <pre class="code" ref="pre">{{ json }}</pre>
  </div>
</template>
