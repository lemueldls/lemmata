// Builds the preset fonts the API returns before a request can ask for one.
//
// The assets are generated, so this is a no-op when they already exist. It runs
// before the build reads the assets directory, which keeps the API from starting
// with an empty list. A prepare step only writes types, so it is skipped.
import { execFileSync } from "node:child_process";
import { defineNuxtModule } from "@nuxt/kit";

export default defineNuxtModule({
  meta: { name: "lemmata-presets" },
  setup(_options, nuxt) {
    if ((nuxt.options as { _prepare?: boolean })._prepare === true) return;

    let prepared = false;
    const ensure = (): void => {
      if (prepared) return;
      prepared = true;
      execFileSync("node", ["scripts/presets.mjs"], {
        cwd: nuxt.options.rootDir,
        stdio: ["ignore", "inherit", "inherit"],
      });
    };

    nuxt.hook("modules:done", ensure);
  },
});
