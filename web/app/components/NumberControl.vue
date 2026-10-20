<script setup lang="ts">
// A number field with its own steppers, so the metric fields share one shape.
import {
  NumberFieldDecrement,
  NumberFieldIncrement,
  NumberFieldInput,
  NumberFieldRoot,
} from "reka-ui";

withDefaults(
  defineProps<{
    modelValue: number;
    min?: number;
    max?: number;
    step?: number;
  }>(),
  { step: 1 },
);

const emit = defineEmits<{ "update:modelValue": [value: number] }>();

function update(value: number | null | undefined): void {
  emit("update:modelValue", value ?? 0);
}
</script>

<template>
  <NumberFieldRoot
    class="number-control"
    :model-value="modelValue"
    :min="min"
    :max="max"
    :step="step"
    :format-options="{ useGrouping: false }"
    @update:model-value="update"
  >
    <NumberFieldInput class="number-control__input" />
    <span class="number-control__steps">
      <NumberFieldIncrement class="number-control__step">+</NumberFieldIncrement>
      <NumberFieldDecrement class="number-control__step">−</NumberFieldDecrement>
    </span>
  </NumberFieldRoot>
</template>
