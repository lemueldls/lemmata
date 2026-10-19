<script setup lang="ts">
// Texture healing, as the font sets it and as it sets it with `calt`.
//
// The change is a sixteenth of a cell, which is two or three pixels at a reading
// size. Two columns of that look identical and two rows have to be remembered,
// so this is one set of words that changes setting instead, which is motion, and
// the eye reads a three pixel jump far better than it reads a static offset.
//
// The plate alternates on its own while it is on screen, because a reader who
// touches nothing should still see the change happen. Working the control takes
// it over and stops the alternation for good, and a reader who has asked for
// reduced motion never gets it at all.
import { useLemmataFont } from "~/composables/font";

const SAMPLES = ["mi", "im", "mim", "millim"];

/// How long each setting is held while the plate is taking turns.
const HOLD = 2200;

const font = useLemmataFont();
const lines = ref<HTMLElement | null>(null);
const probe = ref<HTMLElement | null>(null);
const healed = ref(false);
/// Set once a reader works the control. The alternation is there to show the
/// change, not to keep showing it.
const taken = ref(false);
const onScreen = ref(false);
const cell = ref("12px");
const reading = ref("Reading the font…");

let timer: ReturnType<typeof setInterval> | undefined;
let watcher: IntersectionObserver | undefined;

function width(element: HTMLElement): number {
  const range = document.createRange();
  range.selectNodeContents(element);

  return range.getBoundingClientRect().width;
}

function draw(): void {
  const metrics = font.metrics;
  const word = lines.value?.querySelector<HTMLElement>(".heal-word");
  const spans = probe.value?.querySelectorAll<HTMLElement>("span");
  const off = spans?.[0];
  const on = spans?.[1];
  if (!metrics || !word || !off || !on) return;
  const points = Number.parseFloat(getComputedStyle(word).fontSize);
  cell.value = `${((metrics.advance ?? 0) / 1000) * points}px`;
  const moved = width(on) - width(off);
  reading.value =
    `${SAMPLES.length} words at ${Math.round(points)} px · ` +
    `the advance moves ${moved.toFixed(2)} px`;
}

/// Whether the alternation is off the table, which it is once a reader has
/// chosen a setting or asked for less motion.
function still(): boolean {
  return taken.value || matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function hold(): void {
  if (timer !== undefined || still() || !onScreen.value) return;
  timer = setInterval(() => {
    healed.value = !healed.value;
  }, HOLD);
}

function release(): void {
  if (timer === undefined) return;
  clearInterval(timer);
  timer = undefined;
}

function choose(state: boolean): void {
  taken.value = true;
  release();
  healed.value = state;
}

watch(
  () => font.ready,
  (ready) => {
    if (ready) nextTick(draw);
  },
);

onMounted(() => {
  addEventListener("resize", draw);
  // Taking turns is worth nothing in a section nobody has scrolled to, so it
  // runs only while the plate is on screen. The pointer pauses it too: a reader
  // looking closely is not a reader who wants it moving.
  watcher = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      onScreen.value = entry.isIntersecting;
      if (entry.isIntersecting) hold();
      else release();
    }
  });
  if (lines.value) watcher.observe(lines.value);
});

onBeforeUnmount(() => {
  release();
  watcher?.disconnect();
  removeEventListener("resize", draw);
});
</script>

<template>
  <div
    class="panel"
    @pointerenter="release"
    @pointerleave="hold"
    @focusin="release"
    @focusout="hold"
  >
    <div class="panel__head">
      <span>Contextual alternates</span>
      <div class="segmented" role="group" aria-label="Contextual alternates">
        <button type="button" :aria-pressed="!healed" @click="choose(false)">
          off
        </button>
        <button type="button" :aria-pressed="healed" @click="choose(true)">
          on
        </button>
      </div>
    </div>
    <div class="heal-lines" ref="lines" :style="{ '--cell': cell }">
      <p
        v-for="sample in SAMPLES"
        :key="sample"
        class="heal-word grid-field"
        :data-calt="healed ? 'on' : 'off'"
      >
        {{ sample }}
      </p>
      <p class="heal-probe" ref="probe" aria-hidden="true">
        <span data-calt="off">{{ SAMPLES[0] }}</span><span
          data-calt="on"
        >{{ SAMPLES[0] }}</span>
      </p>
    </div>
    <div class="panel__body">
      <p class="note">
        A cell is one width for every letter, so the letters that leave it empty
        sit beside the ones that fill it. Healing moves the drawing within the
        cell and gives the room to the neighbour that wants it: a sixteenth of the
        advance, and never the advance itself. The plate takes turns while it is
        on screen, and the control holds it where you put it.
      </p>
    </div>
    <div class="panel__foot">
      <StatusLine state="ok">{{ reading }}</StatusLine>
    </div>
  </div>
</template>
