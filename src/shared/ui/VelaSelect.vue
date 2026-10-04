<script setup lang="ts">
import {
  SelectContent,
  SelectItem,
  SelectItemIndicator,
  SelectItemText,
  SelectPortal,
  SelectRoot,
  SelectTrigger,
  SelectValue,
  SelectViewport,
} from "reka-ui";
import AppIcon from "./AppIcon.vue";

type SelectOption = { value: string; label: string };

defineProps<{
  modelValue: string;
  label: string;
  options: SelectOption[];
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

function onValueChange(value: unknown): void {
  emit("update:modelValue", String(value));
}
</script>

<template>
  <SelectRoot :model-value="modelValue" @update:model-value="onValueChange">
    <SelectTrigger class="vela-select-trigger" :aria-label="label">
      <SelectValue :aria-label="label" />
      <AppIcon class="vela-select-chevron" name="chevron-down" :size="14" />
    </SelectTrigger>
    <SelectPortal>
      <SelectContent class="vela-select-content" position="popper" :side-offset="5" align="end">
        <SelectViewport class="vela-select-viewport">
          <SelectItem v-for="option in options" :key="option.value" class="vela-select-item" :value="option.value">
            <SelectItemText>{{ option.label }}</SelectItemText>
            <SelectItemIndicator class="vela-select-item-indicator">✓</SelectItemIndicator>
          </SelectItem>
        </SelectViewport>
      </SelectContent>
    </SelectPortal>
  </SelectRoot>
</template>
