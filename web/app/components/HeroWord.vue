<script setup lang="ts">
// The wordmark, set to the width of the measure, in the font the site ships, on
// a grid of one cell per advance.
//
// The rules are the font's own ascender, x-height and baseline, drawn where the
// font puts them: the baseline comes from a probe sitting on it rather than from
// the ascent and the half-leading, and the other two are the ink heights read
// out of the tables.
import { useLemmataFont } from "~/composables/font";

const font = useLemmataFont();

const stage = ref<HTMLElement | null>(null);
const word = ref<HTMLHeadingElement | null>(null);
const probe = ref<HTMLElement | null>(null);
const shown = ref(false);
const ascender = ref<string | number>("");
const xHeight = ref<string | number>("");

interface Fitted {
  points: number;
  cell: number;
}

/// Sets the wordmark to what is left of the measure once the labels have their
/// column, and answers the size it landed on and the width of one letter.
///
/// The cell is the rendered width divided by the number of characters rather
/// than the font's advance on its own. The wordmark is set with negative
/// tracking, and a cell that ignores it drifts a little per letter and most of a
/// letter by the last one. Taking it from the text means any later change to the
/// tracking is already in the number.
function fit(): Fitted | null {
  const element = word.value;
  const holder = stage.value;
  if (!element || !holder || !holder.clientWidth) return null;
  const text = element.firstChild;
  if (!text) return null;
  const characters = text.textContent?.length ?? 0;
  if (!characters) return null;
  element.style.fontSize = "100px";
  const range = document.createRange();
  range.selectNode(text);
  const width = range.getBoundingClientRect().width;
  if (!width) return null;
  const room = Math.max(holder.clientWidth - 132, holder.clientWidth * 0.55);
  const points = Math.min((100 * room) / width, 340);
  element.style.fontSize = `${points}px`;

  return { points, cell: room / characters };
}

function draw(): void {
  const metrics = font.metrics;
  const holder = stage.value;
  if (!metrics || !holder) return;
  const fitted = fit();
  if (!fitted) return;
  const { points, cell } = fitted;

  // The grid follows the size, so the letters stay on their cells.
  holder.style.setProperty("--cell", `${cell}px`);

  // On a narrow screen the wordmark fills the measure and the labels would sit
  // on the letters, so the rules go away rather than lie across them.
  if (holder.clientWidth < 520) {
    shown.value = false;
    return;
  }
  if (metrics.ascender === null || metrics.xHeight === null || !probe.value) {
    return;
  }

  const scale = points / 1000;
  const baseline =
    probe.value.getBoundingClientRect().bottom -
    holder.getBoundingClientRect().top;
  const place = (selector: string, top: number): void => {
    const rule = holder.querySelector(selector) as HTMLElement | null;
    if (rule) rule.style.top = `${top}px`;
  };
  place(".rule--asc", baseline - metrics.ascender * scale);
  place(".rule--x", baseline - metrics.xHeight * scale);
  place(".rule--base", baseline);
  ascender.value = metrics.ascender;
  xHeight.value = metrics.xHeight;
  shown.value = true;
}

watch(
  () => font.ready,
  (ready) => {
    if (ready) nextTick(draw);
  },
);
onMounted(() => {
  addEventListener("resize", draw);
  if (font.ready) nextTick(draw);
});
onBeforeUnmount(() => removeEventListener("resize", draw));
</script>

<template>
  <div class="hero__stage" ref="stage">
    <div class="hero__grid" aria-hidden="true" />
    <h1 class="wordmark" ref="word">lemmata<i class="probe" ref="probe" aria-hidden="true" /></h1>
    <div v-show="shown" class="rules">
      <i class="rule rule--asc"><span>ascender <b>{{ ascender }}</b></span></i>
      <i class="rule rule--x"><span>x-height <b>{{ xHeight }}</b></span></i>
      <i class="rule rule--base"><span>baseline</span></i>
    </div>
  </div>
</template>
