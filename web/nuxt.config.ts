export default defineNuxtConfig({
  runtimeConfig: {
    public: {
      // Set NUXT_PUBLIC_CJK_BASE to the R2 mirror root at deploy time.
      cjkBase: process.env.NUXT_PUBLIC_CJK_BASE ?? "",
    },
  },
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
      {
        name: "Lemmata CJK Sample",
        src: "/fonts/lemmata-cjk-sample.woff2",
        global: false,
        preload: false,
      },
    ],
  },
  // The API runs in the worker. The pages can render there too, which keeps the
  // build and the deploy on the same code path. `wrangler.jsonc` holds the entry
  // and the assets binding, so the build leaves one config rather than two.
  nitro: {
    preset: process.env.NITRO_PRESET,
    cloudflare: { deployConfig: false },
  },
  // The configurator needs a document and a worker of its own. Rendering it on
  // the edge only to hydrate the same page adds a failure the page cannot use.
  routeRules: {
    "/generate": { ssr: false },
  },
  // `golden/parity.json` and `corpus/equations.mathml` sit outside `web/`, and
  // the dev tool only reads from inside the project root without this.
  vite: { server: { fs: { allow: [".."] } } },
});
