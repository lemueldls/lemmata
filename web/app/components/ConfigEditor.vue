<script setup lang="ts">
// The plan as text. This is the same object the controls edit, so the editor
// and the fields cannot disagree.
const plan = defineModel<string>({ required: true });

const emit = defineEmits<{ reset: [] }>();

const copied = ref(false);

const parsed = computed(() => {
  const text = plan.value.trim();
  if (!text) return { ok: true as const, value: {} as Record<string, unknown> };

  try {
    return { ok: true as const, value: JSON.parse(text) as Record<string, unknown> };
  } catch (error) {
    return {
      ok: false as const,
      message: error instanceof Error ? error.message : String(error),
    };
  }
});

const lineCount = computed(() => plan.value.split("\n").length);
const keyCount = computed(() =>
  parsed.value.ok ? Object.keys(parsed.value.value).length : 0,
);

function format(): void {
  if (!parsed.value.ok) return;
  plan.value = JSON.stringify(parsed.value.value, null, 2);
}

async function copy(): Promise<void> {
  try {
    await navigator.clipboard.writeText(plan.value);
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
  <div class="editor">
    <div class="editor__bar">
      <span>plan.json</span>
      <button type="button" @click="format">format</button>
      <button type="button" @click="copy">{{ copied ? "copied" : "copy" }}</button>
      <button type="button" @click="$emit('reset')">reset</button>
    </div>

    <textarea
      id="plan"
      v-model="plan"
      class="textarea"
      rows="15"
      aria-label="Plan JSON"
    />

    <div class="editor__foot" :data-state="parsed.ok ? 'ok' : 'bad'">
      <span>
        {{ parsed.ok ? "parsed" : "parse error" }} /
        {{ parsed.ok ? `${keyCount} keys` : parsed.message }}
      </span>
      <span>{{ lineCount }} lines</span>
    </div>
  </div>
</template>
