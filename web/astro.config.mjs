import { defineConfig, fontProviders } from "astro/config";

// Astro's default layout: source in `web/`, build in `web/dist`, the wasm in
// `web/pkg` as a workspace package.
//
// Static output is the product rather than a preference: the generator runs in
// the browser, so nothing renders at request time (r[site.static]).
//
// The two faces the site is set in are built by `pnpm font` and committed under
// `web/fonts/`, which Astro serves from its own hashed path. The Fonts API here
// only has to say where they are, put the preload in the head, and write the
// `@font-face` rules. It renames the family it emits, which is why nothing
// should look for the font by the name in this file.
export default defineConfig({
  compressHTML: true,
  fonts: [
    {
      provider: fontProviders.local(),
      name: "Lemmata",
      cssVariable: "--font-lemmata",
      fallbacks: ["monospace"],
      options: {
        variants: [
          {
            src: ["./fonts/lemmata.ttf"],
            // The wordmark is measured to the width of the measure once the
            // font is here, so a fallback first would be a size change as well
            // as a swap, and the change is what a reader would notice.
            display: "block",
          },
        ],
      },
    },
    {
      provider: fontProviders.local(),
      name: "Lemmata Math",
      cssVariable: "--font-lemmata-math",
      fallbacks: ["serif"],
      options: {
        variants: [{ src: ["./fonts/lemmata-math.woff2"] }],
      },
    },
  ],
  // `golden/parity.json` and `corpus/equations.mathml` sit outside `web/`, and
  // the dev server only serves from inside the project root without this.
  vite: { server: { fs: { allow: [".."] } } },
});
