// The browser suite's configuration.
//
// The page is served from the built site, so the server's root is the directory
// `nuxt generate` writes: `web/.output/public`.

import { defineConfig } from "@playwright/test";

const sitePort = 8781;

export default defineConfig({
  testDir: ".",
  timeout: 60_000,
  use: {
    baseURL: `http://127.0.0.1:${sitePort}`,
    // A browser to use instead of the one Playwright resolves. The nix shell
    // provides a matched driver and browser pair and leaves this unset; a
    // checkout with some other Playwright install can point it at that one.
    launchOptions: process.env.PLAYWRIGHT_CHROMIUM
      ? { executablePath: process.env.PLAYWRIGHT_CHROMIUM }
      : {},
  },
  // The built site is served at the root of its own, which is what a deploy
  // does and what its root-absolute asset URLs assume.
  webServer: {
    command: `python3 -m http.server ${sitePort} --bind 127.0.0.1`,
    cwd: "../../web/.output/public",
    url: `http://127.0.0.1:${sitePort}/index.html`,
    reuseExistingServer: true,
  },
});
