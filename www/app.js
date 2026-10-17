// The page's half of the browser build. Everything that takes real work happens
// in `lemmata_web`, so this only fetches the input, hands it over, and offers
// the result as a download.

import init, { build_font, output_name } from "./pkg/lemmata_web.js";

const status = document.querySelector("#status");
const build = document.querySelector("#build");
const download = document.querySelector("#download");

// The build is a function of its input, so both are recorded and the page's
// state is readable from the outside. That is what the end-to-end check reads.
const state = { ready: false, byteLength: 0, error: null, name: null };
window.__lemmata = state;

const say = (text) => {
  status.textContent = text;
};

const report = (error) => {
  state.error = String(error);
  say(`Build failed: ${error}`);
};

try {
  await init();
  state.name = output_name();
  state.ready = true;
  say("Ready.");
} catch (error) {
  report(error);
}

build.addEventListener("click", async () => {
  build.disabled = true;
  download.hidden = true;
  say("Fetching the glyph set...");
  try {
    const response = await fetch("../target/glyphs.bin");
    if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
    const capture = new Uint8Array(await response.arrayBuffer());

    say("Building...");
    const started = performance.now();
    const bytes = build_font(capture);
    const elapsed = performance.now() - started;

    state.byteLength = bytes.length;
    state.error = null;

    const url = URL.createObjectURL(
      new Blob([bytes], { type: "font/ttf" }),
    );
    download.href = url;
    download.download = state.name;
    download.hidden = false;
    say(
      `Built ${bytes.length.toLocaleString()} bytes in ` +
        `${elapsed.toFixed(0)} ms.`,
    );
  } catch (error) {
    report(error);
  } finally {
    build.disabled = false;
  }
});
