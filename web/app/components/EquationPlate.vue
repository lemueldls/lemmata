<script setup lang="ts">
// One equation from the corpus, set in the math face. The face is a second font
// the site ships, so this page waits for it rather than building it, and the
// stylesheet only reaches for the family once the holder says so, which is what
// keeps the download out of the way until the section is in sight.
//
// The corpus sits in a hidden div, and the browsers that parse it have already
// made MathML of it, so the equation comes out of there rather than from a
// second parse.
import corpus from "../../../corpus/equations.mathml?raw";

const holder = ref<HTMLElement | null>(null);
const status = ref("Waiting for the section to come into view.");
const state = ref("note");

async function show(): Promise<void> {
  const target = holder.value;
  if (!target) return;
  try {
    status.value = "Setting the equation…";
    // Using the family is what asks for the file, so the attribute goes on first
    // and the load is asked for by the name the element resolves to.
    target.dataset.face = "ready";
    const primary = getComputedStyle(target).fontFamily.split(",")[0]?.trim() ?? "";
    await document.fonts.load(`24px ${primary}`, "\u222B");
    const source = document.querySelector("#corpus");
    const found = source ? [...source.querySelectorAll("math")] : [];
    if (!found.length) throw new Error("the corpus in the page carries no equations");
    const pick =
      found.find((node) => node.textContent?.includes("\u222B")) ?? found[0];
    if (pick) target.append(pick);
    state.value = "ok";
    status.value = `Ready. ${found.length} in the corpus, this one set in the font built from the math glyph set.`;
  } catch (error) {
    state.value = "bad";
    status.value = `The math face failed: ${error instanceof Error ? error.message : String(error)}`;
  }
}

onMounted(() => {
  const section = document.querySelector("#corollary");
  if (!section) return;
  const observer = new IntersectionObserver(
    (entries) => {
      if (!entries.some((entry) => entry.isIntersecting)) return;
      observer.disconnect();
      void show();
    },
    { rootMargin: "300px" },
  );
  observer.observe(section);
});
</script>

<template>
  <div class="panel">
    <div class="panel__body">
      <div class="equation" id="equation" ref="holder" />
    </div>
    <div class="panel__foot">
      <StatusLine :state="state">{{ status }}</StatusLine>
      <NuxtLink class="btn" to="/math/">Set your own</NuxtLink>
    </div>
  </div>
  <div hidden id="corpus" v-html="corpus" />
</template>
