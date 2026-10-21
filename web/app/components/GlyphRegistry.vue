<script setup lang="ts">
// The glyph registry: every drawing the port builds, grouped by letter.
//
// The letter grid is the navigation. Two small fonts drive the drawings: one
// maps each letter to its representative so the grid shows the current choice,
// and one maps the selected group's variants to private-use codepoints. Both
// come from the source cache, so a plan change recomputes only the contours that
// changed and a group change fills only that group.
import {
  registry_group_specimen,
  registry_index_specimen,
  registry_names,
} from "lemmata-web";
import { usePlan } from "~/composables/plan";
import { useLemmataFont } from "~/composables/font";

const planner = usePlan();
const plan = planner.plan;
const font = useLemmataFont();

const names = ref<string[]>([]);
const groups = ref<{ group: string; variants: string[] }[]>([]);
const group = ref("a");
const hovered = ref<string | null>(null);
const filter = ref("");
const search = ref<HTMLInputElement | null>(null);
const error = ref("");
const building = ref(false);
const indexFamily = ref("var(--built)");
const groupFamily = ref("var(--built)");

let indexFace: FontFace | null = null;
let groupFace: FontFace | null = null;
let indexToken = 0;
let groupToken = 0;
let metricsTimer: ReturnType<typeof setTimeout> | undefined;
let choiceTimer: ReturnType<typeof setTimeout> | undefined;

function primary(name: string): boolean {
  if (name.indexOf(".") !== 1) return false;
  const groupName = name.slice(0, 1);
  const suffix = name.slice(2);
  if (!/[A-Za-z]/.test(groupName)) return false;

  return suffix.length > 0 && /^[A-Za-z0-9]+$/.test(suffix);
}

function humanize(name: string): string {
  const suffix = name.includes(".") ? name.slice(name.indexOf(".") + 1) : name;

  return suffix
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/([A-Z]+)([A-Z][a-z])/g, "$1 $2")
    .replace(/-/g, " ")
    .toLowerCase();
}

function toCamel(name: string): string {
  return name
    .split("-")
    .map((part, index) =>
      index === 0 ? part : `${part.charAt(0).toUpperCase()}${part.slice(1)}`,
    )
    .join("");
}

function toKebab(name: string): string {
  return name.replace(/[A-Z]/g, (character) => `-${character.toLowerCase()}`);
}

function suffixOf(name: string): string {
  return name.includes(".") ? name.slice(name.indexOf(".") + 1) : name;
}

function variantsOf(which: string): string[] {
  return groups.value.find((entry) => entry.group === which)?.variants ?? [];
}

const variants = computed(() => variantsOf(group.value));

const drawingCount = computed(() =>
  groups.value.reduce((total, entry) => total + entry.variants.length, 0),
);

const chosenName = computed(() => {
  const pick = planner.chosen(group.value);

  return pick ? `${group.value}.${toCamel(pick)}` : "";
});

const canApply = computed(() => {
  const code = group.value.codePointAt(0);
  if (code === undefined) return false;
  if (font.map) return font.map.has(code);

  return ["a", "b", "d", "p"].includes(group.value);
});

const previewName = computed(
  () => hovered.value ?? (chosenName.value || variants.value[0] || ""),
);

const previewLabel = computed(() =>
  previewName.value ? humanize(previewName.value) : "no drawing",
);

const selectedChar = computed(() => {
  const name = previewName.value;
  const index = name ? variants.value.indexOf(name) : -1;

  return index >= 0 ? String.fromCodePoint(0xe000 + index) : group.value;
});

const shown = computed(() => {
  const tokens = filter.value.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (!tokens.length) return variants.value;

  return variants.value.filter((name) => {
    const label = humanize(name);
    const slug = toKebab(suffixOf(name));

    return tokens.every(
      (token) => label.includes(token) || slug.includes(token),
    );
  });
});

function glyphFor(name: string): string {
  const index = variants.value.indexOf(name);

  return index >= 0 ? String.fromCodePoint(0xe000 + index) : group.value;
}

function selectGroup(which: string): void {
  group.value = which;
  hovered.value = null;
  filter.value = "";
  void refreshGroup(which);
}

function choose(name: string): void {
  hovered.value = name;
  if (!canApply.value) return;
  planner.choose(group.value, toKebab(suffixOf(name)));
}

function focusSearch(event: KeyboardEvent): void {
  const target = event.target;
  if (
    target instanceof HTMLElement &&
    (target.isContentEditable ||
      ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName))
  ) {
    return;
  }
  if (event.key !== "/") return;
  event.preventDefault();
  search.value?.focus();
}

function parsedMetrics(): string {
  try {
    const value = JSON.parse(plan.value) as Record<string, unknown>;

    return [
      value.weight,
      value.width,
      value.spacing,
      value.italicAngle,
    ].join("|");
  } catch {
    return "";
  }
}

function parsedChoices(): string {
  try {
    const value = JSON.parse(plan.value) as Record<string, unknown>;

    return JSON.stringify(value.variants ?? {});
  } catch {
    return "";
  }
}

async function refreshIndex(): Promise<void> {
  const token = ++indexToken;
  const family = `Lemmata Registry Index ${token}`;
  building.value = true;
  try {
    const bytes = registry_index_specimen(plan.value);
    const next = new FontFace(family, bytes.buffer as ArrayBuffer);
    await next.load();
    if (token !== indexToken) return;
    document.fonts.add(next);
    if (indexFace) document.fonts.delete(indexFace);
    indexFace = next;
    indexFamily.value = `"${family}", var(--built)`;
    error.value = "";
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : String(reason);
  } finally {
    if (token === indexToken) building.value = false;
  }
}

