<script setup lang="ts">
import { computed, ref, shallowRef } from "vue";
import {
  PopoverContent,
  PopoverPortal,
  PopoverRoot,
  PopoverTrigger,
} from "reka-ui";
import AppIcon from "./AppIcon.vue";
import VelaCalendar from "./VelaCalendar.vue";

// Replaces <input type="date">: a field-like button that opens a Fluent calendar.
// Widget flyouts stay within the small window bounds; the manager uses the same calendar.
defineOptions({ inheritAttrs: false });
const props = withDefaults(defineProps<{ modelValue: string; label: string; placeholder?: string; clearable?: boolean; min?: string; max?: string; variant?: "widget" | "manager" }>(), { placeholder: "选择日期", variant: "widget" });
const emit = defineEmits<{ "update:modelValue": [value: string] }>();
const open = ref(false);
const compact = ref(false);
const boundary = shallowRef<Element | undefined>();
const pickerWidth = ref("244px");
const pickerRoot=ref<{ $el?:HTMLElement }>();
function setOpen(next: boolean) {
  if (next && props.variant === 'widget') {
    boundary.value = pickerRoot.value?.$el?.closest('.widget-window') ?? undefined;
    const bounds = boundary.value?.getBoundingClientRect();
    compact.value = (bounds?.height ?? window.innerHeight) < 240;
    pickerWidth.value = `${Math.min(compact.value ? 208 : 244, (bounds?.width ?? window.innerWidth) - 12)}px`;
  }
  open.value = next;
}
const display = computed(() => {
  if (!props.modelValue) return "";
  const date = new Date(`${props.modelValue}T12:00:00`);
  if (Number.isNaN(date.getTime())) return props.modelValue;
  return `${date.getFullYear()}/${String(date.getMonth()+1).padStart(2,'0')}/${String(date.getDate()).padStart(2,'0')}`;
});
function choose(value: string) { emit("update:modelValue", value); open.value = false; }
function clear() { emit("update:modelValue", ""); open.value = false; }
</script>

<template>
  <PopoverRoot :open="open" @update:open="setOpen">
    <PopoverTrigger ref="pickerRoot" v-bind="$attrs" type="button" class="vela-date-field" :class="{ manager: variant === 'manager', empty: !modelValue }" :aria-label="`${label}：${display || '未设置'}`">
      <span>{{ display || placeholder }}</span><AppIcon name="calendar" :size="14" />
    </PopoverTrigger>
    <PopoverPortal>
      <PopoverContent class="vela-date-flyout" :class="{ 'widget-picker': variant === 'widget', compact: variant === 'widget' && compact }" data-vela-picker side="bottom" align="start" :side-offset="5" :collision-padding="6" :collision-boundary="boundary" :prioritize-position="true" sticky="always" :style="variant === 'widget' ? { width: pickerWidth } : undefined" :aria-label="label" @escape-key-down.stop>
        <VelaCalendar :model-value="modelValue" :min="min" :max="max" :clearable="clearable" :compact="variant === 'widget' && compact" @select="choose" @clear="clear" />
      </PopoverContent>
    </PopoverPortal>
  </PopoverRoot>
</template>
