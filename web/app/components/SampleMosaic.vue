<script setup lang="ts">
// A wall of samples, the way a code face is usually shown: one card per kind of
// text, different widths, all in the same face.
//
// The sources stay inside the capture's coverage. That is why the code is
// lowercase Python, the table has no rules and the config is a column of pairs
// rather than JSON. A bracket or a capital would hand the line to a fallback.
interface Sample {
  kind: string;
  category: "code" | "text" | "data";
  lang: string | null;
  source: string;
}

interface Block extends Sample {
  html: string | null;
}

const SAMPLES: Sample[] = [
  {
    kind: "python",
    category: "code",
    lang: "python",
    source: `# bisect a span until it is small
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
    kind: "shell",
    category: "code",
    lang: "shellscript",
    source: `$ lemmata build --plan plan.json
wrote lemmata.ttf
127 glyphs / 18104 bytes / 41 ms
$ open lemmata.ttf`,
  },
  {
    kind: "prose",
    category: "text",
    lang: null,
    source: `the quick brown fox jumps over
the lazy dog, then turns around
and does the whole thing again`,
  },
  {
    kind: "table",
    category: "data",
    lang: null,
    source: `part      qty   price
------    ---   -----
bolt       12    3.40
nut         8    1.25
cog        40    0.75`,
  },
  {
    kind: "diff",
    category: "code",
    lang: "diff",
    source: `@@ cap height @@
-735
+748
@@ x height @@
-520
+536`,
  },
  {
    kind: "config",
    category: "data",
    lang: null,
    source: `weight      400
spacing     normal
capHeight   735
xHeight     520
ascender    965
descender   -215`,
  },
  {
    kind: "editorial",
    category: "text",
    lang: null,
    source: `a typeface is a set of decisions.
the page is where they show, line
after line, at the size you chose.`,
  },
  {
    kind: "metrics",
    category: "data",
    lang: null,
    source: `glyphs      127
characters  59
units/em    1000
advance     500
math        yes`,
  },
  {
    kind: "lean",
    category: "code",
    lang: "lean4",
    source: `example : 2 + 3 = 5 := by decide

example : 6 * 7 = 42 := by
  have h : 6 * 7 = 42 := by decide
  exact h`,
  },
  {
    kind: "alphabet",
    category: "text",
    lang: null,
    source: `abcdefghijklmnopqrstuvwxyz
0123456789
the quick brown fox`,
  },
  {
    kind: "interface",
    category: "text",
    lang: null,
    source: `save changes
build again
your file is ready`,
  },
];

const FILTERS = [
  { id: "all", label: "all" },
  { id: "code", label: "code" },
  { id: "text", label: "text" },
  { id: "data", label: "data" },
] as const;

const filter = ref<(typeof FILTERS)[number]["id"]>("all");

const { data: blocks } = await useAsyncData<Block[]>(
  "sample-mosaic",
  async () => {
    if (!import.meta.client) {
      const { highlight } = await import("~/utils/highlight");

      return Promise.all(
        SAMPLES.map(async (sample) => ({
          ...sample,
          html: sample.lang ? await highlight(sample.source, sample.lang) : null,
        })),
      );
    }

    return SAMPLES.map((sample) => ({ ...sample, html: null }));
  },
  { default: () => SAMPLES.map((sample) => ({ ...sample, html: null })) },
);

const shown = computed(() =>
  (blocks.value ?? []).filter(
    (sample) => filter.value === "all" || sample.category === filter.value,
  ),
);

function chars(source: string): number {
  return source.replace(/\s/g, "").length;
}
</script>

<template>
  <div class="sample-filter" role="group" aria-label="Sample kind">
    <button
      v-for="option in FILTERS"
      :key="option.id"
      type="button"
      :aria-pressed="filter === option.id"
      @click="filter = option.id"
    >
      {{ option.label }}
    </button>
  </div>

  <div class="mosaic">
    <article
      v-for="block in shown"
      :key="block.kind"
      class="sample"
    >
      <header class="sample__head">
        <span class="sample__kind">{{ block.kind }}</span>
        <span>{{ block.lang ?? block.category }}</span>
      </header>

      <div v-if="block.html" class="sample__body" v-html="block.html" />
      <pre v-else class="sample__body">{{ block.source }}</pre>

      <footer class="sample__foot">
        <span>{{ block.source.split("\n").length }} lines</span>
        <span>{{ chars(block.source) }} characters</span>
      </footer>
    </article>
  </div>
</template>
