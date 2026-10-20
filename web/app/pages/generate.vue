<script setup lang="ts">
// The configurator: basics, variants, ligations, a playground and the plan as
// text, with the build rail held beside them.
//
// One plan lives on this page. A field, the variant grid and the text editor all
// write the same object, so a build cannot read a different plan than the one a
// reader sees.
import {
  advance,
  cmap,
  glyphCount,
  ink,
  isPublic,
  unitsPerEm,
  verticalMetrics,
  type Reading,
} from "~/utils/font";
import { usePlan } from "~/composables/plan";
import { useBuild } from "~/composables/build";

useHead({ title: "Configurator" });

const { plan, capture, ensure, spacing, setSpacing, reset } = usePlan();
const {
  status,
  state,
  download,
  downloadName,
  bytes: built,
  elapsed,
  busy,
  run,
} = useBuild();

const preview = ref("var(--built)");
const mounted = ref(false);

const BASICS = [
  { key: "capHeight", label: "cap height", unit: "units", step: 1 },
  { key: "xHeight", label: "x height", unit: "units", step: 1 },
  { key: "ascender", label: "ascender", unit: "units", step: 1 },
  { key: "descender", label: "descender", unit: "units", step: 1 },
  { key: "weight", label: "weight", unit: "class", step: 100 },
  { key: "italicAngle", label: "italic angle", unit: "deg", step: 1 },
];

const reading = computed<Reading | null>(() => {
  const font = built.value;
  if (!font) return null;
  const map = cmap(font);
  const vertical = verticalMetrics(font);

  return {
    upem: unitsPerEm(font),
    drawn: glyphCount(font) ?? 0,
    mapped: [...map.keys()].filter(isPublic).length,
    xHeight: ink(map, font, "x")?.yMax ?? null,
    ascender: vertical.ascender,
    descender: vertical.descender,
    advance: advance(map, font, "x"),
  };
});

/// The plan as a form. A plan that does not parse belongs to the person who
/// typed it, so the fields leave it alone rather than replacing the text.
function value(key: string): number {
  try {
    return Number(JSON.parse(plan.value)[key] ?? 0);
  } catch {
    return 0;
  }
}

function set(key: string, next: number): void {
  try {
    const object = JSON.parse(plan.value);
    object[key] = next;
    plan.value = JSON.stringify(object, null, 2);
  } catch {
    // Leave the text as it is.
  }
}

async function build(): Promise<void> {
  await ensure();
  if (capture.value) await run(plan.value, capture.value);
}

function downloadNow(): void {
  const href = download.value;
  if (!href) return;
  const link = document.createElement("a");
  link.href = href;
  link.download = downloadName.value;
  link.click();
}

function onAction(event: Event): void {
  const detail = (event as CustomEvent<string>).detail;
  if (detail === "build") void build();
  if (detail === "download") downloadNow();
}

let face: FontFace | null = null;

watch(built, async (font) => {
  if (!font) return;
  if (face) document.fonts.delete(face);
  face = new FontFace("Lemmata Preview", font.buffer as ArrayBuffer);
  await face.load();
  document.fonts.add(face);
  preview.value = '"Lemmata Preview", var(--built)';
});

onMounted(async () => {
  addEventListener("lemmata:action", onAction);
  await ensure();
  mounted.value = true;
});

onBeforeUnmount(() => {
  removeEventListener("lemmata:action", onAction);
  if (face) document.fonts.delete(face);
});
</script>

