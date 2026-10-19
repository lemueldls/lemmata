<script setup lang="ts">
// The footer: the index, the licence and the keys that work from anywhere.
import { useLemmataFont } from "~/composables/font";

const font = useLemmataFont();

const readings = computed(() => [
  ["glyphs", font.metrics?.drawn ?? "—"],
  ["characters", font.metrics?.mapped ?? "—"],
  ["units / em", font.metrics?.upem ?? "—"],
  ["advance", font.metrics?.advance ?? "—"],
]);
</script>

<template>
  <footer class="site-foot">
    <div class="wrap">
      <div class="site-foot__top">
        <div>
          <p class="site-foot__word">lemmata</p>
          <p class="site-foot__note">
            A typeface factory for the browser. One plan goes in, a font comes out,
            and the page never needs an account.
          </p>
        </div>

        <div>
          <h3>index</h3>
          <ul>
            <li><NuxtLink to="/">Specimen</NuxtLink></li>
            <li><NuxtLink to="/generate/">Configurator</NuxtLink></li>
            <li><NuxtLink to="/math/">Equations</NuxtLink></li>
            <li><a href="/#samples">Samples</a></li>
          </ul>
        </div>

        <div>
          <h3>face</h3>
          <ul>
            <li v-for="[name, value] in readings" :key="name">
              {{ name }} <span class="muted">/ {{ value }}</span>
            </li>
          </ul>
        </div>

        <div>
          <h3>licence</h3>
          <ul>
            <li>Glyph source: Iosevka 34.9.0, OFL</li>
            <li>Everything else: MIT OR Apache-2.0</li>
            <li>No upload. No account.</li>
          </ul>
        </div>
      </div>

      <div class="site-foot__bottom">
        <span><i>+</i> Lemmata 2026</span>
        <span>Built from the font, not a screenshot</span>
        <span>Every sample set in the face it describes</span>
      </div>
    </div>
  </footer>
</template>
