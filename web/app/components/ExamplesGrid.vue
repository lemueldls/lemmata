<script setup lang="ts">
// One card per kind of text a reader might set: code, prose, figures, a
// terminal, an address, a table.
//
// Every sample is written in the characters the capture holds, which is the
// letters, the digits and some punctuation, so nothing here falls back to
// another face without saying so.
import { useLemmataFont } from "~/composables/font";

useLemmataFont();

const EXAMPLES = [
  {
    kind: "code",
    text: `for (let i = 0; i < 10; i += 1) {
  total = total + i;
}`,
  },
  {
    kind: "prose",
    text: `the quick brown fox jumps
over the lazy dog, and
then it does it again`,
  },
  {
    kind: "figures",
    text: `0123456789
1,234.56 -7.8% 3/4
U+0061 · U+002E · U+2042`,
  },
  {
    kind: "terminal",
    text: `$ make -j 8 all
built 3 files in 1.2s
$ echo "done" && exit 0`,
  },
  {
    kind: "address",
    text: `  Ordinal   Glyph
  -------   -----
     1st      U+0031
     2nd      U+0032`,
  },
  {
    kind: "maths",
    text: `x = (a + b) / (c - d)
f(x) = 2 * x + 1
p ^ q => not p or q`,
  },
];
</script>

<template>
  <div class="examples">
    <div v-for="example in EXAMPLES" :key="example.kind" class="example">
      <div class="example__head">{{ example.kind }}</div>
      <pre class="example__body">{{ example.text }}</pre>
    </div>
  </div>
</template>
