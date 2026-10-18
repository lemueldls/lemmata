// The page and this worker exchange messages rather than calls:
//
//   page -> worker  { capture, plan }        build
//   worker -> page  { phase, done, total }   progress
//   page -> worker  { ask: "plan" }          the reference plan
//   page -> worker  { cancel }               stop
//   worker -> page  { plan }                 the reference plan
//   worker -> page  { bytes } | { error }    the font, or why not
//
// One message is missing in practice. `build_font` is a synchronous call, so a
// cancel cannot interrupt it; it sets a flag that the next request checks, and
// progress is a phase rather than a count. Real cancellation and real progress
// need a build that yields between glyphs, which is work in the wasm rather
// than in this protocol.
//
// The build stays on the worker thread, which is the point: a full set is tens
// of thousands of glyphs and would freeze the page.

import init, { build_font, default_plan } from "lemmata-web";

// The worker has its own wasm instance, so it initialises its own glue.
// Importing `build_font` without awaiting this is how the first browser run
// failed: `__wbindgen_malloc` is not there until the module is instantiated.
const ready = init();

let cancelled = false;

self.onmessage = async (event) => {
  const message = event.data;

  if (message.cancel) {
    cancelled = true;
    self.postMessage({ phase: "cancelled" });
    return;
  }

  if (cancelled) {
    self.postMessage({ error: "cancelled" });
    return;
  }

  if (message.ask === "plan") {
    await ready;
    self.postMessage({ plan: default_plan() });
    return;
  }

  try {
    await ready;
    self.postMessage({ phase: "building", done: 0, total: 1 });
    const started = performance.now();
    // A `Uint8Array`, not an `ArrayBuffer`: wasm-bindgen's `&[u8]` reads
    // `length`, and an `ArrayBuffer` only has `byteLength`, which is how this
    // reported a truncated capture.
    const capture =
      message.capture instanceof Uint8Array
        ? message.capture
        : new Uint8Array(message.capture);
    const bytes = build_font(capture, message.plan);
    self.postMessage({
      bytes,
      elapsed: performance.now() - started,
      phase: "done",
      done: 1,
      total: 1,
    });
  } catch (error) {
    self.postMessage({ error: String(error) });
  } finally {
    cancelled = false;
  }
};
