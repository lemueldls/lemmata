// The registry's two paths: picking a letter from the visible grid, and the
// specimen font following a plan change. The grid replaced a scrolling rail,
// and the two fonts come from the source cache rather than a full rebuild.
//
// Needs `pnpm install`, a browser, and a built site.

import { expect, test } from "@playwright/test";

test("the registry selects a letter from the grid", async ({ page }) => {
  const failures = [];
  page.on("console", (message) => {
    if (message.type() === "error") failures.push(message.text());
  });

  await page.goto("/generate/");
  await page.locator(".registry__letter").first().waitFor();
  expect(await page.locator(".registry__letter").count()).toBeGreaterThan(5);
  expect(await page.locator(".registry__rail, .registry__select-trigger").count()).toBe(0);

  // The second tile is b, in the lower-case-first order.
  await page.locator(".registry__letter").nth(1).click();
  await expect(page.locator(".registry__stage-kicker")).toContainText("b /");
  expect(await page.locator(".registry__card").count()).toBeGreaterThan(1);

  // The m group has enough drawings to need the card scroller.
  await page.locator(".registry__letter").nth(5).click();
  const scroll = page.locator(".registry__cards-scroll");
  const sizes = await scroll.evaluate((element) => ({
    client: element.clientHeight,
    full: element.scrollHeight,
  }));
  expect(sizes.full).toBeGreaterThan(sizes.client);
  await scroll.evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });
  expect(await scroll.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
  expect(failures).toEqual([]);
});

test("the registry redraws when the plan changes", async ({ page }) => {
  await page.goto("/generate/");
  await page.locator(".registry__letter").first().waitFor();
  await page.waitForFunction(() =>
    getComputedStyle(document.querySelector(".registry__big")).fontFamily.includes(
      "Lemmata Registry Group",
    ),
  );

  const family = () =>
    page.locator(".registry__big").evaluate((element) => {
      return getComputedStyle(element).fontFamily;
    });
  const before = await family();

  const weight = page.locator(".field").filter({ hasText: "weight" });
  await weight.locator(".number-control__step").first().click();

  await expect.poll(family, { timeout: 5_000 }).not.toBe(before);

  const plan = JSON.parse(await page.locator("textarea#plan").inputValue());
  expect(plan.weight).toBe(500);
});
