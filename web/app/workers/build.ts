// The worker's side of the generator.
//
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
// cancel cannot interrupt it. It sets a flag that the next request reads, and
// progress is a phase rather than a count. Real cancellation and real progress
// need a build that yields between glyphs, which is work in the module rather
// than in this protocol.
//
// The build runs on the worker thread, which is the point: a full set is tens
// of thousands of glyphs and would freeze the page.

import init, { build_font, default_plan, preview_font } from "lemmata-web";

interface Request {
  capture?: Uint8Array | ArrayBuffer;
  plan?: string | boolean;
  /** A short text to preview. When present the request is a preview. */
  text?: string;
  id?: number;
  ask?: string;
  cancel?: boolean;
}

/// `self` in a worker is a different scope from the one the DOM library
/// declares, and the whole of what this file uses of it is these two.
interface WorkerScope {
  postMessage(message: unknown): void;
}

const scope = self as unknown as WorkerScope;

// The worker has its own module instance, so it initialises its own glue.
// Importing `build_font` without awaiting this is how the first browser run
// failed: `__wbindgen_malloc` is not there until the module is instantiated.
const ready = init();

let cancelled = false;

self.onmessage = async (event: MessageEvent<Request>): Promise<void> => {
  const message = event.data;

  if (message.cancel) {
    cancelled = true;
    scope.postMessage({ phase: "cancelled" });
    return;
  }

  if (cancelled) {
    scope.postMessage({ error: "cancelled" });
    return;
  }

  if (message.ask === "plan") {
    await ready;
    scope.postMessage({ plan: default_plan() });
    return;
  }

  try {
    await ready;
    const capture =
      message.capture instanceof Uint8Array
        ? message.capture
        : new Uint8Array(message.capture ?? new ArrayBuffer(0));
    const started = performance.now();

    // A preview recomputes only the glyphs the text draws, so it is cheap
    // enough to run on every plan change.
    if (typeof message.text === "string") {
      const bytes = preview_font(capture, String(message.plan ?? ""), message.text);
      scope.postMessage({
        id: message.id,
        bytes,
        elapsed: performance.now() - started,
        phase: "preview",
        done: 1,
        total: 1,
      });
      return;
    }

    scope.postMessage({ phase: "building", done: 0, total: 1 });
    // A `Uint8Array`, not an `ArrayBuffer`: the binding's `&[u8]` reads
    // `length`, and an `ArrayBuffer` only has `byteLength`, which is how this
    // reported a truncated capture.
    const bytes = build_font(capture, String(message.plan ?? ""));
    scope.postMessage({
      id: message.id,
      bytes,
      elapsed: performance.now() - started,
      phase: "done",
      done: 1,
      total: 1,
    });
  } catch (error) {
    scope.postMessage({
      id: message.id,
      error: error instanceof Error ? error.message : String(error),
    });
  } finally {
    cancelled = false;
  }
};
