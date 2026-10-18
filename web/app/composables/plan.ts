// The plan the site is building, shared between the controls and the generator.
//
// One plan, because a control on one page and a build on another have to agree:
// the picker sets a choice here and the generator reads it. The capture is
// shared for the same reason, and it is also what says which choices exist, so
// the list follows the glyphs rather than a table written into the page.
import init, { default_plan, variants_of } from "lemmata-web";

/// Where the outlines for each spacing live.
///
/// The spacing is a property of the drawings rather than of the advances:
/// Iosevka draws each glyph into a frame whose width comes from its letter
/// class, so the quasi-proportional face is a second build and a second
/// capture. The plan says which one it wants and this is what that means.
const DEFAULT_CAPTURE = "/glyphs.bin";
const CAPTURES: Record<string, string> = {
  normal: DEFAULT_CAPTURE,
  "quasi-proportional": "/glyphs-quasi.bin",
};

const state = reactive({
  plan: "",
  captures: new Map<string, Uint8Array>(),
  ready: false,
});

let started: Promise<void> | null = null;

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

/// The spacing the plan asks for, defaulting the way the builder does.
function spacing(): string {
  return (parsed().spacing as string) ?? "normal";
}

/// The outlines the current spacing needs, fetched once per spacing.
async function load(which: string): Promise<void> {
  if (state.captures.has(which)) return;
  const response = await fetch(CAPTURES[which] ?? DEFAULT_CAPTURE);
  state.captures.set(which, new Uint8Array(await response.arrayBuffer()));
}

const capture = computed(() => state.captures.get(spacing()) ?? null);

/// Reads the wasm, the default plan and the capture the plan asks for.
///
/// Safe to call from more than one place and after the plan changes: the wasm
/// loads once and the capture loads once per spacing.
async function ensure(): Promise<void> {
  started ??= (async () => {
    await init();
    state.plan = default_plan();
    state.ready = true;
  })();
  await started;
  await load(spacing());
}

/// Sets the spacing, which is also what decides the capture.
async function setSpacing(which: string): Promise<void> {
  const document = parsed();
  document.spacing = which;
  state.plan = JSON.stringify(document, null, 2);
  await load(which);
}

/// The variants the capture holds for a group, as the names a plan takes.
///
/// The group's own drawing is not in the list: a plan gets it by saying nothing.
function variants(group: string): string[] {
  const bytes = capture.value;
  if (!bytes) return [];

  return JSON.parse(variants_of(bytes, group)) as string[];
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
    capture,
    ready: computed(() => state.ready),
    spacing: computed(spacing),
    setSpacing,
    ensure,
    variants,
    chosen,
    choose,
  };
}
