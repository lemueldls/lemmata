<script setup lang="ts">
// The specimen page: the face, the plan behind it, the cell, samples, healing,
// equations and a way into the configurator.
//
// Every number in the readouts comes from the font file the page loads, never
// from a table written here.
import { useLemmataFont } from "~/composables/font";
import { useHeroVariants } from "~/composables/hero";

useHead({ title: "Specimen" });

const font = useLemmataFont();
const {
  letter: heroLetters,
  families: heroFamilies,
  phases: heroPhases,
  notes: heroNotes,
  label: heroLabel,
} = useHeroVariants();

/// The reference plan, which is what the builder uses when a key is absent.
const PLAN = {
  capHeight: 735,
  xHeight: 520,
  ascender: 965,
  descender: -215,
  italicAngle: 0,
  weight: 400,
  spacing: "normal",
} as const;

const TICKER = [
  "one plan",
  "one file",
  "texture healing",
  "math table",
  "the ported drawings",
  "offline after load",
];

const FEATURES = [
  {
    name: "metrics",
    note: "cap height, x height, ascender, descender, weight and slope.",
    state: "ready" as const,
  },
  {
    name: "parameters",
    note: "Iosevka's parameter tables, read as they are written.",
    state: "ready" as const,
  },
  {
    name: "variants",
    note: "The ported drawings, previewed in the registry before a plan picks one.",
    state: "ready" as const,
  },
  {
    name: "texture healing",
    note: "a narrow letter gives room to its neighbour, and the advance holds.",
    state: "ready" as const,
  },
  {
    name: "math table",
    note: "constants, italic corrections and delimiters that grow.",
    state: "ready" as const,
  },
  {
    name: "ligature sets",
    note: "Iosevka's chaining rules under calt, drawn by the same preview font.",
    state: "ready" as const,
  },
  {
    name: "optical sizes",
    note: "a face for footnotes and a face for display, from one plan.",
    state: "next" as const,
  },
  {
    name: "script style",
    note: "letters drawn for the size of the body they sit in.",
    state: "next" as const,
  },
];

const status = computed(() => {
  if (font.error) return `the font did not load: ${font.error}`;
  if (!font.metrics) return "reading the font…";

  return `${font.metrics.drawn} glyphs / ${font.metrics.mapped} characters / ready in ${Math.round(font.elapsed)} ms`;
});

const readings = computed(() => {
  const metrics = font.metrics;

  return [
    ["glyphs", metrics?.drawn ?? "—"],
    ["characters", metrics?.mapped ?? "—"],
    ["x height", metrics?.xHeight ?? "—"],
    ["ascender", metrics?.ascender ?? "—"],
    ["descender", metrics?.descender ?? "—"],
    ["advance", metrics?.advance ?? "—"],
    ["units / em", metrics?.upem ?? "—"],
    ["file", font.byteLength ? `${font.byteLength.toLocaleString()} b` : "—"],
  ];
});
</script>

