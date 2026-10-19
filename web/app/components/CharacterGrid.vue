<script setup lang="ts">
// Every character the font maps, drawn from the map the font itself provides.
//
// The grid is the point of a mono face: one cell, one advance, and the drawing
// moving inside it. Hovering a cell enlarges the glyph. Clicking copies the
// character and pins the readout.
import { useLemmataFont } from "~/composables/font";
import { specimenSize } from "~/composables/hotkeys";
import { isPublic } from "~/utils/font";

const font = useLemmataFont();
const query = ref("");
const active = ref<number | null>(null);
const copied = ref<number | null>(null);
const expanded = ref(false);

const hex = (code: number): string =>
  `U+${code.toString(16).toUpperCase().padStart(4, "0")}`;

const chars = computed(() =>
  font.map
    ? [...font.map.keys()]
        .filter(isPublic)
        .sort((left, right) => left - right)
    : [],
);

const shown = computed(() => {
  const needle = query.value.trim().toLowerCase();
  if (!needle) return chars.value;

  return chars.value.filter((code) => {
    const character = String.fromCodePoint(code).toLowerCase();

    return character.includes(needle) || hex(code).toLowerCase().includes(needle);
  });
});

const current = computed(
  () => shown.value.find((code) => code === active.value) ?? shown.value[0] ?? null,
);

const LIMIT = 48;

const visible = computed(() =>
  expanded.value || query.value.trim()
    ? shown.value
    : shown.value.slice(0, LIMIT),
);

const detail = computed(() => {
  const code = current.value;
  if (code === null) return "no match";

  return `${hex(code)} / ${code === 0x20 ? "space" : String.fromCodePoint(code)}`;
});

async function pick(code: number): Promise<void> {
  active.value = code;
  try {
    await navigator.clipboard.writeText(String.fromCodePoint(code));
    copied.value = code;
    setTimeout(() => {
      if (copied.value === code) copied.value = null;
    }, 1200);
  } catch {
    copied.value = null;
  }
}
</script>

<template>
  <div class="char-grid">
    <div class="char-grid__tools">
      <label class="search">
        <span>filter</span>
        <input v-model="query" type="search" placeholder="U+0061 or a" />
      </label>
      <span class="char-grid__count">{{ shown.length }} / {{ chars.length }} cells</span>
      <span class="char-grid__detail">
        {{ copied === current ? "copied" : detail }}
      </span>
    </div>

    <div
      class="char-grid__cells"
      :style="{ '--glyph-size': `${Math.min(42, Math.max(20, specimenSize + 6))}px` }"
    >
      <button
        v-for="code in visible"
        :key="code"
        class="cell"
        :class="{ 'cell--space': code === 0x20 }"
        type="button"
        :aria-pressed="current === code"
        :aria-label="`${hex(code)} ${code === 0x20 ? 'space' : String.fromCodePoint(code)}`"
        @focus="active = code"
        @mouseenter="active = code"
        @click="pick(code)"
      >
        <span class="cell__glyph">{{ code === 0x20 ? "·" : String.fromCodePoint(code) }}</span>
        <span class="cell__code">{{ hex(code) }}</span>
      </button>
    </div>

    <div v-if="shown.length > LIMIT && !query" class="form-row">
      <span class="mono-note">
        {{ expanded ? `all ${shown.length} cells` : `showing ${LIMIT} of ${shown.length} cells` }}
      </span>
      <button class="btn btn--sm" type="button" :aria-expanded="expanded" @click="expanded = !expanded">
        {{ expanded ? "show fewer" : "show all" }}
      </button>
    </div>
  </div>
</template>
