import { defineConfig } from "astro/config";

// The build lands in `www/dist`, beside the prototype pages that `www/` already
// holds, so `/www/dist/...` is the site and `/www/index.html` and
// `/www/math.html` keep working while the prototype is retired.
//
// Static output is the product rather than a preference: the generator runs in
// the browser, so nothing renders at request time (r[site.static]).
export default defineConfig({
  outDir: "../www/dist",
  compressHTML: true,
});
