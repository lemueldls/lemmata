// The browser suite's configuration.
//
// The page fetches `/target/glyphs.bin` and `/www/pkg`, so the static server has
// to serve the repository root rather than `www/`.

import { defineConfig } from "@playwright/test";

const port = 8781;

export default defineConfig({
  testDir: ".",
  timeout: 60_000,
  use: {
    baseURL: `http://127.0.0.1:${port}`,
    // A browser to use instead of the one Playwright resolves. The nix shell
    // provides a matched driver and browser pair and leaves this unset; a
    // checkout with some other Playwright install can point it at that one.
    launchOptions: process.env.PLAYWRIGHT_CHROMIUM
      ? { executablePath: process.env.PLAYWRIGHT_CHROMIUM }
      : {},
  },
  webServer: {
    command: `python3 -m http.server ${port} --bind 127.0.0.1`,
    cwd: "../..",
    url: `http://127.0.0.1:${port}/www/index.html`,
    reuseExistingServer: true,
  },
});
