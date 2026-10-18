// The browser suite's configuration.
//
// The page fetches `/target/glyphs.bin` and `/www/pkg`, so the static server has
// to serve the repository root rather than `www/`.

import { defineConfig } from "@playwright/test";

const port = 8781;
const sitePort = 8782;

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
  // Two servers, because they are two different roots. The built site is served
  // at the root of its own, which is what a deploy does and what its
  // root-absolute asset URLs assume. The prototype pages live at the repository
  // root and are reached from there until they are retired.
  webServer: [
    {
      command: `python3 -m http.server ${port} --bind 127.0.0.1`,
      cwd: "../..",
      url: `http://127.0.0.1:${port}/www/index.html`,
      reuseExistingServer: true,
    },
    {
      command: `python3 -m http.server ${sitePort} --bind 127.0.0.1`,
      cwd: "../../www/dist",
      url: `http://127.0.0.1:${sitePort}/index.html`,
      reuseExistingServer: true,
    },
  ],
});
