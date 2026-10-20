<script setup lang="ts">
// The glyph registry: every drawing the port builds, grouped by letter.
//
// One small font holds all the drawings as private-use codepoints, built once
// from the current plan. The letter rail, the stage and the cards all read that
// one font, so moving between groups costs a click rather than a build.
//
// A card applies its drawing straight away. That keeps the choice one tap on a
// phone, where a separate button below the fold would mean scrolling back. The
// search and the scroll area keep the browser usable as the registry grows.
import {
  ScrollAreaRoot,
  ScrollAreaScrollbar,
  ScrollAreaThumb,
  ScrollAreaViewport,
  SelectContent,
  SelectIcon,
  SelectItem,
  SelectItemIndicator,
  SelectItemText,
  SelectPortal,
  SelectRoot,
  SelectTrigger,
  SelectValue,
  SelectViewport,
} from "reka-ui";
import { registry_names, registry_specimen } from "lemmata-web";
import { usePlan } from "~/composables/plan";
import { useLemmataFont } from "~/composables/font";

const planner = usePlan();
const plan = planner.plan;
const font = useLemmataFont();

const names = ref<string[]>([]);
const groups = ref<{ group: string; variants: string[] }[]>([]);
const pua = ref(new Map<string, number>());
const group = ref("a");
const hovered = ref<string | null>(null);
const filter = ref("");
const search = ref<HTMLInputElement | null>(null);
const ready = ref(false);
const error = ref("");
const building = ref(false);

const registry = ref("var(--built)");
let face: FontFace | null = null;

function primary(name: string): boolean {
  if (name.indexOf(".") !== 1) return false;
  const group = name.slice(0, 1);
  const suffix = name.slice(2);
  if (!/[A-Za-z]/.test(group)) return false;

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
  () =>
    hovered.value ??
    (chosenName.value || (canApply.value ? "" : variants.value[0] || "")),
);

const previewLabel = computed(() =>
  previewName.value ? humanize(previewName.value) : "outline's own drawing",
);

const selectedChar = computed(() => {
  const name = previewName.value;
  const code = name ? pua.value.get(name) : undefined;

  return code ? String.fromCodePoint(code) : group.value;
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
  const code = pua.value.get(name);

  return code
    ? String.fromCodePoint(code)
    : name.includes(".")
      ? name.slice(0, 1)
      : group.value;
}

function groupGlyph(which: string): string {
  const pick = planner.chosen(which);
  const camel = pick ? `${which}.${toCamel(pick)}` : "";
  const chosen = variantsOf(which).includes(camel) ? camel : "";
  if (chosen) return glyphFor(chosen);

  const code = which.codePointAt(0);
  if (code !== undefined && font.map?.has(code)) return which;

  const first = variantsOf(which)[0];
  if (first) return glyphFor(first);

  return which;
}

function selectGroup(which: string): void {
  group.value = which;
  hovered.value = null;
  filter.value = "";
}

function choose(name: string): void {
  hovered.value = name;
  if (!canApply.value) return;
  planner.choose(group.value, toKebab(suffixOf(name)));
}

function clearChoice(): void {
  if (!canApply.value) return;
  planner.choose(group.value, "");
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

    return [value.weight, value.spacing, value.italicAngle].join("|");
  } catch {
    return "";
  }
}

async function buildRegistryFont(): Promise<void> {
  if (!ready.value || building.value) return;
  building.value = true;
  error.value = "";
  try {
    const bytes = registry_specimen(plan.value);
    if (face) document.fonts.delete(face);
    face = new FontFace("Lemmata Registry", bytes.buffer as ArrayBuffer);
    await face.load();
    document.fonts.add(face);
    registry.value = '"Lemmata Registry", var(--built)';
  } catch (reason) {
    error.value = reason instanceof Error ? reason.message : String(reason);
    registry.value = "var(--built)";
  } finally {
    building.value = false;
  }
}

let timer: ReturnType<typeof setTimeout> | undefined;

function schedule(): void {
  if (timer !== undefined) clearTimeout(timer);
  timer = setTimeout(() => void buildRegistryFont(), 240);
}

onMounted(async () => {
  addEventListener("keydown", focusSearch);
  await planner.ensure();
  ready.value = true;

  names.value = registry_names()
    .split("\n")
    .filter(Boolean)
    .sort();

  const filtered = names.value.filter(primary);
  pua.value = new Map(filtered.map((name, index) => [name, 0xe000 + index]));

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
  selectGroup(group.value);
  await buildRegistryFont();
});

watch(() => parsedMetrics(), schedule);

onBeforeUnmount(() => {
  removeEventListener("keydown", focusSearch);
  if (timer !== undefined) clearTimeout(timer);
  if (face) document.fonts.delete(face);
});
</script>

