// Code samples, highlighted at build time.
//
// The palette is the site's own: night, paper and signal share the green and
// amber accents, so the samples never look pasted in from another project. Both
// palettes go into one markup as `--shiki-light` and `--shiki-dark`. The
// stylesheet picks between them, which makes a theme swap free.
//
// The JavaScript regex engine is the one that does not fetch a second grammar
// blob at runtime. A handful of samples do not need the Oniguruma features.
import { createHighlighterCore } from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";
import diff from "shiki/langs/diff.mjs";
import json from "shiki/langs/json.mjs";
import lean4 from "shiki/langs/lean4.mjs";
import python from "shiki/langs/python.mjs";
import shellscript from "shiki/langs/shellscript.mjs";

const night = {
  name: "lemmata-night",
  type: "dark" as const,
  colors: {
    "editor.background": "#00000000",
    "editor.foreground": "#e9e7d8",
  },
  tokenColors: [
    {
      scope: ["comment", "punctuation.definition.comment"],
      settings: { foreground: "#737866", fontStyle: "italic" },
    },
    {
      scope: ["keyword", "storage", "storage.type", "storage.modifier"],
      settings: { foreground: "#d9ff54" },
    },
    {
      scope: [
        "string",
        "string.quoted",
        "string.template",
        "punctuation.definition.string",
      ],
      settings: { foreground: "#ffbe7a" },
    },
    {
      scope: [
        "constant.numeric",
        "constant.language",
        "constant.character.escape",
        "constant.other",
      ],
      settings: { foreground: "#ff7a45" },
    },
    {
      scope: [
        "entity.name.function",
        "support.function",
        "meta.function-call",
      ],
      settings: { foreground: "#8ecbff" },
    },
    {
      scope: [
        "entity.name.type",
        "entity.name.class",
        "support.type",
        "support.class",
      ],
      settings: { foreground: "#7ee0c2" },
    },
    {
      scope: [
        "variable",
        "variable.parameter",
        "meta.object-literal.key",
        "punctuation.separator.key-value",
      ],
      settings: { foreground: "#e9e7d8" },
    },
    {
      scope: ["punctuation", "keyword.operator"],
      settings: { foreground: "#a4a893" },
    },
    {
      scope: ["entity.name.tag", "support.type.property-name"],
      settings: { foreground: "#c7a6ff" },
    },
  ],
};

const paper = {
  name: "lemmata-paper",
  type: "light" as const,
  colors: {
    "editor.background": "#00000000",
    "editor.foreground": "#161811",
  },
  tokenColors: [
    {
      scope: ["comment", "punctuation.definition.comment"],
      settings: { foreground: "#7d826c", fontStyle: "italic" },
    },
    {
      scope: ["keyword", "storage", "storage.type", "storage.modifier"],
      settings: { foreground: "#4f6b00" },
    },
    {
      scope: [
        "string",
        "string.quoted",
        "string.template",
        "punctuation.definition.string",
      ],
      settings: { foreground: "#a94e1e" },
    },
    {
      scope: [
        "constant.numeric",
        "constant.language",
        "constant.character.escape",
        "constant.other",
      ],
      settings: { foreground: "#b03a12" },
    },
    {
      scope: [
        "entity.name.function",
        "support.function",
        "meta.function-call",
      ],
      settings: { foreground: "#245d7a" },
    },
    {
      scope: [
        "entity.name.type",
        "entity.name.class",
        "support.type",
        "support.class",
      ],
      settings: { foreground: "#006d5c" },
    },
    {
      scope: [
        "variable",
        "variable.parameter",
        "meta.object-literal.key",
        "punctuation.separator.key-value",
      ],
      settings: { foreground: "#161811" },
    },
    {
      scope: ["punctuation", "keyword.operator"],
      settings: { foreground: "#555b48" },
    },
    {
      scope: ["entity.name.tag", "support.type.property-name"],
      settings: { foreground: "#6b3fa0" },
    },
  ],
};

const THEMES = { light: "lemmata-paper", dark: "lemmata-night" };

type Highlighter = Awaited<ReturnType<typeof createHighlighterCore>>;

let started: Promise<Highlighter> | null = null;

function engine(): Promise<Highlighter> {
  started ??= createHighlighterCore({
    themes: [night, paper],
    langs: [diff, json, lean4, python, shellscript],
    engine: createJavaScriptRegexEngine(),
  });

  return started;
}

/// The markup for one sample, holding both palettes.
export async function highlight(source: string, lang: string): Promise<string> {
  const shiki = await engine();

  return shiki
    .codeToHtml(source, { lang, themes: THEMES, defaultColor: false })
    .replace(' tabindex="0"', "");
}
