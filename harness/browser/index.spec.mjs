// The index page sets the equation gallery in the math face, which is a
// different path from the math page's editor. Both were once left in the
// browser's math font because the family was set on a container and MathML's
// user agent rule sets `font-family` on the `math` element itself.
//
// Needs `pnpm install` and a browser: `nix develop .#browser` provides both.

import { expect, test } from "@playwright/test";

test("the plus marks sit on one column grid", async ({ page }) => {
  await page.goto("/");
  await page.locator(".grid-guide__cross").first().waitFor();

  const xs = await page.evaluate(() =>
    [
      ...new Set(
        [...document.querySelectorAll(".grid-guide__cross")].map((cross) => {
          const box = cross.getBoundingClientRect();

          return Math.round(box.left + box.width / 2);
        }),
      ),
    ].sort((left, right) => left - right),
  );

  expect(xs).toHaveLength(5);
  expect(xs[0]).toBeGreaterThan(0);
  expect(xs[4]).toBeLessThan(1440);
});

test("the index equation is set in the face the page loaded", async ({ page }) => {
  const failures = [];
  page.on("console", (message) => {
    if (message.type() === "error") failures.push(message.text());
  });

  await page.goto("/");
  await page.locator("#math").scrollIntoViewIfNeeded();
  await page.waitForFunction(
    () => document.fonts.check('1em "Lemmata Math"'),
    null,
    { timeout: 60_000 },
  );
  expect(failures).toEqual([]);

  // The `math` element, not the holder around it.
  const equation = page.locator("#math .eq-card__body math").first();
  await expect(equation).toHaveCSS("font-family", /Lemmata/);

  // And the rendered box: the same equation in an absent family has to measure
  // differently, or the family reached the rule and not the glyphs.
  const applied = await page.evaluate(() => {
    const set = document.querySelector("#math .eq-card__body math");
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
