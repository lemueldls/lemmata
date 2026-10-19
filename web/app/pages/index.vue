<script setup lang="ts">
// The landing: the font this site is set in, the plan behind it, the whole
// character set, a mock interface, examples, the healing, the numbers and an
// equation. Every number on it comes out of the font itself.
//
// Stub: spec.optical-sizes and spec.script-style below have nothing behind them;
// the variant picker and the ligatures are stubs in `generate.vue`, where a
// reader would look for them.
import { useLemmataFont } from "~/composables/font";

useHead({
  title: "A typeface built from a plan",
  // The cover is up until the font is read, and the scripts that read it are the
  // only thing that can take it away. Without them it has to go by the rule.
  noscript: [{ innerHTML: "<style>.boot{display:none}</style>" }],
});

const font = useLemmataFont();

/// The reference plan, which is what `Plan::default()` holds.
const PLAN = {
  capHeight: 735,
  xHeight: 520,
  ascender: 965,
  descender: -215,
  italicAngle: 0,
  weight: 400,
};

/// Sentences to set the font in: short enough for the plate, and each one says
/// something the page then shows.
const SAMPLES = [
  "one width for every letter",
  "the ink moves, the cell does not",
  "one plan in, one font out",
  "the same bytes either way",
];

/// What the typeface holds, in the order a mechanism is explained.
const FEATURES = [
  { name: "Metrics", note: "Every key in the plan goes to the font's tables." },
  { name: "Parameters", note: "The parameter tables, read as Iosevka writes them." },
  { name: "Variants", note: "Every letter in as many spellings as you care to compare." },
  {
    name: "Texture healing",
    note: "A letter that leaves its cell empty gives the room to the neighbour that fills one, and the cell is where it was.",
  },
  {
    name: "Math",
    note: "The constants, the italic corrections, and the delimiters that grow to fit.",
  },
  { name: "Ligatures", note: "The sets a coding face is expected to have." },
  { name: "Optical sizes", note: "A face for footnotes and a face for display, from one plan." },
  { name: "Script style", note: "Letters drawn for the size of the body they sit in." },
];

const status = computed(() => {
  if (font.error) return `The font could not be read: ${font.error}`;
  const metrics = font.metrics;
  if (!metrics) return "Reading the font…";

  return (
    `${metrics.drawn.toLocaleString()} glyphs · ${metrics.mapped} characters · ` +
    `fingerprint ${font.digest.slice(0, 12)} · ready in ${Math.round(font.elapsed)} ms`
  );
});
</script>

<template>
  <BootCover />
  <section class="hero" data-key-section>
    <div class="wrap">
      <p class="kicker">A plan, a font, a whole character set</p>
      <HeroWord />
      <p class="lede">
        This page is set in the font it describes. The plan is the input, the font
        is the output, and the numbers beside them come out of the font itself.
      </p>
      <div class="actions">
        <a class="btn" href="#letters">See the font</a>
        <NuxtLink class="btn btn--primary" to="/generate/">Open the generator</NuxtLink>
      </div>
      <StatusLine :state="font.ready ? 'ok' : 'note'">{{ status }}</StatusLine>
      <SpecimenPlate :samples="SAMPLES" face="Lemmata Regular" :size="40" />
    </div>
  </section>

  <SectionBlock number="01" label="Plan">
    <h2>Everything comes from the plan</h2>
    <p>
      Every key defaults, so this is the reference plan, and every key it holds
      goes to the font.
    </p>
    <div class="cols">
      <PlanCard :plan="PLAN" />
      <div class="panel">
        <div class="panel__head"><span>What the typeface holds</span></div>
        <div class="panel__body"><FeatureList :rows="FEATURES" /></div>
      </div>
    </div>
  </SectionBlock>

  <SectionBlock id="letters" number="02" label="Letters">
    <h2>The whole character set</h2>
    <p>
      A monospace face is a grid, and the grid is the design: one advance for
      every letter, with the drawing moving inside it.
    </p>
    <div class="rule-art"><i /></div>
    <LetterGrid />
    <div class="rule-art"><i /></div>
    <p>And the same font doing a job, with the sizes a page would give it.</p>
    <MockUi />
  </SectionBlock>

  <SectionBlock number="03" label="Examples">
    <h2>One card per kind of text</h2>
    <p>
      Every sample is written in the characters the font holds, which is the
      letters, the digits and some punctuation, so nothing here borrows another
      face without saying so.
    </p>
    <ExamplesGrid />
  </SectionBlock>

  <SectionBlock number="04" label="Healing">
    <h2>Texture healing</h2>
    <p>
      A cell is one width for every letter, so the letters that leave it empty sit
      beside the ones that fill it. Healing moves the drawing within the cell and
      gives the room to the neighbour that wants it.
    </p>
    <HealingPlate />
  </SectionBlock>

  <SectionBlock id="numbers" number="05" label="Numbers">
    <h2>The numbers</h2>
    <p>
      The font is fingerprinted in the browser, and the number it gives is the one
      the reference build gives. The readings beside it come out of the font's own
      tables.
    </p>
    <ChecksumPanel />
  </SectionBlock>

  <SectionBlock id="math" number="06" label="Equations">
    <h2>Equations, set in the font</h2>
    <p>
      The same build writes a MATH table: the constants, the italic corrections,
      and the delimiters that grow to fit. These are set in a second face, made
      from the math glyph set when the section comes into view.
    </p>
    <EquationPlate />
  </SectionBlock>
</template>
