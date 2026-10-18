import { defineConfig } from "astro/config";

// Astro's default layout: source in `web/`, build in `web/dist`, the wasm in
// `web/pkg` as a workspace package.
//
// Static output is the product rather than a preference: the generator runs in
// the browser, so nothing renders at request time (r[site.static]).
export default defineConfig({
  compressHTML: true,
  // `golden/parity.json` and `corpus/equations.mathml` sit outside `web/`, and
  // the dev server only serves from inside the project root without this.
  vite: { server: { fs: { allow: [".."] } } },
});
