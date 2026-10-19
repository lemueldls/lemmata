// Measures what a transpiler would have to read out of a definition file, by
// asking the compiler for each of its dumps and reporting their shape.
//
// The compiler is the reference for the frontend: PTL's surface is macros, and
// these dumps are what the reader, the expander and the lowering each produce.
//
// Nothing here prints a tree. The dumps carry the whole source on every node as
// `within`, which is both why they are enormous and why one of them cannot be
// stringified at all, so the script reports sizes and kinds and stops.
//
// Usage:
//   IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs <file.ptl>

import { execFileSync } from "node:child_process";

const buildDir = process.env.IOSEVKA_BUILD_DIR ?? "/tmp/opencode/iosevka-build";
const file = process.argv[2];
if (!file) {
  console.error("usage: node oracle.mjs <file.ptl>");
  process.exit(1);
}

const dump = (flag) => {
  try {
    return execFileSync(
      process.execPath,
      [`${buildDir}/node_modules/patel/bin/patel-c`, flag, file],
      { cwd: buildDir, maxBuffer: 1 << 30, stdio: ["ignore", "pipe", "pipe"] },
    ).toString("utf8");
  } catch (error) {
    // The dump can fail after writing, so keep whatever came out.
    return (error.stdout ?? Buffer.alloc(0)).toString("utf8");
  }
};

/// The slice of a dump that is the tree, which starts at the first `[{` or `["`
/// and runs to the last `]`.
const tree = (raw) => {
  const starts = [raw.indexOf("[{"), raw.indexOf('["')].filter((at) => at >= 0);
  if (starts.length === 0) return null;
  const from = Math.min(...starts);
  const to = raw.lastIndexOf("]") + 1;
  if (to <= from) return null;
  try {
    return JSON.parse(raw.slice(from, to));
  } catch {
    return null;
  }
};

/// Counts node kinds in an ESTree tree, ignoring the source metadata.
function kinds(node, counts) {
  if (Array.isArray(node)) {
    for (const item of node) kinds(item, counts);
    return;
  }
  if (!node || typeof node !== "object") return;
  if (typeof node.type === "string") {
    counts.set(node.type, (counts.get(node.type) ?? 0) + 1);
  }
  for (const [key, value] of Object.entries(node)) {
    if (key === "loc" || key === "within") continue;
    kinds(value, counts);
  }
}

const kib = (text) => `${Math.round(text.length / 1024)} KiB`;

console.log(`${file}`);
for (const [flag, what] of [
  ["--dump-ast", "the reader: S-expressions, macros unexpanded"],
  ["--dump-expanded", "after expansion: the macro-free core"],
  ["--dump-transformed", "after lowering: ESTree"],
]) {
  const raw = dump(flag);
  const parsed = tree(raw);
  const counts = new Map();
  if (parsed) kinds(parsed, counts);
  const top = [...counts.entries()]
    .sort((a, b) => b[1] - a[1])
    .slice(0, 12)
    .map(([kind, count]) => `${kind} ${count}`)
    .join(", ");
  console.log(
    `  ${flag.padEnd(19)} ${kib(raw).padStart(8)}  ${parsed ? "parses" : "does not serialise"}  ${what}\n      ${top}`,
  );
}