<template>
  <div class="registry">
    <div class="registry__mobilebar">
      <SelectRoot
        :model-value="group"
        @update:model-value="(value) => selectGroup(String(value))"
      >
        <SelectTrigger class="registry__select-trigger" aria-label="Letter group">
          <span class="registry__select-glyph" :style="{ fontFamily: registry }">
            {{ groupGlyph(group) }}
          </span>
          <SelectValue class="registry__select-value" />
          <SelectIcon class="registry__select-icon">▾</SelectIcon>
        </SelectTrigger>

        <SelectPortal>
          <SelectContent class="registry__select-content" position="popper" :side-offset="4">
            <SelectViewport>
              <SelectItem
                v-for="entry in groups"
                :key="entry.group"
                :value="entry.group"
                class="registry__select-item"
              >
                <SelectItemText>
                  {{ entry.group }} / {{ entry.variants.length }} drawings
                </SelectItemText>
                <SelectItemIndicator class="registry__select-indicator">●</SelectItemIndicator>
              </SelectItem>
            </SelectViewport>
          </SelectContent>
        </SelectPortal>
      </SelectRoot>

      <span class="registry__mobile-count">{{ pua.size }} drawings</span>
    </div>

    <div class="registry__layout">
      <aside class="registry__rail">
        <div class="registry__rail-head">
          <span>letters</span>
          <span>{{ groups.length }}</span>
        </div>

        <ScrollAreaRoot class="registry__rail-scroll">
          <ScrollAreaViewport class="registry__rail-viewport">
            <div class="registry__groups" role="group" aria-label="Ported letter groups">
              <button
                v-for="entry in groups"
                :key="entry.group"
                class="registry__group"
                type="button"
                :aria-pressed="group === entry.group"
                @click="selectGroup(entry.group)"
              >
                <span class="registry__group-glyph" :style="{ fontFamily: registry }">
                  {{ groupGlyph(entry.group) }}
                </span>
                <span class="registry__group-name">{{ entry.group }}</span>
                <span class="registry__group-count">{{ entry.variants.length }}</span>
                <span
                  v-if="planner.chosen(entry.group)"
                  class="registry__group-dot"
                  aria-hidden="true"
                />
              </button>
              <span v-if="!groups.length" class="status-line">
                <i /> reading the registry…
              </span>
            </div>
          </ScrollAreaViewport>
          <ScrollAreaScrollbar class="registry__scrollbar" orientation="vertical">
            <ScrollAreaThumb class="registry__thumb" />
          </ScrollAreaScrollbar>
        </ScrollAreaRoot>
      </aside>

      <div class="registry__main">
        <div class="registry__stage">
          <div class="registry__stage-art">
            <span class="registry__big" :style="{ fontFamily: registry }">
              {{ selectedChar }}
            </span>
          </div>

          <div class="registry__stage-info">
            <span class="registry__stage-kicker">
              {{ group }} / {{ previewLabel }}
            </span>
            <span class="registry__stage-state" :data-ready="canApply">
              {{ canApply ? "maps to the outline set" : "preview only, no base glyph" }}
            </span>
            <span class="registry__stage-plan">
              {{ !canApply ? "plan: no base glyph" : chosenName ? `plan: ${humanize(chosenName)}` : "plan: outline drawing" }}
            </span>
            <div class="registry__stage-actions">
              <button
                v-if="chosenName"
                class="btn btn--sm"
                type="button"
                :disabled="!canApply"
                @click="clearChoice"
              >
                clear choice
              </button>
              <span class="mono-note">
                <template v-if="error">the registry font failed: {{ error }}</template>
                <template v-else-if="building">building the registry font…</template>
                <template v-else>{{ pua.size }} drawings / {{ groups.length }} letters</template>
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

        <ScrollAreaRoot class="registry__cards-scroll">
          <ScrollAreaViewport class="registry__cards-viewport">
            <div class="registry__cards">
              <button
                v-if="canApply"
                class="registry__card registry__card--default"
                type="button"
                :aria-pressed="chosenName === ''"
                @click="clearChoice"
              >
                <span class="registry__card-glyph" :style="{ fontFamily: 'var(--built)' }">
                  {{ group }}
                </span>
                <span class="registry__card-name">outline's own drawing</span>
                <span class="registry__card-mark">{{ chosenName === "" ? "●" : "" }}</span>
              </button>

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
                <span class="registry__card-glyph" :style="{ fontFamily: registry }">
                  {{ glyphFor(name) }}
                </span>
                <span class="registry__card-name">{{ humanize(name) }}</span>
                <span class="registry__card-mark">{{ chosenName === name ? "●" : "" }}</span>
              </button>

              <p v-if="!shown.length" class="registry__empty">
                no drawings match “{{ filter }}”
              </p>
            </div>
          </ScrollAreaViewport>
          <ScrollAreaScrollbar class="registry__scrollbar" orientation="vertical">
            <ScrollAreaThumb class="registry__thumb" />
          </ScrollAreaScrollbar>
        </ScrollAreaRoot>
      </div>
    </div>
  </div>
</template>
