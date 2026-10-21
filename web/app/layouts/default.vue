<script setup lang="ts">
// The shell: head, masthead, footer, key map and status rail.
//
// The palette is set by an inline script before the stylesheet applies. A swap
// after paint would show the wrong ground for a frame, which is the one thing a
// toggle cannot repair afterwards.
useHotkeys();

useHead({
  titleTemplate: (title) => (title ? `${title} / Lemmata` : "Lemmata / type factory"),
  meta: [
    {
      name: "description",
      content:
        "A typeface factory in the browser. Set the metrics, choose the drawings, and build a font that never leaves the tab.",
    },
    { name: "theme-color", content: "#0b0d0a" },
  ],
  script: [
    {
      tagPriority: "critical",
      innerHTML:
        'const stored = localStorage.getItem("lemmata:theme:v2");' +
        'const allowed = ["night", "paper", "signal"];' +
        'const system = matchMedia("(prefers-color-scheme: light)").matches ? "paper" : "night";' +
        'document.documentElement.dataset.theme = allowed.includes(stored) ? stored : system;',
    },
  ],
  link: [{ rel: "icon", href: "/favicon.svg?v=2", type: "image/svg+xml" }],
});
</script>

<template>
  <div class="site">
    <a class="skip" href="#main">skip to content</a>
    <SiteHeader />
    <main id="main">
      <slot />
    </main>
    <SiteFooter />
    <StatusBar />
    <HotkeyOverlay />
  </div>
</template>
