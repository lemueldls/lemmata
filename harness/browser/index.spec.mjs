// The index page sets one equation in the math face it builds for that section
// and registers under its own name, which is a different path from the math
// page's. Both went unnoticed for the same reason: the family was set on a
// container, and MathML's user agent stylesheet sets `font-family: math` on the
// `math` element itself, which beats inheritance. So the container looked right
// and every equation was set in the browser's math font.
//
// Needs `pnpm install` and a browser: `nix develop .#browser` provides both.

import { expect, test } from "@playwright/test";

test("the index equation is set in the face the page built", async ({ page }) => {
  const failures = [];
  page.on("console", (message) => {
    if (message.type() === "error") failures.push(message.text());
  });

  await page.goto("/");
  await page.locator("#corollary").scrollIntoViewIfNeeded();
  await page.waitForFunction(
    () => document.querySelector("#equation")?.dataset.face === "ready",
    null,
    { timeout: 60_000 },
  );
  expect(failures).toEqual([]);

  // The `math` element, not the holder around it.
  await expect(page.locator("#equation math").first()).toHaveCSS(
    "font-family",
    /Lemmata/,
  );

  // And the rendered box: the same equation in an absent family has to measure
  // differently, or the family reached the rule and not the glyphs.
  const applied = await page.evaluate(() => {
    const set = document.querySelector("#equation math");
    const host = document.createElement("div");
    host.style.fontFamily = '"Lemmata Absent", serif';
    host.append(set.cloneNode(true));
    document.body.append(host);
    return {
      ours: set.querySelector("mo").getBoundingClientRect().width,
      absent: host.querySelector("mo").getBoundingClientRect().width,
    };
  });
  expect(applied.ours).not.toBe(applied.absent);
});
