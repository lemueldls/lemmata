// Runs upstream's numeric helpers over a grid of inputs and prints the results,
// so the Rust port of `packages/util` can be compared against them.
//
// Usage:
//   IOSEVKA_BUILD_DIR=/tmp/opencode/iosevka-build node oracle.mjs \
//     > ../../golden/util.json

import { pathToFileURL } from "node:url";

const buildDir = process.env.IOSEVKA_BUILD_DIR ?? "/tmp/opencode/iosevka-build";
const util = await import(pathToFileURL(`${buildDir}/packages/util/src/index.mjs`).href);

const cases = [];
const add = (name, args, result) => cases.push({ name, args, result });

for (const a of [0, 100, -50]) {
  for (const b of [100, 300]) {
    for (const p of [0, 0.25, 0.5, 1, 1.5, -0.5]) {
      add("mix", [a, b, p], util.mix(a, b, p));
    }
  }
}

for (const [l, r, b, p] of [
  [0, 100, 20, 0.5],
  [100, 0, 20, 0.5],
  [-50, 50, 10, 0.25],
  [0, 0, 0, 1],
  [10, 10, 4, 0.75],
]) {
  add("barMixL", [l, r, b, p], util.barMixL(l, r, b, p));
}

for (const args of [
  [0, 0, 100, 50, 25],
  [0, 0, 100, 50, 150],
  [-20, 5, 80, -30, 0],
  [0, 0, 1, 1, 0.5],
]) {
  add("linreg", args, util.linreg(...args));
}

for (const args of [
  [0, 1, 0.5],
  [0, 1, -1],
  [0, 1, 2],
  [-5, 5, 0],
]) {
  add("clamp", args, util.clamp(...args));
}

for (const args of [
  [0.5, 1],
  [1.5, 1],
  [-0.5, 1],
  [2.5, 1],
  [-1.5, 1],
  [7, 3],
  [0.24, 0.1],
  [0.25, 0.1],
  [-0.25, 0.1],
]) {
  add("quantize", args, util.quantize(...args));
}

for (const args of [
  ["undefined", 5],
  [7, 5],
  ["undefined", "undefined", 3],
  ["undefined", "undefined"],
]) {
  const jsArgs = args.map((value) => (value === "undefined" ? undefined : value));
  add("fallback", args, util.fallback(...jsArgs));
}

for (const args of [
  [0, 50, 100, 0],
  [0, 50, 100, 0.25],
  [0, 50, 100, 1],
  [0, 50, 100, 1.5],
]) {
  add("bez2", args, util.bez2(...args));
}

for (const args of [
  [0, 30, 70, 100, 0],
  [0, 30, 70, 100, 0.25],
  [0, 30, 70, 100, 0.5],
  [0, 30, 70, 100, 1],
]) {
  add("bez3", args, util.bez3(...args));
}

for (const value of [true, false]) {
  add("boole", [value], util.boole(value));
  add("boolePn", [value], util.boolePn(value));
}

for (const args of [
  [0, 0, 100, 0, 20, 1],
  [0, 0, 0, 100, 20, 1],
  [10, 20, 30, 40, 15, 1.2],
]) {
  const result = util.strokeOffset(...args);
  add("strokeOffset", args, [result.x, result.y]);
}

for (const args of [
  [0, 0, 100, 0.5],
  [0, 10, -50, -0.25],
]) {
  add("slY", args, util.slY(...args));
  add("slX", args, util.slX(...args));
}

for (const args of [
  [0, 0, 100, 0, 25],
  [0, 0, 0, 100, 25],
  [10, 10, 40, 50, 10],
]) {
  add("distP", args, util.distP(...args));
}

for (const kind of ["sine", "square", "triangle", "sawtooth"]) {
  for (const x of [-2, -1, -0.5, 0, 0.5, 1, 2, 3, 4, 5]) {
    add(`waveform.${kind}`, [x], util.Waveform[kind](x));
  }
}

process.stdout.write(JSON.stringify({ oracle: "util", cases }, null, 2) + "\n");
