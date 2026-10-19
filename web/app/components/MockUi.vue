<script setup lang="ts">
// The font doing a job rather than sitting in a line: a window, a heading, a
// paragraph, a row, a button and a line of code, all set in it.
//
// A specimen answers whether someone likes the letters. This answers whether they
// can read a page in them, which is the question a reader actually has.
import { useLemmataFont } from "~/composables/font";

useLemmataFont();
</script>

<template>
  <div class="mock">
    <div class="mock__bar">
      <i class="mock__dot" />
      <i class="mock__dot" />
      <i class="mock__dot" />
      <span>preview — the site, set in the site's font</span>
    </div>
    <div class="mock__body">
      <p class="mock__headline">Notes on a typeface factory</p>
      <p class="mock__line">
        A monospace cell is one width for every letter. The design is what happens
        inside the cell: how much room the drawing takes, and where the rest of it
        goes.
      </p>
      <div class="mock__code">let font = build(&glyphs, &metrics)?;</div>
      <div class="mock__row">
        <span>Weight 400 · Width normal · Slope upright</span>
        <span class="mock__button">Build</span>
      </div>
    </div>
  </div>
</template>
