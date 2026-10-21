<script setup lang="ts">
// The top bar: name, routes, palette and the key map.
//
// The strip under it is the status a terminal would put on its title line:
// what the face is and whether it has arrived.
import { THEMES, useTheme } from "~/composables/theme";
import { helpOpen } from "~/composables/hotkeys";
import { useLemmataFont } from "~/composables/font";

const { theme, setTheme } = useTheme();
const font = useLemmataFont();

const live = computed(() => {
  if (font.error) return "font failed";
  if (!font.ready) return "loading font";

  return "font ready";
});
</script>

<template>
  <header class="site-head">
    <div class="site-head__bar">
      <NuxtLink class="brand" to="/" aria-label="Lemmata home">
        <LogoMark :size="24" />
        <span class="brand__word">lemmata</span>
        <span class="brand__tag">type factory</span>
      </NuxtLink>

      <nav class="site-nav" aria-label="Main">
        <NuxtLink class="site-nav__link" to="/">
          <span>01</span> specimen
        </NuxtLink>
        <NuxtLink class="site-nav__link" to="/generate/">
          <span>02</span> configurator
        </NuxtLink>
        <NuxtLink class="site-nav__link" to="/math/">
          <span>03</span> equations
        </NuxtLink>
      </nav>

      <div class="site-head__tools">
        <div class="theme-switch" role="group" aria-label="Palette">
          <button
            v-for="option in THEMES"
            :key="option.id"
            type="button"
            :title="option.label"
            :aria-label="`${option.label} palette`"
            :aria-pressed="theme === option.id"
            @click="setTheme(option.id)"
          >
            {{ option.short }}
          </button>
        </div>
        <button
          class="key-button"
          type="button"
          :aria-expanded="helpOpen"
          @click="helpOpen = !helpOpen"
        >
          <kbd>?</kbd><span>keys</span>
        </button>
      </div>
    </div>

    <div class="site-head__strip" aria-hidden="true">
      <div class="wrap">
        <span>type system for code and math</span>
        <span>mono / upright / 400</span>
        <span class="site-head__live"><i /> {{ live }}</span>
      </div>
    </div>
  </header>
</template>
