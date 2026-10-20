<script setup lang="ts">
// A slider with the same label row as the other controls.
//
// Reka's slider is mounted after hydration. The static page keeps a native
// range with the same value and bounds, so the control works before the client
// bundle arrives and the prerender never touches the component's browser
// measurement.
import {
  SliderRange,
  SliderRoot,
  SliderThumb,
  SliderTrack,
} from "reka-ui";

withDefaults(
  defineProps<{
    modelValue: number;
    min?: number;
    max?: number;
    step?: number;
    label?: string;
    unit?: string;
  }>(),
  { min: 0, max: 100, step: 1, label: "", unit: "" },
);

const emit = defineEmits<{ "update:modelValue": [value: number] }>();

function update(value: number[] | undefined): void {
  const next = value?.[0];
  if (next !== undefined) emit("update:modelValue", next);
}

function input(event: Event): void {
  const target = event.target;
  if (target instanceof HTMLInputElement) {
    emit("update:modelValue", Number(target.value));
  }
}
</script>

<template>
  <div class="range-control">
    <div v-if="label" class="range-control__label">
      <span>{{ label }}</span>
      <b>{{ modelValue }}{{ unit ? ` ${unit}` : "" }}</b>
    </div>

    <ClientOnly>
      <SliderRoot
        class="range-control__root"
        :model-value="[modelValue]"
        :min="min"
        :max="max"
        :step="step"
        @update:model-value="update"
      >
        <SliderTrack class="range-control__track">
          <SliderRange class="range-control__range" />
        </SliderTrack>
        <SliderThumb class="range-control__thumb" :aria-label="label || 'value'" />
      </SliderRoot>

      <template #fallback>
        <input
          class="range-control__fallback"
          type="range"
          :value="modelValue"
          :min="min"
          :max="max"
          :step="step"
          :aria-label="label || 'value'"
          @input="input"
        />
      </template>
    </ClientOnly>
  </div>
</template>
