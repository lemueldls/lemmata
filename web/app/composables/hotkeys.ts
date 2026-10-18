// A key press anywhere outside a field is a shortcut. The footer lists them,
// which is the only place they are written down.
import { useTheme } from "./theme";

const ROUTES: Record<string, string> = { g: "/generate/", e: "/math/" };

export function useHotkeys(): void {
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
      if (event.key === "t") {
        toggle();
        return;
      }
      if (event.key === "p") {
        (document.querySelector("#present") as HTMLButtonElement | null)?.click();
        return;
      }
      const route = ROUTES[event.key];
      if (route) navigateTo(route);
    });
  });
}
