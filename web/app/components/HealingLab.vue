<script setup lang="ts">
// Texture healing, shown as two settings of the same word.
//
// The change happens inside the cell, so the advance is identical on both
// sides. The page says so with a width taken from the rendered line rather than
// with a claim. The cell rule behind each word comes from that same width, so
// the ink can be seen moving while the columns hold.
import { useLemmataFont } from "~/composables/font";

const WORDS = ["millim", "mim", "mammal", "illicit", "vacuum"];

const font = useLemmataFont();
const word = ref(WORDS[0] as string);
const cell = ref("64px");
const delta = ref("—");
const wordEl = ref<HTMLElement | null>(null);
const probe = ref<HTMLElement | null>(null);

function widthOf(element: HTMLElement): number {
  const range = document.createRange();
  range.selectNodeContents(element);

  return range.getBoundingClientRect().width;
}

function draw(): void {
  const line = wordEl.value;
  const hidden = probe.value;
  if (!line || !hidden) return;

  const size = getComputedStyle(line).fontSize;
  hidden.style.fontSize = size;
  const spans = hidden.querySelectorAll<HTMLElement>("span");
  const off = spans[0];
  const on = spans[1];
  if (!off || !on) return;

  const plain = widthOf(off);
  cell.value = `${plain / Math.max(word.value.length, 1)}px`;
  delta.value = `advance ${(widthOf(on) - plain).toFixed(2)} px`;
}

watch([() => font.ready, word], () => nextTick(draw));

onMounted(() => {
  addEventListener("resize", draw);
  if (font.ready) nextTick(draw);
});

onBeforeUnmount(() => removeEventListener("resize", draw));
</script>

<template>
  <div class="heal">
    <div class="heal__controls">
      <div class="heal__words" role="group" aria-label="Sample word">
        <button
          v-for="sample in WORDS"
          :key="sample"
          type="button"
          :aria-pressed="word === sample"
          @click="word = sample"
        >
          {{ sample }}
        </button>
      </div>
      <span class="heal__delta">{{ delta }} / cell {{ cell }}</span>
    </div>

    <div class="heal__pair" :style="{ '--cell': cell }">
      <div class="heal__side">
        <header><span>calt 0</span><span>off</span></header>
        <p ref="wordEl" class="heal__word" data-calt="off">{{ word }}</p>
        <p class="note">
          Each letter keeps the drawing it was given. The narrow letters leave
          their cells empty.
        </p>
      </div>
      <div class="heal__side heal__side--on">
        <header><span>calt 1</span><span>on</span></header>
        <p class="heal__word" data-calt="on">{{ word }}</p>
        <p class="note">
          The narrow letter cedes room and the wide one takes it. The advance
          does not move.
        </p>
      </div>
    </div>

    <p ref="probe" class="heal__probe" aria-hidden="true">
      <span data-calt="off">{{ word }}</span><span data-calt="on">{{ word }}</span>
    </p>

    <ol class="heal__steps">
      <li>
        <b>01 / the cell</b>
        Every letter owns the same advance, so an <code>l</code> has more room than its ink
        needs and an <code>m</code> has less than it wants.
      </li>
      <li>
        <b>02 / the swap</b>
        <code>calt</code> watches the neighbours. The narrow letter gives up a sidebearing
        and the wide letter takes room from it.
      </li>
      <li>
        <b>03 / the grid</b>
        The advance is fixed. Column two starts where it started before, and the
        line still scans as a grid.
      </li>
    </ol>
  </div>
</template>