async function refreshGroup(which: string): Promise<void> {
  const token = ++groupToken;
  const family = `Lemmata Registry Group ${token}`;
  building.value = true;
  try {
    const bytes = registry_group_specimen(plan.value, which);
    const next = new FontFace(family, bytes.buffer as ArrayBuffer);
    await next.load();
    if (token !== groupToken) return;
    document.fonts.add(next);
    if (groupFace) document.fonts.delete(groupFace);
    groupFace = next;
    groupFamily.value = `"${family}", var(--built)`;
    error.value = "";
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : String(reason);
  } finally {
    if (token === groupToken) building.value = false;
  }
}

function scheduleMetrics(): void {
  if (!parsedMetrics()) return;
  if (metricsTimer !== undefined) clearTimeout(metricsTimer);
  metricsTimer = setTimeout(() => {
    void refreshIndex();
    void refreshGroup(group.value);
  }, 160);
}

function scheduleChoice(): void {
  if (!parsedChoices()) return;
  if (choiceTimer !== undefined) clearTimeout(choiceTimer);
  choiceTimer = setTimeout(() => void refreshIndex(), 120);
}

onMounted(async () => {
  addEventListener("keydown", focusSearch);
  await planner.ensure();
  names.value = registry_names()
    .split("\n")
    .filter(Boolean)
    .sort();

  const filtered = names.value.filter(primary);
  const grouped = new Map<string, string[]>();
  for (const name of filtered) {
    const base = name.slice(0, 1);
    const list = grouped.get(base) ?? [];
    list.push(name);
    grouped.set(base, list);
  }

  groups.value = [...grouped.entries()]
    .map(([entry, variants]) => ({ group: entry, variants }))
    .sort((left, right) => {
      const lowerLeft = left.group === left.group.toLowerCase() ? 0 : 1;
      const lowerRight = right.group === right.group.toLowerCase() ? 0 : 1;

      return lowerLeft - lowerRight || left.group.localeCompare(right.group);
    });

  if (!groups.value.some((entry) => entry.group === group.value)) {
    group.value = groups.value[0]?.group ?? "a";
  }
  await Promise.all([refreshIndex(), refreshGroup(group.value)]);
});

watch(() => parsedMetrics(), scheduleMetrics);
watch(() => parsedChoices(), scheduleChoice);

onBeforeUnmount(() => {
  removeEventListener("keydown", focusSearch);
  if (metricsTimer !== undefined) clearTimeout(metricsTimer);
  if (choiceTimer !== undefined) clearTimeout(choiceTimer);
  if (indexFace) document.fonts.delete(indexFace);
  if (groupFace) document.fonts.delete(groupFace);
});
</script>

<template>
  <div class="registry">
    <div class="registry__letters-head">
      <span>letters</span>
      <span>{{ groups.length }}</span>
    </div>

    <div class="registry__letters" role="group" aria-label="Ported letter groups">
      <button
        v-for="entry in groups"
        :key="entry.group"
        class="registry__letter"
        type="button"
        :aria-pressed="group === entry.group"
        @click="selectGroup(entry.group)"
      >
        <span class="registry__letter-glyph" :style="{ fontFamily: indexFamily }">
          {{ entry.group }}
        </span>
        <span class="registry__letter-name">{{ entry.group }}</span>
        <span class="registry__letter-count">{{ entry.variants.length }}</span>
        <span
          v-if="planner.chosen(entry.group)"
          class="registry__letter-dot"
          aria-hidden="true"
        />
      </button>
      <span v-if="!groups.length" class="status-line">
        <i /> reading the registry…
      </span>
    </div>

    <div class="registry__main">
      <div class="registry__stage">
        <div class="registry__stage-art">
          <span class="registry__big" :style="{ fontFamily: groupFamily }">
            {{ selectedChar }}
          </span>
        </div>

        <div class="registry__stage-info">
          <span class="registry__stage-kicker">
            {{ group }} / {{ previewLabel }}
          </span>
          <span class="registry__stage-state" :data-ready="canApply">
            {{ canApply ? "computed drawing / can join the plan" : "computed drawing / preview only" }}
          </span>
          <span class="registry__stage-plan">
            {{ !canApply ? "plan: no base glyph" : chosenName ? `plan: ${humanize(chosenName)}` : "plan: no choice yet" }}
          </span>
          <div class="registry__stage-actions">
            <span class="mono-note">
              <template v-if="error">the registry font failed: {{ error }}</template>
              <template v-else-if="building">updating the registry font…</template>
              <template v-else>
                {{ drawingCount }} drawings / {{ groups.length }} letters
              </template>
            </span>
          </div>
        </div>
      </div>

      <div class="registry__tools">
        <label class="registry__search">
          <span>/</span>
          <input
            ref="search"
            v-model="filter"
            type="search"
            placeholder="filter drawings"
            aria-label="Filter the registry"
          />
          <button v-if="filter" type="button" @click="filter = ''">clear</button>
        </label>
        <span class="registry__result">{{ shown.length }} / {{ variants.length }}</span>
      </div>

      <div class="registry__cards-scroll">
        <div class="registry__cards">
          <button
            v-for="name in shown"
            :key="name"
            class="registry__card"
            type="button"
            :aria-pressed="chosenName === name"
            @mouseenter="hovered = name"
            @mouseleave="hovered = null"
            @focus="hovered = name"
            @blur="hovered = null"
            @click="choose(name)"
          >
            <span class="registry__card-glyph" :style="{ fontFamily: groupFamily }">
              {{ glyphFor(name) }}
            </span>
            <span class="registry__card-name">{{ humanize(name) }}</span>
            <span class="registry__card-mark">{{ chosenName === name ? "●" : "" }}</span>
          </button>

          <p v-if="!shown.length" class="registry__empty">
            no drawings match “{{ filter }}”
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
