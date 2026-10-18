// Builds the fonts the site is set in, and reports where they end up.
//
// Two jobs. The first is to run `scripts/font.mjs` before anything reads the
// font files: it builds them if they are missing and checks the reference
// against `golden/parity.json`. The second is to listen to the fonts module's
// own hooks, because where it serves a face from is an implementation detail
// rather than a URL contract, and a page that hashes a font would rather know
// where it is than guess.
import { execFileSync } from "node:child_process";
import { defineNuxtModule } from "@nuxt/kit";

/// What `@nuxt/fonts` hands a `fonts:resolved` listener. It does not augment the
/// hook map for module authors, so the shape is declared here.
interface ResolvedFont {
  fontFamily: string;
  files: { url: string }[];
}

/// Where each face is declared, so a resolution that moves one is visible in the
/// build log rather than only on the page.
const DECLARED: Record<string, string> = {
  Lemmata: "/fonts/lemmata.ttf",
  "Lemmata Math": "/fonts/lemmata-math.woff2",
};

export default defineNuxtModule({
  meta: { name: "lemmata-fonts" },
  setup(_options, nuxt) {
    let checked = false;
    const ensure = (): void => {
      if (checked) return;
      checked = true;
      execFileSync("node", ["scripts/font.mjs"], {
        cwd: nuxt.options.rootDir,
        stdio: ["ignore", "inherit", "inherit"],
      });
    };

    // The earliest point that is still before anything reads the files: a family
    // is resolved while its stylesheet is transformed, and a dev server resolves
    // on the first request for one, both of which come later than this.
    nuxt.hook("modules:done", ensure);

    const hook = nuxt.hook as unknown as (
      name: string,
      listener: (font: ResolvedFont) => void,
    ) => void;
    hook("fonts:resolved", (font) => {
      const declared = DECLARED[font.fontFamily];
      if (declared === undefined) return;
      const served = font.files.map((file) => file.url);
      if (!served.includes(declared)) {
        console.warn(
          `lemmata-fonts: ${font.fontFamily} is declared at ${declared} and served at ${served.join(", ")}`,
        );

        return;
      }
      console.log(`lemmata-fonts: ${font.fontFamily} served from ${declared}`);
    });
  },
});
