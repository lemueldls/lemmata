<script setup lang="ts">
// The equations page. The face it is set in is one of the two the site serves, so
// the page has nothing to compile: it waits for the font and then sets MathML in
// it. The MathML is editable, which is the part worth playing with.
//
// What it can show is what the MATH table carries: the constants, the glyph info
// and the variants, so a delimiter grows to fit its content. Script style needs
// an `ssty` feature the font does not write, and math kerning and the horizontal
// constructions are unwritten, so a fraction's bar does not grow the way a
// parenthesis does. `docs/math.md` has the list.
import corpus from "../../../corpus/equations.mathml?raw";

useHead({ title: "Equations" });

const FAMILY = '"Lemmata Math"';

const status = ref("Setting the equations…");
/// The corpus with its own notes taken out: the field is for equations, and the
/// file's comments are for whoever reads the file.
const source = ref(corpus.replace(/<!--[\s\S]*?-->/g, "").trim());
const equations = ref<HTMLElement | null>(null);

/// Measures a string in a family, to tell whether a font rendered it.
function widthOf(name: string, text: string): number {
  const context = document.createElement("canvas").getContext("2d");
  if (!context) return 0;
  context.font = `32px ${name}`;

  return context.measureText(text).width;
}

// A renderer that cannot serve the corpus falls back, so the check is that the
// family changes the measurement. The sample is the constructs a math font has
// to carry: a radical, a large operator, an integral, Greek.
function measure(host: HTMLElement): void {
  const ours = widthOf(FAMILY, "\u2211 \u222B \u221A x \u03B1");
  const fallback = widthOf('"Lemmata Absent"', "\u2211 \u222B \u221A x \u03B1");
  const rendered = host.querySelectorAll("math").length;

  // A delimiter grown to fit its content is built by stacking pieces, so it
  // stays as wide as the glyph it was cut from, while a renderer with no
  // assembly scales the glyph and widens it. Both are recorded rather than
  // asserted: whether a browser stretches at all is the browser's business.
  const delimiters = [...host.querySelectorAll("mo")]
    .filter((mo) => mo.textContent === "(" || mo.textContent === ")")
    .map((mo) => {
      const box = mo.getBoundingClientRect();

      return { width: Math.round(box.width), height: Math.round(box.height) };
    });
  document.body.dataset.measure = JSON.stringify(delimiters);

  status.value =
    ours !== fallback && rendered > 0
      ? `Ready. ${rendered} equations.`
      : `Fallback. ${rendered} equations.`;
}

function render(): void {
  const host = equations.value;
  if (!host) return;
  host.innerHTML = source.value;
  measure(host);
}

onMounted(async () => {
  // The face is in the head with the rest of the page, so there is nothing to
  // build and nothing to decompress: the page waits for the font it is about to
  // use and then sets the corpus.
  await document.fonts.load(`24px ${FAMILY}`, "\u222B");
  render();
});
</script>

<template>
  <div class="wrap page">
    <h1>Equations</h1>
    <p class="lede">
      These are set in the font this project built from the math glyph set. The
      MathML is editable: change an equation and it is set again in the same
      font, with no upload.
    </p>
    <div class="rule-art"><i /></div>
    <p class="plate-label">The corpus</p>
    <p id="status">{{ status }}</p>
    <textarea id="source" v-model="source" rows="12" spellcheck="false" @input="render" />
    <div id="equations" ref="equations" />
    <noscript><p>The equations need JavaScript: they are set from the field above.</p></noscript>
  </div>
</template>

<style>
/* The equations are the point: they are set in the font this project built. Not
   `display: block` on the math elements, which would replace the `block math`
   the user agent stylesheet gives a `math[display="block"]` and lay every token
   out on a line of its own; the attribute on each `math` element is what puts it
   on its own line. */
#equations {
  font-family: "Lemmata Math", serif;
  font-size: 24px;
}

#equations math {
  margin: 0.5rem 0;
}

#status {
  color: var(--accent);
  font-size: var(--step--1);
}
</style>
