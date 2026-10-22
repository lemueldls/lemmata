<script setup lang="ts">
// The CJK sample: a small prebuilt subset, and the 2:1 grid it is built on.
//
// The full face is a native build. This page shows what the grid looks like and
// points at the release, rather than asking the browser to read a 14 MB source.
useHead({ title: "CJK" });

const config = useRuntimeConfig();
const r2 = computed(() => String(config.public.cjkBase ?? "").replace(/\/$/, ""));

const sample = "/fonts/lemmata-cjk-sample.woff2";
const release = "https://github.com/lemueldls/lemmata/releases/latest";

const rows = [
  { latin: "abcdefghij", cjk: "一二三四五" },
  { latin: "0123456789", cjk: "六七八九十" },
  { latin: "code 2026", cjk: "中文排版" },
];
</script>

<template>
  <div class="wrap page">
    <header class="page-head">
      <div>
        <p class="eyebrow"><b>+</b> cjk <i>/</i> 04</p>
        <h1 class="page-head__title">Two cells, <em>one glyph.</em></h1>
        <p class="lede" style="margin-top: 18px">
          A full-width CJK glyph takes exactly twice the Latin cell. That is the
          grid Sarasa Mono already has, and the grid this build keeps when the
          Latin cell is 600 units.
        </p>
      </div>

      <div class="page-head__side">
        <p class="mono-note">
          the sample below is a small subset. the full face is built natively and
          published as a download.
        </p>
        <dl class="page-head__facts">
          <div><dt>source</dt><dd>Sarasa Mono SC</dd></div>
          <div><dt>CJK cell</dt><dd>2 × Latin</dd></div>
          <div><dt>locale</dt><dd>CN</dd></div>
          <div><dt>sample</dt><dd>50 glyphs</dd></div>
        </dl>
      </div>
    </header>

    <PlusPlate index="01" label="grid" meta="2:1">
      <div class="section-intro section-intro--single">
        <div>
          <p class="eyebrow"><b>+</b> the cell</p>
          <h2 class="display" style="font-size: var(--step-3)">
            Ten cells, five characters.
          </h2>
        </div>
        <p class="prose">
          Each row is ten Latin cells wide. The CJK row holds five characters,
          because each one is two cells. The sample face is subset from the same
          build, so the widths on this page are the widths in the download.
        </p>
      </div>

      <div class="cjk-grid" aria-label="Latin and CJK at a two to one width">
        <div v-for="row in rows" :key="row.latin" class="cjk-grid__row">
          <span class="cjk-grid__label">latin</span>
          <span class="cjk-grid__cells">{{ row.latin }}</span>
          <span class="cjk-grid__label">cjk</span>
          <span class="cjk-grid__cells">{{ row.cjk }}</span>
        </div>
      </div>

      <table class="cjk-table">
        <thead>
          <tr><th>text</th><th>advance</th><th>cells</th></tr>
        </thead>
        <tbody>
          <tr><td><code>a</code></td><td>600</td><td>1</td></tr>
          <tr><td><code>一</code></td><td>1200</td><td>2</td></tr>
          <tr><td><code>中文</code></td><td>2400</td><td>4</td></tr>
          <tr><td><code>，</code></td><td>1200</td><td>2</td></tr>
        </tbody>
      </table>
    </PlusPlate>

    <PlusPlate index="02" label="download" meta="sample and release">
      <div class="section-intro section-intro--single">
        <div>
          <p class="eyebrow"><b>+</b> the files</p>
          <h2 class="display" style="font-size: var(--step-3)">
            Take the sample, or the face.
          </h2>
        </div>
        <p class="prose">
          The sample carries the characters on this page. The full CN face is
          built with <code>lemmata build --cjk</code> or a family document, and
          ships as static TTF and WOFF2 with the OFL notice.
        </p>
      </div>

      <div class="cjk-actions">
        <a class="cjk-action" :href="sample" download>
          <span>sample</span>
          <b>WOFF2, 50 glyphs</b>
        </a>
        <a class="cjk-action" :href="release">
          <span>release</span>
          <b>full CJK faces</b>
        </a>
        <a v-if="r2" class="cjk-action" :href="r2">
          <span>mirror</span>
          <b>R2</b>
        </a>
      </div>

      <p class="prose cjk-note">
        The CJK glyphs are from Sarasa Gothic, which draws its CJK from Source Han
        Sans. The sample's notice is beside the file. A generated face must use a
        new family name, carry the OFL text and keep the source notices.
      </p>
    </PlusPlate>
  </div>
</template>

<style scoped>
.cjk-grid {
  font-family: "Lemmata CJK Sample", var(--mono);
  font-size: clamp(18px, 3vw, 30px);
  line-height: 1.5;
  margin: 28px 0;
  border-top: 1px solid var(--line);
  border-bottom: 1px solid var(--line);
}

.cjk-grid__row {
  display: grid;
  grid-template-columns: 3.5rem 10ch 3.5rem 10ch;
  align-items: baseline;
  gap: 0.75rem;
  padding: 0.55rem 0;
  border-bottom: 1px solid var(--line);
}

.cjk-grid__row:last-child {
  border-bottom: 0;
}

.cjk-grid__label {
  font-family: var(--mono);
  font-size: 0.55em;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--dim);
}

.cjk-grid__cells {
  display: inline-block;
  white-space: nowrap;
  border-left: 1px solid var(--line);
  border-right: 1px solid var(--line);
  padding: 0 0.15em;
}

.cjk-table {
  width: 100%;
  border-collapse: collapse;
  margin-top: 20px;
  font-family: var(--mono);
  font-size: 0.82rem;
}

.cjk-table th,
.cjk-table td {
  text-align: left;
  padding: 0.45rem 0.6rem;
  border-bottom: 1px solid var(--line);
}

.cjk-table th {
  color: var(--dim);
  font-weight: 500;
}

.cjk-actions {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 12px;
  margin: 24px 0;
}

.cjk-action {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 16px;
  border: 1px solid var(--line);
  color: inherit;
  text-decoration: none;
}

.cjk-action:hover {
  border-color: var(--ink);
}

.cjk-action span {
  font-family: var(--mono);
  font-size: 0.65rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--dim);
}

.cjk-action b {
  font-weight: 500;
}

.cjk-note {
  font-size: 0.85rem;
}
</style>
