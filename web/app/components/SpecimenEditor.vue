<script setup lang="ts">
// The specimen: the font, a sample anyone can edit, and the count of the
// characters in that sample the font can actually draw.
//
// The count is read from the font's character map rather than measured in the
// page, because a character the font is missing is handed to a fallback, and
// the fallback's numbers would pass for the font's.
import { coverage } from "~/utils/font";
import { useLemmataFont } from "~/composables/font";

const SAMPLES = {
  code: `// one plan in, one font out
let metrics = plan.metrics()?;
let font = build(&glyphs, &metrics)?;`,
  prose: `the quick brown fox jumps over the lazy dog
five or six big planes flew quickly overhead
a silk purse from a sow's ear, and no small beer`,
  figures: `0123456789
+1234.5678 -90% #42 3/4 (2019)
i. ii. iii. iv. v. 1st 2nd 3rd 4th`,
} as const;

type Kind = keyof typeof SAMPLES;

const font = useLemmataFont();
const kind = ref<Kind>("code");
const size = ref(22);
const sample = ref<string>(SAMPLES.code);
const presenting = ref(false);

const pick = (next: Kind): void => {
  kind.value = next;
  sample.value = SAMPLES[next];
};

const coverageLine = computed(() => {
  if (!font.map) return "Reading the font…";
  const { total, missing } = coverage(font.map, sample.value);
  if (!missing.length) return `all ${total} characters in this sample are in the font`;

  return `${total - missing.length} of ${total} characters in this sample are in the font · missing ${missing.join(" ")}`;
});

const covered = computed(
  () => !!font.map && !coverageLine.value.includes("missing"),
);

function togglePresent(): void {
  presenting.value = !presenting.value;
  document.body.style.overflow = presenting.value ? "hidden" : "";
}

function next(): void {
  const kinds = Object.keys(SAMPLES) as Kind[];
  pick(kinds[(kinds.indexOf(kind.value) + 1) % kinds.length] ?? "code");
}

onMounted(() => {
  addEventListener("keydown", (event: KeyboardEvent) => {
    if (!presenting.value) return;
    if (event.key === "Escape") togglePresent();
    if (event.key === " ") {
      event.preventDefault();
      next();
    }
  });
});
</script>

<template>
  <div class="panel">
    <div class="panel__head">
      <div class="segmented">
        <button
          v-for="name in Object.keys(SAMPLES)"
          :key="name"
          type="button"
          :aria-pressed="kind === name"
          @click="pick(name as Kind)"
        >
          {{ name }}
        </button>
      </div>
      <label class="size">
        size
        <input v-model.number="size" type="range" min="12" max="52" step="1" />
        <b>{{ size }}</b>
      </label>
    </div>
    <textarea
      v-model="sample"
      class="sample"
      rows="6"
      spellcheck="false"
      :style="{ fontSize: `${size}px` }"
    />
    <div class="panel__foot">
      <StatusLine :state="covered ? 'ok' : 'note'">{{ coverageLine }}</StatusLine>
      <button class="btn" id="present" type="button" @click="togglePresent">
        Present
      </button>
    </div>
  </div>

  <div v-if="presenting" class="stage" id="stage" tabindex="-1">
    <div class="stage__body">
      <p class="stage__sample">{{ sample }}</p>
    </div>
    <p class="stage__hint">space · next sample &nbsp; esc · done</p>
  </div>
</template>
