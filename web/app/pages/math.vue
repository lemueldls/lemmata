<script setup lang="ts">
// The equation bench: one editable MathML field, the whole corpus beside it,
// and the constructs the table is there to serve.
//
// The MathML is edited as text and set by the browser. There is no upload and no
// render step on another machine, so the equation on screen is the equation in
// the field.
import corpus from "../../../corpus/equations.mathml?raw";
import { specimenSize } from "~/composables/hotkeys";

useHead({ title: "Equations" });

const LABELS = [
  "addition",
  "mixed operators",
  "square of a sum",
  "fraction equality",
  "radical",
  "summation",
  "integral",
  "inequality",
  "nested radical",
  "fenced fraction",
  "greek letters",
  "matrix",
];

const blocks = (corpus.replace(/<!--[\s\S]*?-->/g, "").match(/<math[\s\S]*?<\/math>/g) ??
  []).map((math, index) => ({
  id: index,
  label: LABELS[index] ?? `equation ${index + 1}`,
  math,
}));

const selected = ref(5);
const source = ref(blocks[selected.value]?.math ?? "");
const copied = ref(false);
const status = ref("waiting for the math face…");
const state = ref("note");

const valid = computed(() => /<math[\s\S]*<\/math>/.test(source.value.trim()));

const current = computed(
  () => blocks[selected.value] ?? { label: "custom", math: "" },
);

function pick(id: number): void {
  selected.value = id;
  source.value = blocks[id]?.math ?? "";
  copied.value = false;
}

function reset(): void {
  source.value = current.value.math;
}

async function copy(): Promise<void> {
  try {
    await navigator.clipboard.writeText(source.value);
    copied.value = true;
    setTimeout(() => {
      copied.value = false;
    }, 1400);
  } catch {
    copied.value = false;
  }
}

const CONSTRUCTS = [
  { label: "delimiters", id: 9, note: "A fence grows to fit what it holds." },
  { label: "radicals", id: 4, note: "The radical takes the height of its body." },
  { label: "operators", id: 5, note: "Limits sit above and below the sign." },
  { label: "matrices", id: 11, note: "Rows and columns keep one baseline grid." },
];

onMounted(async () => {
  try {
    await document.fonts.load('24px "Lemmata Math"', "\u222B");
    if (!document.fonts.check('24px "Lemmata Math"', "\u222B")) {
      throw new Error("the math face is not in the font set");
    }
    state.value = "ok";
    status.value = `Ready. ${blocks.length} equations / the math face is loaded`;

    // Write the rendered boxes of the fences to the body dataset so the browser
    // suite can report them. Whether a renderer stretches a fence is its own
    // business, so this is a note rather than a pass condition.
    const boxes = [...document.querySelectorAll<HTMLElement>("#equations mo")]
      .filter((mo) => ["(", ")"].includes(mo.textContent ?? ""))
      .map((mo) => {
        const box = mo.getBoundingClientRect();

        return { width: Math.round(box.width), height: Math.round(box.height) };
      });
    document.body.dataset.measure = JSON.stringify(boxes.slice(0, 8));
  } catch {
    state.value = "bad";
    status.value = "the math face did not load; the browser fallback is showing";
  }
});
</script>