<template>
  <section class="hero" data-key-section>
    <div class="wrap hero__frame">
      <GridGuide :top="false" :bottom="false" />

      <div class="hero__inner">
        <div class="hero__main">
          <p class="eyebrow">
            <b>+</b> specimen 001 <i>/</i> {{ heroLabel }}
          </p>

          <h1 class="hero__word">
            <span
              v-for="(glyph, index) in heroLetters"
              :key="index"
              class="hero__letter"
              :class="{ 'hero__letter--note': heroNotes[index] }"
            >
              <span
                class="hero__ink"
                :class="{
                  'hero__ink--out': heroPhases[index] === 'out',
                  'hero__ink--in': heroPhases[index] === 'in',
                }"
                :style="{ fontFamily: heroFamilies[index] }"
                >{{ glyph }}</span
              >
            </span>
          </h1>

          <div class="hero__rule">
            <span>one plan</span>
            <i />
            <span>one file</span>
          </div>

          <p class="hero__lede">
            A typeface factory in a browser tab. Set the metrics, choose the
            drawings, build the file.
          </p>

          <div class="hero__actions">
            <NuxtLink class="btn btn--accent" to="/generate/">
              open configurator <span>→</span>
            </NuxtLink>
            <a class="btn btn--ghost" href="#samples">browse samples</a>
          </div>
        </div>

        <aside class="readout" aria-label="Readings from the font">
          <div class="readout__bar">
            <span class="dot" />
            live readings
            <span>+</span>
          </div>

          <dl class="readout__rows">
            <div v-for="[name, value] in readings" :key="name">
              <dt>{{ name }}</dt>
              <dd>{{ value }}</dd>
            </div>
          </dl>

          <div class="readout__cmd">
            <span><span class="prompt">$</span> lemmata build --plan plan.json</span>
            <span class="out">→ {{ font.byteLength.toLocaleString() }} bytes / {{ Math.round(font.elapsed) }} ms</span>
          </div>
        </aside>
      </div>
    </div>
  </section>

  <div class="ticker" aria-hidden="true">
    <div class="wrap ticker__frame">
      <GridGuide :top="false" :bottom="false" />
      <div class="ticker__track">
        <span v-for="item in TICKER" :key="item">{{ item }} <i>+</i></span>
      </div>
    </div>
  </div>

  <PlusPlate index="01" label="the machine" meta="plan / parameters / outlines / file">
    <div class="section-intro">
      <div>
        <p class="eyebrow"><b>+</b> what it is</p>
        <h2 class="display">The machine is <em>a plan.</em></h2>
      </div>
      <div>
        <p class="prose">
          A font is a set of decisions before it is a set of drawings. Cap height,
          x height, ascender, descender, weight, slope and spacing are the
          decisions this build starts from.
        </p>
        <p class="prose">
          The plan is one small object. Change a key, build again, and the tables
          move with it. The panel on the right is the reference plan, the one the
          builder uses when a key is absent.
        </p>
      </div>
    </div>

    <div class="split split--wide">
      <div class="stack">
        <FeatureMatrix :rows="FEATURES" />
        <PipelineMap />
      </div>
      <div class="stack stack--sm">
        <PlanCard :plan="PLAN" />
        <p class="mono-note">
          Seven keys. Omitted keys take the values above.
        </p>
      </div>
    </div>
  </PlusPlate>

  <PlusPlate id="letters" index="02" label="the cell" meta="glyph map / live">
    <div class="section-intro">
      <div>
        <p class="eyebrow"><b>+</b> the grid</p>
        <h2 class="display">One advance. <em>Every letter.</em></h2>
      </div>
      <div>
        <p class="prose">
          A monospaced face is a grid, and the grid is the design. Every letter
          gets the same cell. The drawing moves inside it, and the narrow letters
          leave room that the wide ones can use.
        </p>
      </div>
    </div>

    <MockWorkbench />

    <div class="form-row">
      <span class="mono-note">{{ status }}</span>
      <NuxtLink class="link-arrow" to="/generate/">
        open the configurator <span>→</span>
      </NuxtLink>
    </div>

    <hr class="hr" style="margin: 28px 0 22px" />

    <div class="row row--between" style="margin-bottom: 14px">
      <h3 class="display" style="font-size: var(--step-3)">The whole set</h3>
      <span class="mono-note">click a cell to copy its character</span>
    </div>
    <CharacterGrid />
  </PlusPlate>

  <PlusPlate id="samples" index="03" label="samples" meta="code / text / data">
    <div class="section-intro">
      <div>
        <p class="eyebrow"><b>+</b> in use</p>
        <h2 class="display">The same face, <em>doing work.</em></h2>
      </div>
      <div>
        <p class="prose">
          A wall of samples is the quickest test of a code face. The sources stay
          inside the outline set: lowercase, digits and the punctuation around
          them. Nothing here borrows another face without saying so.
        </p>
      </div>
    </div>

    <SampleMosaic />
  </PlusPlate>

  <PlusPlate id="healing" index="04" label="texture healing" meta="TXTR / before and after">
    <div class="section-intro">
      <div>
        <p class="eyebrow"><b>+</b> the bend</p>
        <h2 class="display">The grid bends. <em>The cell does not.</em></h2>
      </div>
      <div>
        <p class="prose">
          A monospaced cell is one width for every letter, so a narrow letter has
          more room than its ink needs and a wide one has less than it wants.
          Texture healing moves the ink within the cell and leaves the advance
          alone.
        </p>
      </div>
    </div>

    <HealingLab />
  </PlusPlate>

  <PlusPlate id="math" index="05" label="equations" meta="math table / mathml">
    <div class="section-intro">
      <div>
        <p class="eyebrow"><b>+</b> the table</p>
        <h2 class="display">Math with <em>a table behind it.</em></h2>
      </div>
      <div>
        <p class="prose">
          The same build writes a MATH table: the constants, the italic
          corrections and the delimiters that grow to fit their content. A browser
          can set the equation in the same face as the prose around it.
        </p>
        <p class="prose">
          The corpus is MathML, and the equations page lets you edit it. Change a
          symbol and it is set again without a reload.
        </p>
      </div>
    </div>

    <EquationGallery />

    <div class="form-row">
      <span class="mono-note">twelve equations in the corpus</span>
      <NuxtLink class="link-arrow" to="/math/">
        open the equation bench <span>→</span>
      </NuxtLink>
    </div>
  </PlusPlate>

  <PlusPlate index="06" label="configurator" meta="one plan / one file">
    <div class="cta">
      <div class="cta__grid" aria-hidden="true" />
      <div class="cta__inner">
        <div>
          <p class="eyebrow" style="color: color-mix(in oklab, var(--bg) 55%, var(--ink))">
            <b>+</b> the workshop
          </p>
          <h2 class="cta__title">your face,<br />not ours.</h2>
          <p class="cta__copy">
            Set the metrics, pick a drawing from the outline set, choose a
            spacing, and take the file. The browser does the build.
          </p>
          <div class="cta__actions">
            <NuxtLink class="btn btn--accent" to="/generate/">
              build a face <span>→</span>
            </NuxtLink>
            <NuxtLink class="btn btn--ghost" to="/math/">try an equation</NuxtLink>
          </div>
        </div>

        <div class="cta__cmd" aria-label="What the configurator edits">
          <div><span>plan</span><span>7 keys</span></div>
          <div><span>glyphs drawn</span><span>{{ font.metrics?.drawn ?? "—" }}</span></div>
          <div><span>registry</span><span>the ported set</span></div>
          <div><span>build</span><span>in the tab</span></div>
          <div><span>output</span><span>ttf / woff2</span></div>
        </div>
      </div>
    </div>
  </PlusPlate>
