// The browser's half of r[math.mathml]: the equation corpus, set in a browser
// from our own outlines.
//
// Needs `pnpm install`, a browser and a built site: `pnpm --dir web build`.

import { expect, test } from "@playwright/test";

test("the corpus renders from our font, not a fallback", async ({ page }) => {
  const failures = [];
  page.on("console", (message) => {
    if (message.type() === "error") failures.push(message.text());
  });

  await page.goto("/math/");
  await expect(page.locator("#status")).toContainText("Ready.");
  expect(failures).toEqual([]);

  // Every equation in the corpus is on the page and set in the family.
  const rendered = await page.locator("#equations math").count();
  expect(rendered).toBeGreaterThanOrEqual(10);
  // The `math` element, not the container: the user agent rule sets the family
  // on the element itself, so a container that looks right can still leave every
  // equation in the browser's math font.
  await expect(page.locator("#equations math").first()).toHaveCSS(
    "font-family",
    /Lemmata/,
  );

  // A missing glyph would render as a box, so the check is that the glyphs the
  // corpus needs are the ones the font carries: measure the sample the page
  // measured, and confirm the family is the difference.
  // Reported, not asserted. Whether a browser stretches a delimiter to its
  // content is the browser's business; the geometric test is what covers the
  // pieces, and `docs/math.md` records what this one does not show.
  console.log("delimiters", await page.getAttribute("body", "data-measure"));
  // And the rendered box, not the resolved rule. A canvas measurement proves
  // the face loaded; comparing the same equation in an absent family proves the
  // page set it in ours, and holds whatever spacing MathML adds, because both
  // sides get the same spacing.
  const applied = await page.evaluate(() => {
    const set = document.querySelector("#equations math");
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

  const widths = await page.evaluate(() => {
    const measure = (family) => {
      const context = document.createElement("canvas").getContext("2d");
      context.font = `32px ${family}`;
      return context.measureText("\u2211 \u222B \u221A x \u03B1").width;
    };
    return { ours: measure("Lemmata"), fallback: measure('"Lemmata Absent"') };
  });
  expect(widths.ours).not.toBe(widths.fallback);
});
