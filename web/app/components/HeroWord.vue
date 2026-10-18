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

/// Sets the wordmark to what is left of the measure once the labels have their
/// column, and answers the size it landed on.
function fit(): number {
  const element = word.value;
  const holder = stage.value;
  if (!element || !holder || !holder.clientWidth) return 0;
  const text = element.firstChild;
  if (!text) return 0;
  element.style.fontSize = "100px";
  const range = document.createRange();
  range.selectNode(text);
  const width = range.getBoundingClientRect().width;
  if (!width) return 0;
  const room = Math.max(holder.clientWidth - 132, holder.clientWidth * 0.55);
  const points = Math.min((100 * room) / width, 340);
  element.style.fontSize = `${points}px`;

  return points;
}

function draw(): void {
  const metrics = font.metrics;
  const holder = stage.value;
  if (!metrics || !holder) return;
  const points = fit();
  if (!points) return;

  // The grid follows the size, so the letters stay on their cells.
  const advance = metrics.advance;
  if (advance !== null) {
    holder.style.setProperty("--cell", `${(advance / 1000) * points}px`);
  }

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
