<script setup lang="ts">
// The cover the landing wears while the font arrives.
//
// The font is a webfont, so it comes after the first paint. Until it is here the
// page would be set in the fallback and the wordmark could not be measured, so
// the cover stays up, with no words on it, and leaves once the readings are
// ready. It is in the prerendered HTML, so it is painted first rather than
// appearing after hydration.
//
// A failed load and a slow one both end it, because a cover that never leaves is
// worse than a fallback face.
import { useLemmataFont } from "~/composables/font";

const font = useLemmataFont();
const fading = ref(false);
const gone = ref(false);

/// Leaves, after the fade, so the element can be taken out of the layout.
function leave(): void {
  if (gone.value) return;
  fading.value = true;
  setTimeout(() => {
    gone.value = true;
  }, 300);
}

watch(
  () => font.ready || font.error !== "",
  (done) => {
    if (done) leave();
  },
  { immediate: true },
);

onMounted(() => setTimeout(leave, 8000));
</script>

<template>
  <div
    v-show="!gone"
    class="boot"
    :class="{ 'boot--gone': fading }"
    aria-hidden="true"
  >
    <div class="boot__meter">
      <span class="boot__ticks" />
      <span class="boot__track"><i /></span>
    </div>
  </div>
  <noscript>
    <style>
      .boot {
        display: none;
      }
    </style>
  </noscript>
</template>