<template>
  <div class="wrap page">
    <header class="page-head">
      <div>
        <p class="eyebrow"><b>+</b> equations <i>/</i> 03</p>
        <h1 class="page-head__title">Set the <em>symbols.</em></h1>
        <p class="lede" style="margin-top: 18px">
          The same build that writes the text face writes a MATH table. This page
          edits MathML and sets it in that face: constants, italic corrections and
          delimiters that grow to fit.
        </p>
      </div>

      <div class="page-head__side">
        <p class="mono-note">
          the field is live. change a tag or a number and the equation is set
          again in the same face.
        </p>
        <dl class="page-head__facts">
          <div><dt>corpus</dt><dd>{{ blocks.length }} equations</dd></div>
          <div><dt>source</dt><dd>mathml</dd></div>
          <div><dt>face</dt><dd>lemmata math</dd></div>
          <div><dt>part</dt><dd>MATH table</dd></div>
        </dl>
      </div>
    </header>

    <div class="math-layout">
      <aside class="math-index">
        <h2>corpus</h2>
        <button
          v-for="block in blocks"
          :key="block.id"
          type="button"
          :aria-pressed="selected === block.id"
          @click="pick(block.id)"
        >
          <span>{{ String(block.id + 1).padStart(2, "0") }}</span>
          <span>{{ block.label }}</span>
        </button>
      </aside>

      <div class="math-main">
        <PlusPlate index="01" label="render" meta="live mathml">
          <div class="section-intro section-intro--single">
            <div>
              <p class="eyebrow"><b>+</b> the field</p>
              <h2 class="display" style="font-size: var(--step-3)">
                {{ current.label }}
              </h2>
            </div>
            <p class="prose">
              The left side is the source, the right side is the browser setting
              it. Both use the same MathML, so there is nothing to compile and
              nothing to upload.
            </p>
          </div>

          <div class="math-editor">
            <div class="math-editor__source">
              <textarea
                v-model="source"
                rows="10"
                aria-label="MathML source"
              />
            </div>

            <div
              v-if="valid"
              class="math-editor__render"
              :style="{ fontSize: `${Math.min(44, Math.max(16, specimenSize))}px` }"
              v-html="source"
            />
            <div v-else class="math-editor__render">
              <p class="note">The field needs a math element with a closing tag.</p>
            </div>
          </div>

          <div class="form-row">
            <span id="status" class="status-line" :data-state="state">
              <i /> {{ status }}
            </span>
            <div class="row">
              <button class="btn btn--sm" type="button" :disabled="!valid" @click="copy">
                {{ copied ? "copied" : "copy mathml" }}
              </button>
              <button class="btn btn--sm" type="button" @click="reset">reset</button>
            </div>
          </div>
        </PlusPlate>

        <PlusPlate index="02" label="corpus" meta="click one to edit">
          <div class="section-intro section-intro--single">
            <div>
              <p class="eyebrow"><b>+</b> the set</p>
              <h2 class="display" style="font-size: var(--step-3)">
                Twelve equations.
              </h2>
            </div>
            <p class="prose">
              The same list the corpus holds: addition, exponents, fractions,
              radicals, a sum, an integral, an inequality, Greek letters and a
              matrix. Pick one and it moves into the field above.
            </p>
          </div>

          <div id="equations" class="math-gallery">
            <button
              v-for="block in blocks"
              :key="block.id"
              class="eq-card eq-card--button"
              type="button"
              :aria-pressed="selected === block.id"
              @click="pick(block.id)"
            >
              <span class="eq-card__bar">
                <span>{{ block.label }}</span>
                <span>{{ String(block.id + 1).padStart(2, "0") }}</span>
              </span>
              <span class="eq-card__body" v-html="block.math" />
            </button>
          </div>
        </PlusPlate>

        <PlusPlate index="03" label="constructs" meta="what the table holds">
          <div class="section-intro section-intro--single">
            <div>
              <p class="eyebrow"><b>+</b> the parts</p>
              <h2 class="display" style="font-size: var(--step-3)">
                What the table covers.
              </h2>
            </div>
            <p class="prose">
              The constants place the parts. The variants give a delimiter a size
              for each step. The assemblies stack the pieces when a size alone is
              not enough.
            </p>
          </div>

          <div class="constructs">
            <article v-for="item in CONSTRUCTS" :key="item.label" class="construct">
              <div class="construct__math" v-html="blocks[item.id]?.math ?? ''" />
              <h3>{{ item.label }}</h3>
              <p>{{ item.note }}</p>
            </article>
          </div>

          <p class="mono-note" style="margin-top: 14px">
            script style (<code>ssty</code>) and the flat accent (<code>flac</code>) are the two pieces
            still waiting on the glyph set.
          </p>
        </PlusPlate>
      </div>
    </div>
  </div>
</template>
