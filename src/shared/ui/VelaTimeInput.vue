<script setup lang="ts">
import { computed, ref, onUnmounted } from "vue";
import AppIcon from "./AppIcon.vue";

// Replaces <input type="time"> and the h/m/s number boxes with Windows Clock style
// digit segments: arrows, wheel, Page Up/Down and typed digits all adjust a segment.
type Part = "h" | "m" | "s";
const props = withDefaults(defineProps<{ modelValue: number; seconds?: boolean; duration?: boolean; label: string; compact?: boolean }>(), { seconds: false, duration: false, compact: false });
const emit = defineEmits<{ "update:modelValue": [value: number] }>();
const parts = computed<Part[]>(() => props.seconds ? ["h", "m", "s"] : ["h", "m"]);
const names: Record<Part, string> = { h: "时", m: "分", s: "秒" };
const values = computed(() => ({ h: Math.floor(props.modelValue / 3600), m: Math.floor(props.modelValue / 60) % 60, s: props.modelValue % 60 }));
const segments = ref<HTMLElement[]>([]);
let typed = "";
let typedTimer: ReturnType<typeof setTimeout> | undefined;
onUnmounted(() => { if (typedTimer) clearTimeout(typedTimer); });
function limit(part: Part) { return part === "h" ? (props.duration ? 24 : 23) : 59; }
function write(next: Record<Part, number>) {
  let total = next.h * 3600 + next.m * 60 + next.s;
  if (props.duration) total = Math.min(total, 24 * 3600);
  emit("update:modelValue", total);
}
function set(part: Part, value: number) { write({ ...values.value, [part]: value }); }
function step(part: Part, delta: number) {
  const max = limit(part), size = max + 1;
  set(part, ((values.value[part] + delta) % size + size) % size);
}
function focusPart(index: number) { segments.value[Math.max(0, Math.min(parts.value.length - 1, index))]?.focus(); }
function onKey(event: KeyboardEvent, part: Part, index: number) {
  const keys: Record<string, () => void> = {
    ArrowUp: () => step(part, 1), ArrowDown: () => step(part, -1),
    PageUp: () => step(part, part === "h" ? 6 : 10), PageDown: () => step(part, part === "h" ? -6 : -10),
    Home: () => set(part, 0), End: () => set(part, limit(part)),
    ArrowLeft: () => focusPart(index - 1), ArrowRight: () => focusPart(index + 1),
    Backspace: () => set(part, 0), Delete: () => set(part, 0),
    Enter: () => (event.currentTarget as HTMLElement).closest("form")?.requestSubmit(),
  };
  if (keys[event.key]) { event.preventDefault(); typed = ""; keys[event.key](); return; }
  if (!/^\d$/.test(event.key)) return;
  event.preventDefault();
  // Two typed digits fill a segment; a first digit that cannot start a valid pair moves on at once.
  const next = typed + event.key, max = limit(part);
  if (Number(next) > max) typed = event.key; else typed = next;
  set(part, Math.min(Number(typed), max));
  if (typedTimer) clearTimeout(typedTimer);
  if (typed.length === 2 || Number(typed) * 10 > max) { typed = ""; focusPart(index + 1); }
  else typedTimer = setTimeout(() => (typed = ""), 1000);
}
function onWheel(event: WheelEvent, part: Part) { event.preventDefault(); step(part, event.deltaY < 0 ? 1 : -1); }
</script>

<template>
  <div class="vela-time-input" :class="{ compact }" role="group" :aria-label="label">
    <template v-for="(part, index) in parts" :key="part">
      <span v-if="index" class="vela-time-colon" aria-hidden="true">:</span>
      <div class="vela-time-part">
        <button type="button" class="vela-time-step" tabindex="-1" :aria-label="`增加${names[part]}`" @pointerdown.prevent @click="step(part, 1); focusPart(index)"><AppIcon name="chevron-up" :size="12" /></button>
        <div :ref="el => { if (el) segments[index] = el as HTMLElement; }" class="vela-time-segment" role="spinbutton" tabindex="0" :aria-label="`${label}${names[part]}`" :aria-valuenow="values[part]" aria-valuemin="0" :aria-valuemax="limit(part)" :aria-valuetext="`${values[part]} ${names[part]}`" @keydown="onKey($event, part, index)" @wheel="onWheel($event, part)" @blur="typed = ''">{{ String(values[part]).padStart(2, "0") }}</div>
        <button type="button" class="vela-time-step" tabindex="-1" :aria-label="`减少${names[part]}`" @pointerdown.prevent @click="step(part, -1); focusPart(index)"><AppIcon name="chevron-down" :size="12" /></button>
      </div>
    </template>
  </div>
</template>
