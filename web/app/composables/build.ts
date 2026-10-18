// The generator's half of the worker: the page and the worker exchange messages
// rather than calls, and one pending reply is enough because requests are
// serialised.
//
// The build stays on the worker thread, which is the point: a full set is tens
// of thousands of glyphs and would freeze the page.
import type { WritableComputedRef } from "vue";
import BuildWorker from "~/workers/build.ts?worker";
import { usePlan } from "~/composables/plan";

interface BuildReply {
  bytes?: Uint8Array;
  error?: string;
  elapsed?: number;
  phase?: string;
  done?: number;
  total?: number;
}

interface Build {
  plan: WritableComputedRef<string>;
  status: Ref<string>;
  state: Ref<string>;
  download: Ref<string>;
  downloadName: Ref<string>;
  bytes: ShallowRef<Uint8Array | null>;
  elapsed: Ref<number>;
  busy: Ref<boolean>;
  run: () => Promise<void>;
}

export function useBuild(): Build {
  // The plan and the capture are shared with the controls, so a choice made
  // beside the specimen is the one the build reads.
  const { plan, capture, ensure } = usePlan();
  const status = ref("Starting the worker…");
  const state = ref("note");
  const download = ref("");
  const downloadName = ref("Lemmata-Regular.ttf");
  const bytes = shallowRef<Uint8Array | null>(null);
  const elapsed = ref(0);
  const busy = ref(false);

  let worker: Worker | null = null;
  let pending: ((reply: BuildReply) => void) | null = null;

  const build = (planText: string): Promise<BuildReply> =>
    new Promise((resolve, reject) => {
      pending = (reply) =>
        reply.error ? reject(new Error(reply.error)) : resolve(reply);
      worker?.postMessage({ capture: capture.value, plan: planText });
    });

  onMounted(async () => {
    await ensure();
    worker = new BuildWorker();
    worker.addEventListener("message", (event: MessageEvent<BuildReply>) => {
      const reply = pending;
      if (!reply || !(event.data.bytes ?? event.data.error)) return;
      pending = null;
      reply(event.data);
    });
    status.value = "Ready.";
  });

  const run = async (): Promise<void> => {
    busy.value = true;
    download.value = "";
    status.value = "Building…";
    state.value = "note";
    try {
      const built = await build(plan.value);
      const font = built.bytes ?? new Uint8Array();
      bytes.value = font;
      elapsed.value = built.elapsed ?? 0;
      download.value = URL.createObjectURL(
        new Blob([font as BlobPart], { type: "font/ttf" }),
      );
      status.value = `Built ${font.length.toLocaleString()} bytes in ${elapsed.value.toFixed(0)} ms.`;
      state.value = "ok";
    } catch (error) {
      status.value = `Build failed: ${error instanceof Error ? error.message : String(error)}`;
      state.value = "bad";
    } finally {
      busy.value = false;
    }
  };

  return { plan, status, state, download, downloadName, bytes, elapsed, busy, run };
}
