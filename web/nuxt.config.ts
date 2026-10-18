export default defineNuxtConfig({
  compatibilityDate: "2026-10-18",
  modules: ["@nuxt/fonts"],
  css: ["~/assets/css/site.css"],
  fonts: {
    families: [
      {
        name: "Lemmata",
        src: "/fonts/lemmata.ttf",
        global: true,
        preload: true,
      },
      {
        name: "Lemmata Math",
        src: "/fonts/lemmata-math.woff2",
        global: true,
        preload: false,
      },
    ],
  },
  // `golden/parity.json` and `corpus/equations.mathml` sit outside `web/`, and
  // the dev server only serves from inside the project root without this.
  vite: { server: { fs: { allow: [".."] } } },
});
