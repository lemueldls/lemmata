// The build card is the specimen now, so its sample has to be editable and its
// feature switches have to reach the text directly.

import { expect, test } from "@playwright/test";

test("the build card sample edits and switches features", async ({ page }) => {
  await page.goto("/generate/");
  const sample = page.locator(".build-card__sample");
  await sample.waitFor();

  await sample.fill("edit this specimen");
  await expect(sample).toHaveValue("edit this specimen");

  const texture = page.locator(".build-card__features .feature").first();
  await texture.click();
  const settings = await sample.evaluate((element) => {
    return getComputedStyle(element).fontFeatureSettings;
  });
  expect(settings).toContain('"calt" 0');

  await page.locator(".build-card__sample-foot .text-button").click();
  await expect(sample).not.toHaveValue("edit this specimen");
});