</template>

<style scoped>
/* Each letter is its own box so its occurrence can hold its own face. The
   word is monospaced, so nothing is lost between the boxes. */
.hero__letter {
  position: relative;
  display: inline-block;
  white-space: pre;
  perspective: 900px;
}

.hero__ink {
  display: inline-block;
  transform-origin: 50% 58%;
  backface-visibility: hidden;
}

/* A split-flap: the letter folds edge-on, the face changes in that frame, and
   it unfolds with the new drawing. The two drawings are never visible at the
   same time. */
.hero__ink--out {
  animation: hero-fold-out 240ms cubic-bezier(0.55, 0.05, 0.9, 0.6) forwards;
}

.hero__ink--in {
  animation: hero-fold-in 360ms cubic-bezier(0.12, 0.62, 0.28, 1) forwards;
}

@keyframes hero-fold-out {
  from {
    transform: rotateX(0deg) scale(1);
    opacity: 1;
  }
  to {
    transform: rotateX(-86deg) scale(0.985);
    opacity: 0.18;
  }
}

@keyframes hero-fold-in {
  from {
    transform: rotateX(86deg) scale(0.985);
    opacity: 0.18;
  }
  to {
    transform: rotateX(0deg) scale(1);
    opacity: 1;
  }
}

/* The annotation: an accent rule drawn under the letter that just changed,
   which names the change in the eyebrow at the same moment. */
.hero__letter--note::after {
  content: "";
  position: absolute;
  right: 10%;
  bottom: -0.02em;
  left: 10%;
  height: 0.022em;
  background: var(--accent);
  transform-origin: left center;
  animation: hero-note 900ms ease-out forwards;
  pointer-events: none;
}

@keyframes hero-note {
  0% {
    transform: scaleX(0);
    opacity: 1;
  }
  38% {
    transform: scaleX(1);
    opacity: 1;
  }
  100% {
    transform: scaleX(1);
    opacity: 0;
  }
}

@media (prefers-reduced-motion: reduce) {
  .hero__ink--out,
  .hero__ink--in,
  .hero__letter--note::after {
    animation: none;
  }
}
</style>
