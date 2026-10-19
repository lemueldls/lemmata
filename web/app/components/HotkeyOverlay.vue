<script setup lang="ts">
// The key map. It is the same list the listener answers, written once here so a
// reader can see the whole surface without guessing.
import { helpOpen } from "~/composables/hotkeys";
import { useRoute } from "vue-router";

const route = useRoute();

watch(
  () => route.fullPath,
  () => {
    helpOpen.value = false;
  },
);

const GROUPS = [
  {
    title: "move",
    rows: [
      { keys: ["↑", "↓"], label: "previous / next block" },
      { keys: ["1", "…", "9"], label: "jump to a block" },
      { keys: ["j", "k"], label: "scroll by a line" },
      { keys: ["w", "s"], label: "scroll by a page" },
    ],
  },
  {
    title: "view",
    rows: [
      { keys: ["t", "m"], label: "cycle the palette" },
      { keys: ["+", "-"], label: "scale the specimen" },
      { keys: ["r"], label: "reset the specimen size" },
      { keys: ["?"], label: "open or close this map" },
    ],
  },
  {
    title: "go",
    rows: [
      { keys: ["i"], label: "specimen" },
      { keys: ["g"], label: "configurator" },
      { keys: ["e"], label: "equations" },
      { keys: ["b"], label: "build the plan" },
      { keys: ["d"], label: "download the font" },
    ],
  },
];

const close = ref<HTMLButtonElement | null>(null);

watch(helpOpen, async (open) => {
  if (!import.meta.client) return;
  document.body.style.overflow = open ? "hidden" : "";
  if (open) {
    await nextTick();
    close.value?.focus();
  }
});

onBeforeUnmount(() => {
  if (import.meta.client) document.body.style.overflow = "";
});
</script>

<template>
  <Transition name="fade">
    <div
      v-if="helpOpen"
      class="keymap"
      role="dialog"
      aria-modal="true"
      aria-labelledby="keymap-title"
      @click.self="helpOpen = false"
    >
      <div class="keymap__panel">
        <header class="keymap__head">
          <b>+</b>
          <span id="keymap-title">key map</span>
          <span class="muted">every key, one page</span>
          <button ref="close" type="button" @click="helpOpen = false">
            esc to close
          </button>
        </header>

        <div class="keymap__grid">
          <section v-for="group in GROUPS" :key="group.title" class="keymap__group">
            <h3>{{ group.title }}</h3>
            <dl>
              <div v-for="row in group.rows" :key="row.label" class="keymap__row">
                <dt>
                  <kbd v-for="key in row.keys" :key="key">{{ key }}</kbd>
                </dt>
                <dd>{{ row.label }}</dd>
              </div>
            </dl>
          </section>
        </div>

        <footer class="keymap__foot">
          <span><kbd>esc</kbd> closes this map and leaves a field</span>
          <span>keys work from any page</span>
        </footer>
      </div>
    </div>
  </Transition>
</template>

<style>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 140ms linear;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
