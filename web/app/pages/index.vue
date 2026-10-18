<script setup lang="ts">
// The landing, written as a proof: the plan is the premise, the font is the
// construction, the checksum is the proof, and the equations are the corollary.
//
// Stubs, and what they want. Grep for "Stub:" when one is finished.
//
//   Stub: spec.optical-sizes  the plan has no optical size key behind this row
//   Stub: spec.script-style   needs the script letters, and an ssty feature
//   Stub: variant.picker      in `VariantPicker.vue`
import { useLemmataFont } from "~/composables/font";

useHead({ title: "A typeface built from a plan" });

const font = useLemmataFont();

/// The reference plan, which is what `Plan::default()` carries. It is content on
/// this page the way the samples are: the premise the rest is built from.
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

  return `${metrics.drawn.toLocaleString()} glyphs · ${metrics.mapped} mapped · sha256 ${font.digest.slice(0, 12)}`;
});
</script>

<template>
  <section class="hero">
    <div class="wrap">
      <p class="kicker">A premise, a construction, a proof</p>
      <HeroWord />
      <p class="lede">
        A lemma is a premise you build the rest on. Take the plan as given and
        this page builds the font: constructed here, measured here, and hashed so
        that it can be checked.
      </p>
      <div class="actions">
        <a class="btn" href="#construction">See the construction</a>
        <NuxtLink class="btn btn--primary" to="/generate/">Open the generator</NuxtLink>
      </div>
      <StatusLine :state="font.ready ? 'ok' : 'note'">{{ status }}</StatusLine>
      <SpecimenPlate :samples="SAMPLES" face="Lemmata Regular" :size="40" />
    </div>
  </section>

  <SectionBlock number="01" label="Premises">
    <h2>Everything is built from the plan</h2>
    <p>
      The plan is the premise. Every key defaults, so this is the reference plan,
      and every key it carries reaches the font.
    </p>
    <div class="cols">
      <PlanCard :plan="PLAN" />
      <div class="panel">
        <div class="panel__head"><span>What the plan carries</span></div>
        <div class="panel__body"><FeatureList :rows="FEATURES" /></div>
      </div>
    </div>
  </SectionBlock>

  <SectionBlock id="construction" number="02" label="Construction">
    <h2>The font, set in itself</h2>
    <p>
      This is the font this site ships, and every reading further down comes out
      of its own tables. Edit the sample and it stays the font.
    </p>
    <SpecimenEditor />

    <SubHead title="Texture healing" />
    <HealingPlate />

    <SubHead title="Spellings of a">
      <p class="note">Pick one and the next build draws it.</p>
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

  <SectionBlock id="proof" number="03" label="Proof">
    <h2>The build proves itself</h2>
    <p>
      A font is hard to argue with and easy to check. The build hashes itself,
      and the number it produces is the number the reference build recorded, so
      the plan and the font meet at one digest. The readings beside it come from
      the tables, not from the page's memory of what it meant to do.
    </p>
    <ProofPanel />
    <p class="qed">Q.E.D.</p>
  </SectionBlock>

  <SectionBlock id="corollary" number="04" label="Corollary">
    <h2>Equations follow</h2>
    <p>
      A MATH table comes out of the same construction: the constants, the italic
      corrections, and the delimiters that grow to fit. This one is set in a
      second face, built from the math glyph set when the section comes into
      view.
    </p>
    <EquationPlate />
  </SectionBlock>
</template>
