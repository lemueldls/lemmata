// Code samples, highlighted by Shiki.
//
// Shiki ships Everforest as one of its themes, so the samples and the page's
// chrome come from a single palette rather than two that nearly match. Both
// themes go into the same markup as `--shiki-light` and `--shiki-dark`, and the
// stylesheet picks between them, so changing the page's theme needs nothing
// here and no second pass.
//
// The JavaScript regex engine is the one that does not fetch a second wasm blob
// at runtime: the generator already downloads one, and a handful of samples do
// not need the oniguruma features.
import { createHighlighterCore } from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";
import lean4 from "shiki/langs/lean4.mjs";
import python from "shiki/langs/python.mjs";
import shellscript from "shiki/langs/shellscript.mjs";
import everforestDark from "shiki/themes/everforest-dark.mjs";
import everforestLight from "shiki/themes/everforest-light.mjs";

const THEMES = { light: "everforest-light", dark: "everforest-dark" };

type Highlighter = Awaited<ReturnType<typeof createHighlighterCore>>;

let started: Promise<Highlighter> | null = null;

/// Built once: the grammars are the same for every sample and loading them is
/// the slow part.
function engine(): Promise<Highlighter> {
  started ??= createHighlighterCore({
    themes: [everforestDark, everforestLight],
    langs: [lean4, python, shellscript],
    engine: createJavaScriptRegexEngine(),
  });

  return started;
}

/// The markup for one sample, carrying both themes.
export async function highlight(source: string, lang: string): Promise<string> {
  const shiki = await engine();

  // Shiki marks its output focusable so a long block can be scrolled from the
  // keyboard. The page has its own focus story and seven more stops in the
  // middle of it is noise.
  return shiki
    .codeToHtml(source, { lang, themes: THEMES, defaultColor: false })
    .replace(' tabindex="0"', "");
}
