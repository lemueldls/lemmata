// The theme lives on `data-theme` on the root element. The inline script in the
// head sets it before the first paint and the stylesheet reads it, so this only
// has to follow along and remember the choice.

export type Theme = "dark" | "light";

const theme = ref<Theme>("dark");

export function useTheme(): { theme: Ref<Theme>; toggle: () => void } {
  onMounted(() => {
    theme.value = (document.documentElement.dataset.theme as Theme) ?? "dark";
  });

  const toggle = (): void => {
    const next: Theme =
      document.documentElement.dataset.theme === "light" ? "dark" : "light";
    document.documentElement.dataset.theme = next;
    localStorage.setItem("lemmata:theme", next);
    theme.value = next;
  };

  return { theme, toggle };
}
