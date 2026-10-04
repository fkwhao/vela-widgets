<script setup lang="ts">
import {
  SliderRange,
  SliderRoot,
  SliderThumb,
  SliderTrack,
} from "reka-ui";

defineProps<{
  modelValue: number;
  min: number;
  max: number;
  step: number;
  label: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: number];
  valueCommit: [value: number];
}>();

function onValueChange(values: number[] | undefined): void {
  if (values?.length) emit("update:modelValue", values[0]);
}

function onValueCommit(values: number[]): void {
  emit("valueCommit", values[0]);
}
</script>

<template>
  <SliderRoot
    class="vela-slider-root"
    :model-value="[modelValue]"
    :min="min"
    :max="max"
    :step="step"
    :aria-label="label"
    @update:model-value="onValueChange"
    @value-commit="onValueCommit"
  >
    <SliderTrack class="vela-slider-track">
      <SliderRange class="vela-slider-range" />
    </SliderTrack>
    <SliderThumb class="vela-slider-thumb" :aria-label="label" />
  </SliderRoot>
</template>
