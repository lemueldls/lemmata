// The plan the site is building, shared between the controls and the generator.
//
// One plan, because a control on one page and a build on another have to agree:
// the picker sets a choice here and the generator reads it. The capture is
// shared for the same reason, and it is also what says which choices exist, so
// the list follows the glyphs rather than a table written into the page.
import init, { default_plan, variants_of } from "lemmata-web";

const state = reactive({
  plan: "",
  capture: null as Uint8Array | null,
  ready: false,
});

let loading: Promise<void> | null = null;

/// Reads the wasm, the default plan and the capture, once.
///
/// Called by anything that needs them, and safe to call from more than one
/// place: the second caller awaits the first load rather than starting another.
function ensure(): Promise<void> {
  loading ??= (async () => {
    await init();
    state.plan = default_plan();
    const response = await fetch("/glyphs.bin");
    state.capture = new Uint8Array(await response.arrayBuffer());
    state.ready = true;
  })();

  return loading;
}

/// The plan as the page edits it: the text the generator is handed, so what the
/// page shows and what the builder reads cannot drift.
const plan = computed({
  get: () => state.plan,
  set: (value: string) => {
    state.plan = value;
  },
});

function parsed(): Record<string, unknown> {
  return state.plan.trim() ? JSON.parse(state.plan) : {};
}

/// The variants the capture holds for a group, as the names a plan takes.
///
/// The group's own drawing is not in the list: a plan gets it by saying nothing.
function variants(group: string): string[] {
  if (!state.capture) return [];

  return JSON.parse(variants_of(state.capture, group)) as string[];
}

/// The variant a group is set to, or the empty string for the capture's drawing.
function chosen(group: string): string {
  const choices = parsed().variants as Record<string, string> | undefined;

  return choices?.[group] ?? "";
}

/// Names a variant for a group. An empty name clears the choice.
function choose(group: string, variant: string): void {
  const document = parsed();
  const choices = { ...((document.variants as Record<string, string>) ?? {}) };
  if (variant) choices[group] = variant;
  else delete choices[group];
  document.variants = choices;
  state.plan = JSON.stringify(document, null, 2);
}

export function usePlan() {
  return {
    plan,
    capture: computed(() => state.capture),
    ready: computed(() => state.ready),
    ensure,
    variants,
    chosen,
    choose,
  };
}
