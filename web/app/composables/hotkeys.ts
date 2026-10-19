// The keys, one place, for the component that prints them and the listener that
// answers them.
//
// Arrows move between sections, digits jump to one, `+` and `-` set the size the
// specimens are shown at, `m` flips the theme, `r` puts the size back, and `p`
// presents the sample. `g` and `e` are the two other pages.
import { useTheme } from "./theme";

export const KEYS = ["up", "down", "digit", "+", "-", "m", "r", "g", "e"] as const;
export type Key = (typeof KEYS)[number];

/// The size the specimens are shown at, in pixels. Every page with a specimen
/// reads it, so `-` and `+` mean the same thing wherever a reader is.
export const specimenSize = ref(28);

const STEPS = [14, 16, 18, 20, 22, 24, 28, 32, 36, 40, 48, 56, 64];

function step(direction: number): void {
  const at = STEPS.findIndex((size) => size >= specimenSize.value);
  const next = STEPS[Math.min(Math.max((at === -1 ? STEPS.length - 1 : at) + direction, 0), STEPS.length - 1)];
  if (next !== undefined) specimenSize.value = next;
}

function move(direction: number): void {
  const sections = [...document.querySelectorAll("[data-key-section]")];
  if (!sections.length) return;
  const here = sections.findIndex((section) => section.getBoundingClientRect().top > 80);
  const target = sections[(here === -1 ? sections.length : here) + (direction > 0 ? 0 : -1)];
  if (target) target.scrollIntoView({ block: "start" });
}

export function useHotkeys(): { keys: readonly Key[] } {
  const { toggle } = useTheme();

  onMounted(() => {
    addEventListener("keydown", (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      const target = event.target;
      if (
        target instanceof HTMLElement &&
        (target.isContentEditable ||
          ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName))
      ) {
        return;
      }
      const routes: Record<string, string> = { g: "/generate/", e: "/math/" };
      if (routes[event.key]) {
        navigateTo(routes[event.key] as string);
        return;
      }
      switch (event.key) {
        case "ArrowDown":
          event.preventDefault();
          move(1);
          return;
        case "ArrowUp":
          event.preventDefault();
          move(-1);
          return;
        case "+":
        case "=":
          step(1);
          return;
        case "-":
          step(-1);
          return;
        case "r":
          specimenSize.value = 28;
          return;
        case "m":
          toggle();
          return;
        default:
          break;
      }
      const digit = Number.parseInt(event.key, 10);
      if (Number.isNaN(digit) || digit < 1) return;
      const sections = [...document.querySelectorAll("[data-key-section]")];
      sections[digit - 1]?.scrollIntoView({ block: "start" });
    });
  });

  return { keys: KEYS };
}
