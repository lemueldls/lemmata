// Builds the fonts the site ships.
//
// Two faces come out of here. The reference font is what the landing is set in
// and what it checks against `golden/parity.json`. The math font is what the
// equations page is set in.
//
// Building them here rather than in a visitor's tab is the point. A page that
// needs a compiler to show a specimen makes every reader pay for the generator,
// and a font built at the site's build is a font the build can be held to.
//
// The fonts are committed, so `pnpm build` needs no Rust toolchain and no
// waiting. `pnpm font:write` is what regenerates them from the captures, and
// the plain `pnpm font` checks that the reference is still the font the pair
// records. The check warns rather than fails: the recorded pair is the parity
// tool's snapshot, and the builder can be ahead of it for as long as it takes
// to run `cargo xtask regen`.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { gunzipSync } from "node:zlib";

const here = dirname(fileURLToPath(import.meta.url));
const web = join(here, "..");
const root = join(web, "..");

const fonts = join(web, "fonts");
const reference = join(fonts, "lemmata.ttf");
const math = join(fonts, "lemmata-math.woff2");
const golden = JSON.parse(
  readFileSync(join(root, "golden/parity.json"), "utf8"),
);

/// The command line, from the repository's manifest, for one output. A `.woff2`
/// extension is what compresses it.
const build = (capture, out, family) =>
  execFileSync(
    "cargo",
    [
      "run",
      "--quiet",
      "--manifest-path",
      join(root, "Cargo.toml"),
      "-p",
      "lemmata-cli",
      "--",
      "build",
      "--glyphs",
      capture,
      "--out",
      out,
      ...(family ? ["--family", family] : []),
    ],
    { stdio: ["ignore", "inherit", "inherit"] },
  );

if (process.argv.includes("--write")) {
  mkdirSync(fonts, { recursive: true });

  // The reference face is the TTF rather than a compressed copy of it, because
  // the landing hashes the file it is served and compares it with the recorded
  // build. A container would change the bytes without changing the font.
  build(join(web, "public/glyphs.bin"), reference);

  // The math face needs a family of its own. It is the same typeface, and two
  // faces carrying one name would fight over it.
  const scratch = mkdtempSync(join(tmpdir(), "lemmata-font-"));
  const capture = join(scratch, "math.bin");
  writeFileSync(
    capture,
    gunzipSync(readFileSync(join(web, "fixtures/math.bin.gz"))),
  );
  build(capture, math, "Lemmata Math");
  rmSync(scratch, { recursive: true, force: true });
} else {
  for (const file of [reference, math]) {
    if (existsSync(file)) continue;
    console.error(`${file} is missing. Build the fonts with: pnpm font:write`);
    process.exit(1);
  }
}

const digest = createHash("sha256").update(readFileSync(reference)).digest("hex");
if (digest === golden.font) {
  console.log(`the reference font is the recorded build, ${digest.slice(0, 16)}…`);
} else {
  console.warn(
    "the reference font is not the build in golden/parity.json:\n" +
      `  shipped  ${digest}\n` +
      `  recorded ${golden.font}\n` +
      "The builder has moved since the pair was recorded. Regenerate it with\n" +
      "`cargo xtask regen`, then `pnpm font:write`.",
  );
}
