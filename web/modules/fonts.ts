// Builds the fonts the site is set in, as part of the site's own build.
//
// `scripts/font.mjs` does the work and this only has to run it at the right
// moment: before the build reads the files, and before the dev server starts
// serving them. It checks the reference font against `golden/parity.json`, so
// running it here is what makes a stale or missing font stop the build rather
// than reach a reader.
//
// The call is synchronous and the fonts are committed, so the common case is a
// hash of seventeen kilobytes.
import { execFileSync } from "node:child_process";
import { defineNuxtModule } from "@nuxt/kit";

export default defineNuxtModule({
  meta: { name: "lemmata-fonts" },
  setup(_options, nuxt) {
    // `nuxt generate` fires both hooks, and the check is the same either way.
    let checked = false;
    const ensure = (): void => {
      if (checked) return;
      checked = true;
      execFileSync("node", ["scripts/font.mjs"], {
        cwd: nuxt.options.rootDir,
        stdio: ["ignore", "inherit", "inherit"],
      });
    };

    nuxt.hook("ready", ensure);
    nuxt.hook("build:before", ensure);
  },
});
