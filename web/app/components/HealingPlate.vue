<script setup lang="ts">
// Texture healing, from the font rather than from the page: the same text
// twice, with `calt` off in one of them.
//
// The readout is the claim, measured. Both lines are the same text, so the only
// change the setting can make to the layout is none, and the number says so.
// `calt` is on by default, which is why the line that shows the healing is the
// one with nothing set on it.
import { useLemmataFont } from "~/composables/font";

const SAMPLE = "millim minimum willow filial";

const font = useLemmataFont();
const lines = ref<HTMLElement | null>(null);
const plain = ref<HTMLElement | null>(null);
const healed = ref<HTMLElement | null>(null);
const cell = ref("12px");
const measure = ref("Reading the font…");

function draw(): void {
  const metrics = font.metrics;
  if (!metrics || !lines.value || !plain.value || !healed.value) return;
  const points = Number.parseFloat(getComputedStyle(plain.value).fontSize);
  const width = (element: HTMLElement): number => {
    const range = document.createRange();
    range.selectNodeContents(element);

    return range.getBoundingClientRect().width;
  };
  const moved = width(healed.value) - width(plain.value);
  const advance = metrics.advance ?? 0;
  const size = (advance / 1000) * points;
  cell.value = `${size}px`;
  measure.value =
    `${SAMPLE.length} cells of ${Math.round(size)} px · ` +
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
      <div class="heal-line">
        <span class="heal-label">calt off</span>
        <p class="heal-text heal-text--plain" ref="plain">{{ SAMPLE }}</p>
      </div>
      <div class="heal-line">
        <span class="heal-label">calt on</span>
        <p class="heal-text" ref="healed">{{ SAMPLE }}</p>
      </div>
    </div>
    <div class="panel__body">
      <p class="note">
        A monospace cell is one width for every letter, so the letters that leave
        the cell empty sit beside the ones that fill it. Healing moves the
        drawing within the cell and gives the space to the neighbour that wants
        it, which is a sixteenth of the advance and never the advance itself.
        Both lines end in the same column, and the grid is what shows it.
      </p>
    </div>
    <div class="panel__foot">
      <StatusLine state="ok">{{ measure }}</StatusLine>
    </div>
  </div>
</template>