<template>
  <div class="wrap page">
    <header class="page-head">
      <div>
        <p class="eyebrow"><b>+</b> configurator <i>/</i> 02</p>
        <h1 class="page-head__title">Build <em>a face.</em></h1>
        <p class="lede" style="margin-top: 18px">
          The plan is the input. Seven keys set the frame, the variant grid picks
          a drawing, and the build rail writes the file. Change anything and build
          again.
        </p>
      </div>

      <div class="page-head__side">
        <p class="mono-note">
          the outline set in this page holds 72 glyphs. a build takes a moment,
          and the file is written in the tab.
        </p>
        <dl class="page-head__facts">
          <div><dt>plan</dt><dd>json / 7 keys</dd></div>
          <div><dt>outline set</dt><dd>72 glyphs</dd></div>
          <div><dt>spacing</dt><dd>{{ spacing }}</dd></div>
          <div><dt>output</dt><dd>ttf</dd></div>
        </dl>
      </div>
    </header>

    <div class="config-layout">
      <aside class="config-aside">
        <BuildPanel
          :status="status"
          :state="state"
          :bytes="built"
          :elapsed="elapsed"
          :busy="busy"
          :download="download"
          :download-name="downloadName"
          :metrics="reading"
          :preview="preview"
          @build="build"
        />
        <p class="mono-note">
          <kbd>b</kbd> builds / <kbd>d</kbd> downloads / <kbd>esc</kbd> leaves a
          field
        </p>
      </aside>

      <div class="config-main">
        <PlusPlate index="01" label="basics" meta="the frame">
          <div class="section-intro section-intro--single">
            <div>
              <p class="eyebrow"><b>+</b> seven keys</p>
              <h2 class="display" style="font-size: var(--step-3)">
                Set the frame.
              </h2>
            </div>
            <p class="prose">
              These keys become the font's vertical metrics and its weight class.
              A weight change moves the tables in the browser; the outline itself
              needs the native tool.
            </p>
          </div>

          <div class="fields">
            <label v-for="field in BASICS" :key="field.key" class="field">
              <span>{{ field.label }} <i>{{ field.unit }}</i></span>
              <input
                type="number"
                :step="field.step"
                :value="value(field.key)"
                @input="set(field.key, Number(($event.target as HTMLInputElement).value))"
              />
            </label>
          </div>

          <div class="form-row">
            <span>spacing</span>
            <div class="segmented" role="group" aria-label="Spacing">
              <button
                v-for="option in ['normal', 'quasi-proportional']"
                :key="option"
                type="button"
                :aria-pressed="spacing === option"
                @click="setSpacing(option)"
              >
                {{ option }}
              </button>
            </div>
          </div>
        </PlusPlate>

        <PlusPlate index="02" label="variants" meta="the drawing">
          <div class="section-intro section-intro--single">
            <div>
              <p class="eyebrow"><b>+</b> character choice</p>
              <h2 class="display" style="font-size: var(--step-3)">
                Pick a spelling.
              </h2>
            </div>
            <p class="prose">
              The picker lists what the outline set holds and writes the choice
              into the plan. The chosen drawing lands on the group's own glyph, so
              features that name letters keep working.
            </p>
          </div>

          <VariantPicker />
        </PlusPlate>

        <PlusPlate index="03" label="ligations" meta="sets / next">
          <div class="section-intro section-intro--single">
            <div>
              <p class="eyebrow"><b>+</b> sequences</p>
              <h2 class="display" style="font-size: var(--step-3)">
                Name the sets.
              </h2>
            </div>
            <p class="prose">
              A coding face is expected to name the arrows, comparisons, logic
              marks and comment marks. The plan names them here. The lookups are
              the next piece of the build.
            </p>
          </div>

          <LigatureList />
        </PlusPlate>

        <PlusPlate index="04" label="playground" meta="edit / resize / compare">
          <div class="section-intro section-intro--single">
            <div>
              <p class="eyebrow"><b>+</b> try it</p>
              <h2 class="display" style="font-size: var(--step-3)">
                Set a line, then set it again.
              </h2>
            </div>
            <p class="prose">
              The preview uses the site face until a build finishes, then it swaps
              to the file you just made. Edit the text, move the size, and switch
              the features.
            </p>
          </div>

          <PlaygroundPanel :preview="preview" />
        </PlusPlate>

        <PlusPlate index="05" label="plan" meta="json / live">
          <div class="section-intro section-intro--single">
            <div>
              <p class="eyebrow"><b>+</b> the object</p>
              <h2 class="display" style="font-size: var(--step-3)">
                Your config.
              </h2>
            </div>
            <p class="prose">
              The fields and the picker write this object. You can edit it here as
              well. A key the builder knows is read on the next build, and a key it
              does not know is left alone.
            </p>
          </div>

          <ConfigEditor v-model="plan" @reset="reset" />

          <div class="form-row">
            <span class="status-line" :data-state="mounted ? 'ok' : 'note'">
              <i /> {{ mounted ? "plan ready" : "loading the builder" }}
            </span>
            <button class="btn btn--accent" type="button" :disabled="busy" @click="build">
              build the plan <kbd>b</kbd>
            </button>
          </div>
        </PlusPlate>
      </div>
    </div>
  </div>
</template>
