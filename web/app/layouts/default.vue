<script setup lang="ts">
// The shell: the head, the masthead, the footer and the shortcuts.
//
// The two faces are declared in `nuxt.config.ts`, so what is left here is the
// theme and the keyboard. The theme is set by an inline script before the
// stylesheet applies, because a swap after paint shows the wrong one for a
// frame, which is the one thing a toggle cannot fix afterwards.
import { useTheme } from "~/composables/theme";

const { theme, toggle } = useTheme();

useHotkeys();

useHead({
  titleTemplate: (title) => (title ? `${title} — Lemmata` : "Lemmata"),
  meta: [
    {
      name: "description",
      content: "A typeface built from a plan, and proved in the page that shows it.",
    },
  ],
  script: [
    {
      tagPriority: "critical",
      innerHTML:
        'const stored = localStorage.getItem("lemmata:theme");' +
        'const system = matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";' +
        'document.documentElement.dataset.theme = stored ?? system;',
    },
  ],
});
</script>

<template>
  <header class="masthead">
    <div class="wrap masthead__inner">
      <NuxtLink class="brand" to="/">Lemmata</NuxtLink>
      <nav>
        <NuxtLink to="/generate/">Generator</NuxtLink>
        <NuxtLink to="/math/">Equations</NuxtLink>
      </nav>
      <div class="masthead__right">
        <span class="tag" title="The Iosevka release the glyph definitions come from">
          Iosevka 34.9.0
        </span>
        <button
          class="icon-button"
          id="theme"
          type="button"
          title="Switch the theme (t)"
          :aria-label="`Switch to the ${theme === 'light' ? 'dark' : 'light'} theme`"
          @click="toggle"
        >
          <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
            <template v-if="theme === 'light'">
              <circle cx="8" cy="8" r="3.4" fill="currentColor" />
              <g stroke="currentColor" stroke-width="1.1">
                <path d="M8 0.6v2.2M8 13.2v2.2M0.6 8h2.2M13.2 8h2.2" />
                <path d="M2.8 2.8l1.6 1.6M11.6 11.6l1.6 1.6M13.2 2.8l-1.6 1.6M4.4 11.6l-1.6 1.6" />
              </g>
            </template>
            <path
              v-else
              d="M13 9.6A5.6 5.6 0 1 1 6.4 3a4.6 4.6 0 0 0 6.6 6.6Z"
              fill="currentColor"
            />
          </svg>
        </button>
      </div>
    </div>
  </header>

  <main>
    <slot />
  </main>

  <footer class="footer">
    <div class="wrap footer__grid">
      <div>
        <h3>Lemmata</h3>
        <p>A typeface built from a plan, and proved in the page that shows it.</p>
      </div>
      <div>
        <h3>Routes</h3>
        <ul>
          <li><NuxtLink to="/">Premises</NuxtLink></li>
          <li><NuxtLink to="/generate/">Generator</NuxtLink></li>
          <li><NuxtLink to="/math/">Equations</NuxtLink></li>
        </ul>
      </div>
      <div>
        <h3>Keys</h3>
        <ul>
          <li><kbd>g</kbd> generator</li>
          <li><kbd>e</kbd> equations</li>
          <li><kbd>p</kbd> present</li>
          <li><kbd>t</kbd> theme</li>
        </ul>
      </div>
      <div>
        <h3>Provenance</h3>
        <ul>
          <li>Glyph definitions: Iosevka 34.9.0, OFL</li>
          <li>Everything else: MIT OR Apache-2.0</li>
          <li>Nothing is uploaded. There is no server to upload to.</li>
        </ul>
      </div>
    </div>
  </footer>
</template>
