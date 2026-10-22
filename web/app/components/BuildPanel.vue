<script setup lang="ts">
// The build rail beside the configurator: the two actions, the progress line,
// the readings and a specimen of the file that came out.
import { specimenSize } from "~/composables/hotkeys";
import type { Reading } from "~/utils/font";

const props = defineProps<{
  status: string;
  state: string;
  bytes: Uint8Array | null;
  elapsed: number;
  busy: boolean;
  download: string;
  downloadName: string;
  metrics: Reading | null;
  preview: string;
}>();

defineEmits<{ build: [] }>();

const DEFAULT_SAMPLE = `the quick brown fox
jumps over the lazy dog
SPHINX OF BLACK QUARTZ, JUDGE MY VOW
-> => != <= >=
0123456789 ():;,."'*`;

const FEATURES = [
  { id: "calt", label: "ligatures" },
  { id: "TXTR", label: "texture" },
  { id: "zero", label: "zero" },
] as const;

type FeatureId = (typeof FEATURES)[number]["id"];

const sample = ref(DEFAULT_SAMPLE);
const leading = ref(1.55);
const features = reactive<Record<FeatureId, boolean>>({
  calt: true,
  TXTR: true,
  zero: false,
});

const featureSettings = computed(
  () =>
    `"calt" ${features.calt ? 1 : 0}, "TXTR" ${features.TXTR ? 1 : 0}, "zero" ${
      features.zero ? 1 : 0
    }`,
);

const rows = computed(() => [
  ["bytes", props.bytes ? props.bytes.length.toLocaleString() : "—"],
  ["time", props.bytes ? `${Math.round(props.elapsed)} ms` : "—"],
  ["glyphs", props.metrics?.drawn ?? "—"],
  ["advance", props.metrics?.advance ?? "—"],
]);

function resetSample(): void {
  sample.value = DEFAULT_SAMPLE;
}
</script>

<template>
  <div class="build-card">
    <header class="build-card__head">
      <span>build</span>
      <span>{{ busy ? "running" : bytes ? "ready" : "idle" }}</span>
    </header>

    <div class="build-card__body">
      <div class="build-card__actions">
        <button
          id="build"
          class="btn btn--accent"
          type="button"
          :disabled="busy"
          @click="$emit('build')"
        >
          {{ busy ? "building…" : "build" }} <kbd>b</kbd>
        </button>
        <a
          v-if="download"
          id="download"
          class="btn btn--ghost"
          :href="download"
          :download="downloadName"
        >
          download <kbd>d</kbd>
        </a>
        <button v-else class="btn btn--ghost" type="button" disabled>
          download <kbd>d</kbd>
        </button>
      </div>

      <div class="build-card__status" :data-state="state" aria-live="polite">
        <b>{{ state === "ok" ? "✓" : state === "bad" ? "!" : "·" }} output</b>
        <span id="status">{{ status }}</span>
      </div>

      <dl class="build-card__readout">
        <div v-for="[name, value] in rows" :key="name">
          <dt>{{ name }}</dt>
          <dd>{{ value }}</dd>
        </div>
      </dl>

      <textarea
        v-model="sample"
        class="build-card__sample"
        rows="5"
        aria-label="Editable specimen text"
        :style="{
          '--preview': preview,
          fontSize: `${Math.round(specimenSize * 0.72)}px`,
          lineHeight: leading,
          fontFeatureSettings: featureSettings,
        }"
      />

      <div class="build-card__sample-foot">
        <span>{{ bytes ? "built file" : "site face" }} / editable</span>
        <button class="text-button" type="button" @click="resetSample">
          reset text
        </button>
        <span>{{ specimenSize }} px / {{ leading.toFixed(2) }}</span>
      </div>

      <div class="build-card__features" role="group" aria-label="Preview features">
        <span class="build-card__features-label">features</span>
        <button
          v-for="feature in FEATURES"
          :key="feature.id"
          class="feature"
          type="button"
          :title="feature.id"
          :aria-pressed="features[feature.id]"
          @click="features[feature.id] = !features[feature.id]"
        >
          {{ feature.label }}
        </button>
      </div>
    </div>

    <div class="build-card__foot">
      <RangeControl
        v-model="specimenSize"
        label="size"
        unit="px"
        :min="12"
        :max="80"
      />
      <RangeControl
        v-model="leading"
        label="leading"
        :min="1"
        :max="2.2"
        :step="0.05"
      />
    </div>
  </div>
</template>
