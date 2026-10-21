// Builds the preset fonts the API returns.
//
// The plans live in `shared/presets.json`, so the list the page shows and the
// list the API returns cannot drift. A preset is built twice: a TTF for editors
// and a WOFF2 for the web. The quasi plan gets the quasi capture, because the
// spacing is in the drawings rather than in the advances.
//
// The files are build outputs, so they are not stored in Git. A deploy builds
// them from the captures. Run `pnpm presets:write` after changing a plan or a
// capture. Without the flag, only missing files are built.

import { execFileSync } from "node:child_process";
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

const here = dirname(fileURLToPath(import.meta.url));
const web = join(here, "..");
const root = join(web, "..");
const plansFile = join(web, "shared/presets.json");
const assets = join(web, "server/assets/presets");

const presets = JSON.parse(readFileSync(plansFile, "utf8"));
const write = process.argv.includes("--write");

mkdirSync(assets, { recursive: true });

const build = (capture, planFile, out, family) => {
  execFileSync(
    "cargo",
    [
      "run",
      "--quiet",
      "--release",
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
      "--plan",
      planFile,
      "--name",
      family,
    ],
    { stdio: ["ignore", "inherit", "inherit"] },
  );
};

for (const preset of presets) {
  const capture = join(
    web,
    "public",
    preset.plan.spacing === "quasi-proportional"
      ? "glyphs-quasi.bin"
      : "glyphs.bin",
  );
  const family = `Lemmata ${preset.label}`;
  const files = ["ttf", "woff2"].map((ext) =>
    join(assets, `${preset.id}.${ext}`),
  );

  if (!write && files.every((file) => existsSync(file))) {
    continue;
  }

  const scratch = mkdtempSync(join(tmpdir(), "lemmata-preset-"));
  const planFile = join(scratch, "plan.json");
  writeFileSync(planFile, JSON.stringify(preset.plan));
  for (const [index, file] of files.entries()) {
    build(capture, planFile, file, family);
    console.log(`preset ${preset.id}: ${index === 0 ? "ttf" : "woff2"}`);
  }
  rmSync(scratch, { recursive: true, force: true });
}
