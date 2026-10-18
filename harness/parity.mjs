// The wasm half of the parity pair.
//
// The native half is `crates/lemmata-web/tests/parity.rs`. Both compare against
// `golden/parity.json`, so a pair that was byte-identical under a pinned
// toolchain and is not any more fails on either side. `cargo xtask parity` runs
// both.
//
// Nothing here is installed: node's own crypto and the wasm-bindgen
// `--target nodejs` build are enough, so this runs offline and in CI.
//
//   wasm-pack build crates/lemmata-web --target nodejs --out-dir target/lemmata-node
//   node harness/parity.mjs

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

const golden = JSON.parse(readFileSync(join(root, "golden/parity.json"), "utf8"));
const capture = readFileSync(
  process.env.LEMMATA_GLYPH_DUMP ?? join(root, "target/glyphs.bin"),
);

const recorded = golden.capture;
if (sha256(capture) !== recorded) {
  console.error(
    `parity: the capture is not the one the pair was recorded from\n` +
      `  recorded ${recorded}\n  this     ${sha256(capture)}`,
  );
  process.exit(1);
}

const require = createRequire(import.meta.url);
const { build_font } = require(join(root, "target/lemmata-node/lemmata_web.js"));
const bytes = Buffer.from(build_font(capture, JSON.stringify(golden.plan)));

const ours = sha256(bytes);
if (ours !== golden.font) {
  console.error(
    `parity: the wasm build no longer matches the recorded pair.\n` +
      `  If the change is intended, regenerate with \`cargo xtask regen\` and a\n` +
      `  drift report, as golden/README.md asks.\n` +
      `  recorded ${golden.font}\n  this     ${ours}`,
  );
  process.exit(1);
}

console.log(
  `parity: wasm ${ours.slice(0, 16)} matches the recorded pair ` +
    `(rustc ${golden.toolchain.rustc}, wasm-bindgen ${golden.toolchain["wasm-bindgen"]}), ` +
    `byte-identical to the native build`,
);
