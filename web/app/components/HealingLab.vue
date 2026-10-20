<script setup lang="ts">
// Texture healing as two aligned rows and one magnified pair.
//
// The rows share the same word, size and cell width, so the ink can be compared
// without one drawing covering the other. The grid line under the word comes
// from the font's own advance and the word has no tracking, so every cell
// boundary lands on the advance the renderer uses.
import { useLemmataFont } from "~/composables/font";

const WORDS = ["calming", "balm", "overwhelm", "palmtop"];

const font = useLemmataFont();
const word = ref(WORDS[0] as string);
const cell = ref("64px");
const detailCell = ref("64px");
const line = ref<HTMLElement | null>(null);
const detailLine = ref<HTMLElement | null>(null);

const advance = computed(() => font.metrics?.advance ?? 500);

const pair = computed(() => {
  const text = word.value;
  for (let index = 0; index < text.length - 1; index += 1) {
    const here = text[index];
    const next = text[index + 1];
    if ((here === "l" && next === "m") || (here === "m" && next === "l")) {
      return index;
    }
  }

  return 0;
});

const pairText = computed(() => word.value.slice(pair.value, pair.value + 2));

function measure(element: HTMLElement | null): string | null {
  if (!element) return null;
  const size = Number.parseFloat(getComputedStyle(element).fontSize);

  return `${((advance.value / 1000) * size).toFixed(3)}px`;
}

function draw(): void {
  const main = measure(line.value);
  const detail = measure(detailLine.value);
  if (main) cell.value = main;
  if (detail) detailCell.value = detail;
}

function choose(sample: string): void {
  word.value = sample;
}

watch([() => font.ready, word, advance], () => nextTick(draw));

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
          @click="choose(sample)"
        >
          {{ sample }}
        </button>
      </div>
      <span class="heal__delta">advance {{ advance }} units / cell {{ cell }}</span>
    </div>

    <div class="heal__demo" :style="{ '--cell': cell, '--chars': word.length }">
      <div class="heal__rows">
        <div class="heal__ruler" aria-hidden="true">
          <span v-for="(_, index) in word" :key="index">{{ index + 1 }}</span>
        </div>

        <div class="heal__row">
          <div class="heal__row-head">
            <span>texture off</span>
            <span>the cell keeps its room</span>
          </div>
          <div class="heal__stage">
            <i
              class="heal__band heal__band--off"
              :style="{ left: `calc(${pair} * var(--cell))` }"
              aria-hidden="true"
            />
            <i
              class="heal__band heal__band--off"
              :style="{ left: `calc(${pair + 1} * var(--cell))` }"
              aria-hidden="true"
            />
            <p ref="line" class="heal__word" data-txtr="off">{{ word }}</p>
          </div>
        </div>

        <div class="heal__row">
          <div class="heal__row-head">
            <span>texture on</span>
            <span>the ink moves, the advance does not</span>
          </div>
          <div class="heal__stage">
            <i
              class="heal__band heal__band--on"
              :style="{ left: `calc(${pair} * var(--cell))` }"
              aria-hidden="true"
            />
            <i
              class="heal__band heal__band--on"
              :style="{ left: `calc(${pair + 1} * var(--cell))` }"
              aria-hidden="true"
            />
            <p class="heal__word" data-txtr="on">{{ word }}</p>
          </div>
        </div>
      </div>

      <aside class="heal__detail">
        <div class="heal__detail-head">
          <span>the pair</span>
          <span>{{ pairText }}</span>
        </div>

        <div class="heal__pair-key" aria-hidden="true">
          <span>{{ pairText[0] === "l" ? "l gives room" : "m takes room" }}</span>
          <span>{{ pairText[1] === "m" ? "m takes room" : "l gives room" }}</span>
        </div>

        <div class="heal__pair">
          <div class="heal__pair-cell">
            <span>off</span>
            <div
              class="heal__stage heal__stage--detail"
              :style="{ '--cell': detailCell, '--chars': 2 }"
            >
              <p
                ref="detailLine"
                class="heal__word heal__word--detail"
                data-txtr="off"
              >
                {{ pairText }}
              </p>
            </div>
          </div>

          <div class="heal__pair-cell">
            <span>on</span>
            <div
              class="heal__stage heal__stage--detail"
              :style="{ '--cell': detailCell, '--chars': 2 }"
            >
              <p class="heal__word heal__word--detail" data-txtr="on">
                {{ pairText }}
              </p>
            </div>
          </div>
        </div>

        <p class="note">
          The narrow letter gives its sidebearing to the wide one. The two cells
          are the same width in both rows.
        </p>
      </aside>
    </div>

    <p class="heal__hint">
      The rows share one cell grid. Compare the pair, or pick a word where the
      two letters sit together.
    </p>

    <ol class="heal__steps">
      <li>
        <b>01 / the cell</b>
        Every letter owns the same advance, so an <code>l</code> has more room
        than its ink needs and an <code>m</code> has less than it wants.
      </li>
      <li>
        <b>02 / the swap</b>
        <code>TXTR</code> watches the neighbours. The narrow letter pulls the
        edge it shares in, and the wide letter extends into what it gave up.
      </li>
      <li>
        <b>03 / the grid</b>
        The advance is fixed. Column two starts where it started before, and the
        line still scans as a grid.
      </li>
    </ol>
  </div>
</template>
