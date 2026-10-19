<script setup lang="ts">
// Texture healing, from the font rather than from the page: the same words twice,
// with `calt` off in one column and on in the other.
//
// Side by side rather than one above the other, because the change is a sixteenth
// of a cell: at this size that is a few pixels, and it is legible when the two are
// next to each other and invisible when a reader has to remember the first one.
// The words are short and dense with the letters that trade the room, so almost
// every character in them is doing something.
//
// The readout is the claim, as a number. Both columns are the same words, so the
// only change the setting can make to the layout is none.
import { useLemmataFont } from "~/composables/font";

const SAMPLES = ["mi", "im", "mim", "millim"];

const font = useLemmataFont();
const lines = ref<HTMLElement | null>(null);
const cell = ref("12px");
const reading = ref("Reading the font…");

function draw(): void {
  const metrics = font.metrics;
  const holder = lines.value;
  if (!metrics || !holder) return;
  // Both lines of one pair, so the two are the same word and what is compared is
  // the setting rather than the text.
  const pair = holder.querySelector<HTMLElement>(".heal-pair");
  const plain = pair?.querySelector<HTMLElement>(".heal-text--plain");
  const healed = pair?.querySelector<HTMLElement>(".heal-text:not(.heal-text--plain)");
  if (!pair || !plain || !healed) return;
  const width = (element: HTMLElement): number => {
    const range = document.createRange();
    range.selectNodeContents(element);

    return range.getBoundingClientRect().width;
  };
  const points = Number.parseFloat(getComputedStyle(plain).fontSize);
  const advance = metrics.advance ?? 0;
  const size = (advance / 1000) * points;
  cell.value = `${size}px`;
  const moved = width(healed) - width(plain);
  reading.value =
    `${SAMPLES.length} pairs at ${Math.round(points)} px · ` +
    `the advance moves ${moved.toFixed(2)} px`;
}

watch(
  () => font.ready,
  (ready) => {
    if (ready) nextTick(draw);
  },
);
onMounted(() => addEventListener("resize", draw));
onBeforeUnmount(() => removeEventListener("resize", draw));
</script>

<template>
  <div class="panel">
    <div class="heal-lines" ref="lines" :style="{ '--cell': cell }">
      <div class="heal-head">
        <span class="heal-label">calt off</span>
        <span class="heal-label">calt on</span>
      </div>
      <div v-for="sample in SAMPLES" :key="sample" class="heal-pair">
        <p class="heal-text heal-text--plain grid-field">{{ sample }}</p>
        <p class="heal-text grid-field">{{ sample }}</p>
      </div>
    </div>
    <div class="panel__body">
      <p class="note">
        A cell is one width for every letter, so the letters that leave it empty
        sit beside the ones that fill it. Healing moves the drawing within the
        cell and gives the room to the neighbour that wants it: a sixteenth of the
        advance, and never the advance itself. Every pair ends in the same place.
      </p>
    </div>
    <div class="panel__foot">
      <StatusLine state="ok">{{ reading }}</StatusLine>
    </div>
  </div>
</template>
