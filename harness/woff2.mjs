// The browser half of `r[build.formats]`: the wasm module compresses a font to
// WOFF2, so a page can serve the container without the CLI.
//
// Nothing here is installed, like `parity.mjs`. `cargo xtask parity` runs both.
//
//   wasm-pack build crates/lemmata-web --target nodejs --out-dir target/lemmata-node
//   node harness/woff2.mjs

import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const capture = readFileSync(join(root, "web/public/glyphs.bin"));
const require = createRequire(import.meta.url);
const { build_font, default_plan, to_woff2 } = require(
  join(root, "target/lemmata-node/lemmata_web.js"),
);

const ttf = Buffer.from(build_font(capture, default_plan()));
const woff2 = Buffer.from(to_woff2(ttf));

if (woff2.subarray(0, 4).toString("latin1") !== "wOF2") {
  console.error("woff2: the wasm container does not name itself");
  process.exit(1);
}
if (woff2.length >= ttf.length) {
  console.error(
    `woff2: ${woff2.length} bytes from ${ttf.length}, which is not compression`,
  );
  process.exit(1);
}

console.log(`woff2: the wasm container is ${woff2.length} bytes from ${ttf.length}`);
