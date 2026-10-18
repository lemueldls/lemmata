// The browser's half of the parity pair.
//
// `harness/parity.mjs` checks the wasm build under node; this checks it through
// a real page, which is the only place the boundary the site actually uses is
// exercised. Both compare against `golden/parity.json`, so all three builds
// have to agree.
//
// Needs `pnpm install` and a browser: `nix develop .#browser` provides both.

import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { expect, test } from "@playwright/test";

const golden = JSON.parse(
  readFileSync(new URL("../../golden/parity.json", import.meta.url), "utf8"),
);
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

test("the page builds the recorded pair, byte for byte", async ({ page }) => {
  const failures = [];
  page.on("console", (message) => {
    if (message.type() === "error") failures.push(message.text());
  });

  await page.goto("/www/index.html");
  await expect(page.locator("#status")).toHaveText("Ready.");
  await page.locator("#plan").fill(JSON.stringify(golden.plan));
  await page.locator("#build").click();
  await expect(page.locator("#status")).toContainText("Built");

  const bytes = await page.evaluate(async () => {
    const response = await fetch(document.querySelector("#download").href);
    return Array.from(new Uint8Array(await response.arrayBuffer()));
  });
  expect(failures).toEqual([]);
  expect(sha256(Buffer.from(bytes))).toBe(golden.font);
});
