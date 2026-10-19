// The keyboard layer, shared by the page that prints the keys and the listener
// that answers them.
//
// The map follows the one a code editor teaches: arrows jump between marked
// blocks, digits go straight to one, `t` cycles the palette, `+` and `-` resize
// a specimen, and `g` and `e` open the two work areas. The help panel is the
// same list, so a reader never has to remember it.
import { useTheme } from "./theme";

export const KEYS = [
  "arrows",
  "1-9",
  "w s",
  "j k",
  "t",
  "+ -",
  "r",
  "g",
  "e",
  "i",
  "b",
  "d",
  "?",
] as const;
export type Key = (typeof KEYS)[number];

/// The size every specimen uses, in pixels.
export const specimenSize = ref(28);

/// Whether the key map is up.
export const helpOpen = ref(false);

const STEPS = [12, 14, 16, 18, 20, 22, 24, 26, 28, 32, 36, 42, 48, 56, 64, 80];

function stepSize(direction: number): void {
  const at = STEPS.findIndex((size) => size >= specimenSize.value);
  const index = Math.min(
    Math.max((at === -1 ? STEPS.length - 1 : at) + direction, 0),
    STEPS.length - 1,
  );
  specimenSize.value = STEPS[index] ?? 28;
}

function sections(): HTMLElement[] {
  return [...document.querySelectorAll<HTMLElement>("[data-key-section]")];
}

function go(direction: number): void {
  const blocks = sections();
  if (!blocks.length) return;
  const line = 120;
  const here = blocks.findIndex(
    (block) => block.getBoundingClientRect().top > line,
  );
  const index = (here === -1 ? blocks.length : here) + (direction > 0 ? 0 : -1);
  blocks[index]?.scrollIntoView({ block: "start" });
}

function jump(section: number): void {
  sections()[section - 1]?.scrollIntoView({ block: "start" });
}

function typing(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable ||
      ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName))
  );
}

function act(name: string): void {
  window.dispatchEvent(new CustomEvent("lemmata:action", { detail: name }));
}

let installed = false;

export function useHotkeys(): void {
  const { cycleTheme } = useTheme();

  if (installed) return;

  onMounted(() => {
    if (installed) return;
    installed = true;

    addEventListener("keydown", (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey) return;

      if (event.key === "Escape") {
        if (helpOpen.value) {
          helpOpen.value = false;
          return;
        }
        if (typing(event.target)) (event.target as HTMLElement).blur();
        return;
      }

      if (typing(event.target)) return;

      const key = event.key;
      const lower = key.toLowerCase();

      if (key === "?" || lower === "h") {
        event.preventDefault();
        helpOpen.value = !helpOpen.value;
        return;
      }

      if (lower === "t" || lower === "m") {
        event.preventDefault();
        cycleTheme();
        return;
      }

      if (lower === "g") {
        event.preventDefault();
        void navigateTo("/generate/");
        return;
      }

      if (lower === "e") {
        event.preventDefault();
        void navigateTo("/math/");
        return;
      }

      if (lower === "i") {
        event.preventDefault();
        void navigateTo("/");
        return;
      }

      if (lower === "b" || lower === "d") {
        event.preventDefault();
        act(lower === "b" ? "build" : "download");
        return;
      }

      if (key === "+" || key === "=") {
        event.preventDefault();
        stepSize(1);
        return;
      }

      if (key === "-" || key === "_") {
        event.preventDefault();
        stepSize(-1);
        return;
      }

      if (lower === "r") {
        event.preventDefault();
        specimenSize.value = 28;
        return;
      }

      if (key === "ArrowDown" || key === "ArrowUp") {
        event.preventDefault();
        go(key === "ArrowDown" ? 1 : -1);
        return;
      }

      if (lower === "j" || lower === "k") {
        event.preventDefault();
        scrollBy({ top: lower === "j" ? 160 : -160, behavior: "smooth" });
        return;
      }

      if (lower === "w" || lower === "s") {
        event.preventDefault();
        scrollBy({ top: lower === "s" ? 360 : -360, behavior: "smooth" });
        return;
      }

      const digit = Number.parseInt(key, 10);
      if (Number.isNaN(digit) || digit < 1) return;
      event.preventDefault();
      jump(digit);
    });
  });
}
