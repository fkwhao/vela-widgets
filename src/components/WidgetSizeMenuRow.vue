<script setup lang="ts">
import AppIcon from "./AppIcon.vue";
import { widgetSizeOptions, type WidgetSize } from "../types";

defineProps<{ size: WidgetSize; disabled?: boolean }>();
defineEmits<{ choose: [size: WidgetSize] }>();
</script>

<template>
  <div class="context-size-row" role="group" aria-label="组件尺寸">
    <span class="context-size-label"><AppIcon name="grid" :size="14" />尺寸</span>
    <div class="context-size-options">
      <button
        v-for="option in widgetSizeOptions"
        :key="option.value"
        :class="{ active: size === option.value }"
        role="menuitemradio"
        :aria-checked="size === option.value"
        :aria-label="`${option.label}尺寸`"
        :disabled="disabled"
        @click="$emit('choose', option.value)"
      >{{ option.label }}</button>
    </div>
  </div>
</template>
