// The site's shell and its fonts.
//
// Static output is the product rather than a preference: the generator runs in
// the browser, so nothing renders at request time, and `nuxt generate` prerenders
// every route into files (r[site.static]).
//
// The two faces the site is set in are built by `pnpm font` and committed under
// `public/fonts/`, which is where the module looks for a `src`. It names them
// here, writes the `@font-face` rules and an optimized fallback with the metrics
// adjusted to each, and preloads the one the landing shows above the fold. The
// math face is not preloaded: one page reaches for it, and only when it does.
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
