<script setup lang="ts">
// The landing: the plan it starts from, the font it ships, the numbers that tie
// the two together, and one equation set in it.
//
// Stubs, and what they want. Grep for "Stub:" when one is finished.
//
//   Stub: spec.optical-sizes  the plan has no optical size key behind this row
//   Stub: spec.script-style   needs the script letters, which the glyph source lacks
import { useLemmataFont } from "~/composables/font";

useHead({
  title: "A typeface built from a plan",
  // The cover stays until the font is read, and the scripts that read it are the
  // only thing that can take it away. Without them it has to go by the rule.
  noscript: [{ innerHTML: "<style>.boot{display:none}</style>" }],
});

const font = useLemmataFont();

/// The reference plan, which is what `Plan::default()` carries. It is content on
/// this page the way the samples are: the object the rest is built from.
const PLAN = {
  capHeight: 735,
  xHeight: 520,
  ascender: 965,
  descender: -215,
  italicAngle: 0,
  weight: 400,
};

/// Sentences to set the font in. Every one of them is written in the characters
/// the capture carries, each says something the page then demonstrates, and each
/// is short enough to fit the plate at the size it is set.
const SAMPLES = [
  "one width for every letter",
  "the ink moves, the cell does not",
  "one plan in, one font out",
  "the same bytes either way",
];

/// What the typeface carries, in the order a mechanism is explained.
const FEATURES = [
  { name: "Metrics", note: "Every key in the plan reaches the font's tables." },
  { name: "Parameters", note: "The parameter tables, read as upstream writes them." },
  { name: "Variants", note: "Every letter in as many spellings as you care to compare." },
  {
    name: "Texture healing",
    note: "A narrow letter gives its sidebearings to a wide neighbour, and the cell stays where it was.",
  },
  {
    name: "Math",
    note: "The constants, the italic corrections, and the delimiters that grow to fit.",
  },
  { name: "Optical sizes", note: "A cut for footnotes and a cut for display, from one plan." },
  { name: "Script style", note: "Letters drawn for the size of the body they sit in." },
];

const status = computed(() => {
  if (font.error) return `The font could not be read: ${font.error}`;
  const metrics = font.metrics;
  if (!metrics) return "Reading the font…";

  return (
    `${metrics.drawn.toLocaleString()} glyphs · ${metrics.mapped} mapped · ` +
    `sha256 ${font.digest.slice(0, 12)} · ready in ${Math.round(font.elapsed)} ms`
  );
});
</script>

<template>
  <BootCover />
  <section class="hero">
    <div class="wrap">
      <p class="kicker">A plan, a font, and the numbers to check it</p>
      <HeroWord />
      <p class="lede">
        This page is set in the font it describes. The plan is the input, the
        font is the output, and both are here so that the result can be checked:
        the file is hashed in the browser and read back out of its own tables.
      </p>
      <div class="actions">
        <a class="btn" href="#construction">See the font</a>
        <NuxtLink class="btn btn--primary" to="/generate/">Open the generator</NuxtLink>
      </div>
      <StatusLine :state="font.ready ? 'ok' : 'note'">{{ status }}</StatusLine>
      <SpecimenPlate :samples="SAMPLES" face="Lemmata Regular" :size="40" />
    </div>
  </section>

  <SectionBlock number="01" label="Plan">
    <h2>Everything is built from the plan</h2>
    <p>
      Every key defaults, so this is the reference plan, and every key it carries
      reaches the font.
    </p>
    <div class="cols">
      <PlanCard :plan="PLAN" />
      <div class="panel">
        <div class="panel__head"><span>What the plan carries</span></div>
        <div class="panel__body"><FeatureList :rows="FEATURES" /></div>
      </div>
    </div>
  </SectionBlock>

  <SectionBlock id="construction" number="02" label="Font">
    <h2>The font, set in itself</h2>
    <p>
      This is the font this site ships, and every reading further down comes out
      of its own tables. Edit the sample and it stays the font.
    </p>
    <SpecimenEditor />

    <SubHead title="Texture healing" />
    <HealingPlate />

    <SubHead title="Spellings of a">
      <p class="note">Pick one and the generator draws it.</p>
    </SubHead>
    <VariantPicker />

    <SubHead title="Every character it carries">
      <p class="note">
        {{ font.map?.size ?? "…" }} characters ·
        {{ font.metrics?.drawn ?? "…" }} glyphs drawn
      </p>
    </SubHead>
    <GlyphSheet />
  </SectionBlock>

  <SectionBlock id="checksums" number="03" label="Checksums">
    <h2>The build checks itself</h2>
    <p>
      The font is hashed in the browser, and the number it produces is the number
      the reference build recorded. The readings beside it come from the font's
      own tables, measured rather than assumed.
    </p>
    <ChecksumPanel />
  </SectionBlock>

  <SectionBlock id="math" number="04" label="Equations">
    <h2>Equations, set in the font it built</h2>
    <p>
      The same build writes a MATH table: the constants, the italic corrections,
      and the delimiters that grow to fit. This one is set in a second face,
      built from the math glyph set when the section comes into view.
    </p>
    <EquationPlate />
  </SectionBlock>
</template>
