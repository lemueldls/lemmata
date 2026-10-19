// The theme lives on `data-theme` on the root element. The inline script in the
// head sets it before the first paint, so this only follows the choice and
// remembers it.
//
// Three palettes: night, paper and signal. Each one names the same roles in
// `site.css`, so a swap costs no layout work.

export type Theme = "night" | "paper" | "signal";

export const THEMES: { id: Theme; short: string; label: string }[] = [
  { id: "night", short: "N", label: "Night" },
  { id: "paper", short: "P", label: "Paper" },
  { id: "signal", short: "S", label: "Signal" },
];

export const THEME_KEY = "lemmata:theme:v2";

const theme = ref<Theme>("night");

const GROUND: Record<Theme, string> = {
  night: "#0b0d0a",
  paper: "#f2efe4",
  signal: "#071018",
};

function paint(next: Theme): void {
  document.documentElement.dataset.theme = next;
  document
    .querySelector('meta[name="theme-color"]')
    ?.setAttribute("content", GROUND[next]);
}

function isTheme(value: string | null | undefined): value is Theme {
  return value === "night" || value === "paper" || value === "signal";
}

export function useTheme(): {
  theme: Ref<Theme>;
  label: ComputedRef<string>;
  setTheme: (next: Theme) => void;
  cycleTheme: () => void;
} {
  if (import.meta.client) {
    const current = document.documentElement.dataset.theme;
    if (isTheme(current)) theme.value = current;
  }

  onMounted(() => {
    const stored = localStorage.getItem(THEME_KEY);
    const current = document.documentElement.dataset.theme;
    theme.value = isTheme(stored)
      ? stored
      : isTheme(current)
        ? current
        : "night";
    paint(theme.value);
  });

  const setTheme = (next: Theme): void => {
    theme.value = next;
    paint(next);
    localStorage.setItem(THEME_KEY, next);
  };

  const cycleTheme = (): void => {
    const at = THEMES.findIndex((option) => option.id === theme.value);
    const next = THEMES[(at + 1) % THEMES.length]?.id ?? "night";
    setTheme(next);
  };

  const label = computed(
    () => THEMES.find((option) => option.id === theme.value)?.label ?? "Night",
  );

  return { theme, label, setTheme, cycleTheme };
}
