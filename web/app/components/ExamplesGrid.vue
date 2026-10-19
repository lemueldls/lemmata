<script setup lang="ts">
// One card per kind of text a reader might set.
//
// Every sample is written in the characters the capture holds: the lowercase
// letters, the digits, and the punctuation around them. That is why the source
// sample is Python and the proof is Lean rather than, say, C or Rocq, and why
// the proof is arithmetic with no type names in it. An uppercase letter or a
// bracket would borrow another face, and the point of the section is that
// nothing here does.
//
// The samples are highlighted when the page is built and the markup is carried
// in the payload, so the browser gets it as HTML and never loads a highlighter.
interface Sample {
  kind: string;
  lang: string | null;
  source: string;
}

interface Block extends Sample {
  html: string | null;
}

const SAMPLES: Sample[] = [
  {
    kind: "source",
    lang: "python",
    source: `# bisect, until the span is small
def root(f, a, b, eps):
    while b - a > eps:
        m = (a + b) / 2.0
        if f(m) < 0.0:
            a = m
        else:
            b = m
    return (a + b) / 2.0`,
  },
  {
    kind: "proof",
    lang: "lean4",
    source: `example : 2 + 3 = 5 := by decide

example : 6 * 7 = 42 := by
  have h : 6 * 7 = 42 := by decide
  exact h`,
  },
  {
    kind: "prose",
    lang: null,
    source: `the quick brown fox jumps
over the lazy dog, and
then it does it again`,
  },
  {
    kind: "figures",
    lang: null,
    source: `0123456789
1,234.56 -7.8% 3/4
1e6 0.5% 2 + 2`,
  },
  {
    kind: "terminal",
    lang: "shellscript",
    source: `$ make -j 8 all
built 3 files in 1.2s
$ echo "done" && exit 0`,
  },
  {
    kind: "table",
    lang: null,
    source: `  part    qty   price
  ------  ---  ------
  bolt     12   3.40
  nut       8   1.25
  cog      40   0.75`,
  },
  {
    kind: "formulas",
    lang: null,
    source: `x = (a + b) / (c - d)
f(x) = 2 * x + 1
p & q  =>  not p or q`,
  },
];

const { data: blocks } = await useAsyncData<Block[]>(
  "examples",
  async () => {
    if (import.meta.server) {
      const { highlight } = await import("~/utils/highlight");

      return Promise.all(
        SAMPLES.map(async (sample) => ({
          ...sample,
          html: sample.lang ? await highlight(sample.source, sample.lang) : null,
        })),
      );
    }

    // Only reached if the payload did not arrive. Uncoloured code still reads.
    return SAMPLES.map((sample) => ({ ...sample, html: null }));
  },
  { default: () => SAMPLES.map((sample) => ({ ...sample, html: null })) },
);
</script>

<template>
  <div class="examples">
    <div v-for="block in blocks" :key="block.kind" class="example">
      <div class="example__head">{{ block.kind }}</div>
      <div v-if="block.html" class="example__body" v-html="block.html" />
      <pre v-else class="example__body">{{ block.source }}</pre>
    </div>
  </div>
</template>
