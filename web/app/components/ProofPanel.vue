<script setup lang="ts">
// The build proves itself: the font the page is set in, hashed here, against the
// number the recorded pair carries, with the readings beside it taken from the
// font's own tables.
import golden from "../../../golden/parity.json";
import { short } from "~/utils/font";
import { useLemmataFont } from "~/composables/font";

const font = useLemmataFont();

const matches = computed(
  () => font.ready && font.digest === golden.font,
);

const readings = computed(() => {
  const metrics = font.metrics;
  if (!metrics) return [];

  return [
    ["units per em", metrics.upem],
    ["glyphs drawn", metrics.drawn],
    ["characters mapped", metrics.mapped],
    ["x-height, x", metrics.xHeight],
    ["ascender, l", metrics.ascender],
    ["descender, p", metrics.descender],
    ["advance, x", metrics.advance],
  ];
});
</script>

<template>
  <div class="cols">
    <div class="panel">
      <div class="panel__head">
        <span>Checksums</span>
        <span class="chip" :data-state="font.ready ? (matches ? 'ok' : 'bad') : 'pending'">
          {{ font.ready ? (matches ? "byte-identical" : "differs") : "hashing" }}
        </span>
      </div>
      <div class="panel__body">
        <dl class="rows">
          <div class="rows__row">
            <dt>capture</dt>
            <dd :title="golden.capture">{{ short(golden.capture) }}</dd>
          </div>
          <div class="rows__row">
            <dt>shipped here</dt>
            <dd :title="font.digest">{{ font.ready ? short(font.digest) : "…" }}</dd>
          </div>
          <div class="rows__row">
            <dt>reference build</dt>
            <dd :title="golden.font">{{ short(golden.font) }}</dd>
          </div>
        </dl>
        <p class="note">
          {{
            font.error
              ? `The font could not be read: ${font.error}`
              : font.ready
                ? matches
                  ? "the font this page is set in matches the reference build"
                  : "the font this page is set in differs from the reference build"
                : "Waiting for the font."
          }}
        </p>
        <p class="note">
          rustc {{ golden.toolchain.rustc }} · wasm-bindgen {{ golden.toolchain["wasm-bindgen"] }} ·
          upstream {{ golden.upstream.slice(0, 7) }}
        </p>
      </div>
    </div>

    <div class="panel">
      <div class="panel__head"><span>Read from the font</span></div>
      <div class="panel__body">
        <dl class="rows">
          <div v-for="[name, value] in readings" :key="name" class="rows__row">
            <dt>{{ name }}</dt>
            <dd>{{ value ?? "not in the font" }}</dd>
          </div>
        </dl>
        <p class="note">
          In font units, out of the tables of the font the site ships. The ink is
          the bounding box the outline carries.
        </p>
      </div>
    </div>
  </div>
</template>
