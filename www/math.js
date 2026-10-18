// Renders the equation corpus with the font the page just built, which is what
// r[math.mathml] asks for: the same equations the Typst harness compiles, set
// in a browser from our own outlines.

import init, { build_font, default_plan } from "./pkg/lemmata_web.js";

const status = document.querySelector("#status");
const equations = document.querySelector("#equations");

/// Measures a string in a family, to tell whether a font rendered it.
function widthOf(family, text) {
  const canvas = document.createElement("canvas");
  const context = canvas.getContext("2d");
  context.font = `32px ${family}`;
  return context.measureText(text).width;
}

async function run() {
  await init();

  // The math capture, not the coverage one: the corpus needs the alphanumerics
  // and the operators as well as the letters.
  const response = await fetch("../target/math.bin");
  const capture = new Uint8Array(await response.arrayBuffer());
  const bytes = build_font(capture, default_plan());

  const face = new FontFace("Lemmata", bytes.buffer);
  await face.load();
  document.fonts.add(face);
  await document.fonts.load("16px Lemmata");

  const corpus = await (await fetch("../corpus/equations.mathml")).text();
  equations.innerHTML = corpus;

  // A renderer that cannot serve the corpus falls back, so the check is that
  // the family changes the measurement. The sample is the constructs a math
  // font has to carry: a radical, a large operator, an integral, Greek.
  const ours = widthOf("Lemmata", "\u2211 \u222B \u221A x \u03B1");
  const fallback = widthOf('"Lemmata Absent"', "\u2211 \u222B \u221A x \u03B1");
  const rendered = document.querySelectorAll("#equations math").length;

  // A delimiter grown to fit its content is built by stacking pieces, so it
  // stays as wide as the glyph it was cut from, while a renderer with no
  // assembly scales the whole glyph and widens it. Recording both measures
  // leaves the difference visible without asserting on it: whether a browser
  // stretches at all is the browser's business, and the geometric test is what
  // covers the pieces.
  document.body.dataset.measure = JSON.stringify(
    [...document.querySelectorAll("#equations mo")]
      .filter((mo) => mo.textContent === "(" || mo.textContent === ")")
      .map((mo) => {
        const box = mo.getBoundingClientRect();
        return { width: Math.round(box.width), height: Math.round(box.height) };
      }),
  );

  status.textContent =
    ours !== fallback && rendered > 0
      ? `Ready. ${rendered} equations.`
      : `Fallback. ${rendered} equations.`;
}

run().catch((error) => {
  status.textContent = `Failed: ${error}`;
});
