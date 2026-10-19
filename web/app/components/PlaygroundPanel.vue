<script setup lang="ts">
// The playground: edit a line, resize it, and switch the features the face
// knows. The face is the current site face until a build finishes, then the
// preview swaps to the file the worker just made.
import { specimenSize } from "~/composables/hotkeys";

defineProps<{ preview: string }>();

const TEXT = `the quick brown fox jumps over the lazy dog
0123456789 ():;,."'*/%+#=<>`;

const SAMPLES = {
  code: `def root(f, a, b, eps):
    while b - a > eps:
        m = (a + b) / 2.0
    return m`,
  prose: `the quick brown fox jumps over the lazy dog.
the lazy dog does not mind at all.`,
  marks: `-> --> <- <-> => != == <= >=
&& || ^^ ~~ !! // /* */ ###`,
};

const text = ref(TEXT);
const line = ref(1.55);
const calt = ref(true);
const liga = ref(true);
const zero = ref(true);
const edit = ref<HTMLTextAreaElement | null>(null);

const settings = computed(
  () =>
    `"calt" ${calt.value ? 1 : 0}, "liga" ${liga.value ? 1 : 0}, "zero" ${
      zero.value ? 1 : 0
    }`,
);

function load(name: keyof typeof SAMPLES): void {
  text.value = SAMPLES[name];
}

function focusEditor(event: KeyboardEvent): void {
  const target = event.target;
  if (
    target instanceof HTMLElement &&
    (target.isContentEditable ||
      ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName))
  ) {
    return;
  }
  if (event.key !== "Enter" || event.metaKey || event.ctrlKey || event.altKey) {
    return;
  }
  event.preventDefault();
  edit.value?.focus();
}

onMounted(() => addEventListener("keydown", focusEditor));
onBeforeUnmount(() => removeEventListener("keydown", focusEditor));
</script>

<template>
  <div class="playground">
    <div class="playground__controls">
      <label class="playground__control">
        <span>size <b>{{ specimenSize }} px</b></span>
        <input v-model.number="specimenSize" type="range" min="12" max="96" step="1" />
      </label>

      <label class="playground__control">
        <span>leading <b>{{ line.toFixed(2) }}</b></span>
        <input v-model.number="line" type="range" min="1" max="2.2" step="0.05" />
      </label>

      <div class="playground__control">
        <span>features</span>
        <div class="playground__toggles">
          <button type="button" :aria-pressed="calt" @click="calt = !calt">calt</button>
          <button type="button" :aria-pressed="liga" @click="liga = !liga">liga</button>
          <button type="button" :aria-pressed="zero" @click="zero = !zero">zero</button>
        </div>
      </div>

      <div class="playground__samples">
        <button type="button" @click="load('code')">code sample</button>
        <button type="button" @click="load('prose')">prose sample</button>
        <button type="button" @click="load('marks')">marks sample</button>
      </div>

      <p class="mono-note">enter edits / esc leaves</p>
    </div>

    <div class="playground__stage">
      <pre
        class="playground__preview"
        :style="{
          '--preview': preview,
          fontSize: `${specimenSize}px`,
          lineHeight: line,
          fontFeatureSettings: settings,
        }"
      >{{ text }}</pre>

      <div class="playground__edit">
        <textarea
          ref="edit"
          v-model="text"
          rows="3"
          aria-label="Playground text"
        />
      </div>
    </div>
  </div>
</template>
