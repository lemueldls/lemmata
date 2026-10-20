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

const SAMPLE = `the quick brown fox
jumps over the lazy dog
if (a <= b) return a
0123456789 ():;,."'*`;

const rows = computed(() => [
  ["bytes", props.bytes ? props.bytes.length.toLocaleString() : "—"],
  ["time", props.bytes ? `${Math.round(props.elapsed)} ms` : "—"],
  ["glyphs", props.metrics?.drawn ?? "—"],
  ["advance", props.metrics?.advance ?? "—"],
]);
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

      <div
        class="build-card__sample"
        :style="{
          '--preview': preview,
          fontSize: `${Math.round(specimenSize * 0.72)}px`,
        }"
      >
        {{ SAMPLE }}
      </div>
      <div class="build-card__sample-foot">
        <span>{{ bytes ? "built file" : "site face" }}</span>
        <span>{{ specimenSize }} px</span>
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
    </div>
  </div>
</template>
