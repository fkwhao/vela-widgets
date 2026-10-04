<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { PopoverContent, PopoverPortal, PopoverRoot, PopoverTrigger } from 'reka-ui';
import AppIcon from './AppIcon.vue';
import { clamp, hexToHsv, hexToRgb, hsvToHex, normalizeHex, rgbToHex } from '../color';

defineOptions({ inheritAttrs: false });
const props = withDefaults(defineProps<{ modelValue: string; label: string; disabled?: boolean; variant?: 'swatch' | 'round' | 'option' | 'field'; selected?: boolean }>(), { variant: 'swatch' });
const emit = defineEmits<{ 'update:modelValue': [value: string] }>();
const open = ref(false);
const hsv = ref(hexToHsv(normalizeHex(props.modelValue) ?? '#3b67b8'));
const hexInput = ref(props.modelValue);
const draft = computed(() => hsvToHex(hsv.value));
const rgb = computed(() => hexToRgb(draft.value));
const validHex = computed(() => normalizeHex(hexInput.value));
const changed = computed(() => draft.value !== normalizeHex(props.modelValue));
const hueColor = computed(() => hsvToHex({ h: hsv.value.h, s: 100, v: 100 }));

function setColor(color: string) {
  const normalized = normalizeHex(color);
  if (!normalized) return;
  hsv.value = hexToHsv(normalized, hsv.value.h);
  hexInput.value = normalized.toUpperCase();
}
function setOpen(value: boolean) {
  if (value && props.disabled) return;
  if (value) setColor(props.modelValue);
  open.value = value;
}
watch(() => props.modelValue, value => { if (!open.value) setColor(value); });
watch(() => props.disabled, value => { if (value) open.value = false; });
function syncHex() { hexInput.value = draft.value.toUpperCase(); }
function editHex(event: Event) {
  hexInput.value = (event.target as HTMLInputElement).value;
  if (validHex.value) hsv.value = hexToHsv(validHex.value, hsv.value.h);
}
function editRgb(index: number, event: Event) {
  const input = event.target as HTMLInputElement;
  if (!input.value.trim() || !Number.isFinite(input.valueAsNumber)) return;
  const channels: [number, number, number] = [...rgb.value];
  channels[index] = Math.round(clamp(input.valueAsNumber, 255));
  setColor(rgbToHex(...channels));
  input.value = String(channels[index]);
}
function restoreRgb(index: number, event: FocusEvent) { (event.target as HTMLInputElement).value = String(rgb.value[index]); }
function editHue(event: Event) { hsv.value.h = Number((event.target as HTMLInputElement).value); syncHex(); }
function position(event: PointerEvent) {
  const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
  hsv.value.s = clamp((event.clientX - bounds.left) / bounds.width * 100);
  hsv.value.v = 100 - clamp((event.clientY - bounds.top) / bounds.height * 100);
  syncHex();
}
function pointerDown(event: PointerEvent) {
  if (event.button !== 0) return;
  const area = event.currentTarget as HTMLElement;
  area.focus(); area.setPointerCapture(event.pointerId); position(event);
}
function pointerMove(event: PointerEvent) { if ((event.currentTarget as HTMLElement).hasPointerCapture(event.pointerId)) position(event); }
function keyboard(event: KeyboardEvent) {
  const step = event.shiftKey ? 10 : 1;
  if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.key)) return;
  event.preventDefault();
  if (event.key === 'ArrowLeft') hsv.value.s = clamp(hsv.value.s - step);
  if (event.key === 'ArrowRight') hsv.value.s = clamp(hsv.value.s + step);
  if (event.key === 'ArrowUp') hsv.value.v = clamp(hsv.value.v + step);
  if (event.key === 'ArrowDown') hsv.value.v = clamp(hsv.value.v - step);
  if (event.key === 'Home') hsv.value.s = 0;
  if (event.key === 'End') hsv.value.s = 100;
  syncHex();
}
function apply() {
  if (!validHex.value) return;
  emit('update:modelValue', validHex.value);
  open.value = false;
}
</script>

<template>
  <PopoverRoot :open="open" @update:open="setOpen">
    <PopoverTrigger v-bind="$attrs" type="button" class="vela-color-trigger" :class="[variant, { selected }]" :disabled="disabled" :aria-label="`${label}：${modelValue}`" :data-tooltip="label">
      <span class="vela-color-rainbow"><AppIcon v-if="selected" name="tick" :size="14" /><span v-else class="vela-color-center" :style="{ background: modelValue }"></span></span>
      <span v-if="variant === 'option'">自定义</span>
      <template v-if="variant === 'field'"><span class="vela-color-field-value">{{ modelValue.toUpperCase() }}</span><AppIcon name="chevron-down" :size="12" /></template>
    </PopoverTrigger>
    <PopoverPortal>
      <PopoverContent class="vela-color-flyout" data-vela-picker side="bottom" align="end" :side-offset="8" :collision-padding="8" :prioritize-position="true" :aria-label="label" @escape-key-down.stop>
        <header class="vela-color-heading"><strong>{{ label }}</strong><span>选择你的颜色</span></header>
        <div class="vela-color-plane" :style="{ backgroundColor: hueColor }" role="slider" tabindex="0" aria-label="饱和度和明度，左右调整饱和度，上下调整明度" :aria-valuemin="0" :aria-valuemax="100" :aria-valuenow="Math.round(hsv.s)" :aria-valuetext="`饱和度 ${Math.round(hsv.s)}%，明度 ${Math.round(hsv.v)}%`" @pointerdown.prevent="pointerDown" @pointermove="pointerMove" @keydown="keyboard">
          <span class="vela-color-cursor" :style="{ left: `${hsv.s}%`, top: `${100 - hsv.v}%`, background: draft }"></span>
        </div>
        <div class="vela-color-hue-row"><span class="vela-color-preview" :style="{ background: draft }"></span><input class="vela-color-hue" type="range" min="0" max="360" step="1" :value="hsv.h" aria-label="色相" @input="editHue" /></div>
        <div class="vela-color-values">
          <label class="vela-color-hex"><span>HEX</span><input :value="hexInput" maxlength="7" spellcheck="false" autocomplete="off" aria-label="HEX 颜色值" @keydown.enter.stop.prevent="apply" :aria-invalid="!validHex" @input="editHex" @blur="validHex && setColor(validHex)" /></label>
          <label v-for="(channel, index) in ['R', 'G', 'B']" :key="channel"><span>{{ channel }}</span><input type="number" min="0" max="255" step="1" :value="rgb[index]" :aria-label="`${channel} 颜色通道`" @input="editRgb(index, $event)" @blur="restoreRgb(index, $event)" /></label>
        </div>
        <p v-if="!validHex" class="vela-color-error" role="alert">请输入 3 位或 6 位十六进制颜色。</p>
        <footer class="vela-color-footer"><div class="vela-color-comparison" :aria-label="`原色 ${modelValue}，新色 ${draft}`"><span :style="{ background: modelValue }"></span><span :style="{ background: draft }"></span><small>{{ changed ? '原色 / 新色' : '当前颜色' }}</small></div><button type="button" class="vela-color-action" @click="open = false">取消</button><button type="button" class="vela-color-action primary" :disabled="!validHex" @click="apply">应用</button></footer>
      </PopoverContent>
    </PopoverPortal>
  </PopoverRoot>
</template>

<!-- Popover content is teleported by Reka; use namespaced global styles. -->
<style>
.vela-color-trigger { display: inline-flex; flex: none; align-items: center; justify-content: center; gap: 9px; padding: 0; color: var(--win-text); cursor: pointer; border: 1px solid var(--win-control-stroke-bottom); border-radius: 6px; background: var(--win-control); transition: background .15s, box-shadow .15s; }
.vela-color-trigger:hover, .vela-color-trigger[data-state=open] { background: var(--win-control-hover); box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 20%, transparent); }
.vela-color-trigger:disabled { opacity: .45; cursor: default; }
.vela-color-trigger.swatch { width: 28px; height: 28px; }
.vela-color-trigger.round { width: 27px; height: 27px; border-radius: 50%; border-color: #ffffff55; background: transparent; }
.vela-color-trigger.round .vela-color-rainbow { width: 100%; height: 100%; border-radius: inherit; }
.vela-color-trigger.option { flex-direction: column; justify-content: center; width: 100%; min-height: 64px; padding: 8px 5px; gap: 6px; border-color: transparent; background: transparent; font-size: 12px; white-space: nowrap; }
.vela-color-trigger.option.selected { background: var(--win-subtle-hover); border-color: var(--accent); }
.vela-color-trigger.field { height: 32px; padding: 4px 9px 4px 5px; }
.vela-color-field-value { font-size: 12px; font-variant-numeric: tabular-nums; }
.vela-color-rainbow { display: grid; place-items: center; width: 22px; height: 22px; flex: none; border-radius: 5px; background: conic-gradient(#ed5970 0deg 60deg, #f4c455 60deg 120deg, #64bd85 120deg 180deg, #52b9df 180deg 240deg, #7779d8 240deg 300deg, #c46daf 300deg 360deg); color: #fff; }
.option .vela-color-rainbow { width: 25px; height: 25px; border-radius: 50%; }
.vela-color-center { width: 11px; height: 11px; border: 2px solid #fff; border-radius: 50%; }
.vela-color-flyout { z-index: 150; width: 292px; max-width: calc(100vw - 16px); max-height: var(--reka-popover-content-available-height, calc(100vh - 16px)); overflow-y: auto; overscroll-behavior: contain; padding: 16px; border: 1px solid var(--win-control-stroke-bottom); border-radius: 12px; background: var(--win-flyout); color: var(--win-text); box-shadow: 0 12px 32px #00000024, 0 2px 6px #00000012; font-size: 12px; animation: color-picker-in .18s cubic-bezier(.2, 0, 0, 1); scrollbar-width: thin; }
.vela-color-heading { display: flex; align-items: baseline; justify-content: space-between; gap: 8px; margin-bottom: 13px; }
.vela-color-heading strong { font-size: 13px; font-weight: 600; }
.vela-color-heading span { color: var(--win-text-3); font-size: 11px; }
.vela-color-plane { position: relative; height: 148px; border-radius: 7px; background-image: linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, transparent); cursor: crosshair; touch-action: none; box-shadow: inset 0 0 0 1px #00000010; }
.vela-color-cursor { position: absolute; width: 14px; height: 14px; border: 2px solid white; border-radius: 50%; box-shadow: 0 0 0 1px #0005, 0 2px 5px #0005; transform: translate(-50%, -50%); pointer-events: none; }
.vela-color-hue-row { display: flex; gap: 12px; align-items: center; margin: 16px 0; }
.vela-color-preview { width: 29px; height: 29px; flex: none; border-radius: 7px; box-shadow: inset 0 0 0 1px #80808040; }
.vela-color-hue { appearance: none; width: 100%; min-width: 0; height: 12px; margin: 0; border-radius: 20px; background: linear-gradient(to right, #f00, #ff0, #0f0, #0ff, #00f, #f0f, #f00); cursor: pointer; }
.vela-color-hue::-webkit-slider-thumb { appearance: none; width: 17px; height: 17px; border: 3px solid #fff; border-radius: 50%; background: transparent; box-shadow: 0 0 0 1px #0003, 0 1px 4px #0005; }
.vela-color-values { display: grid; grid-template-columns: 1.9fr repeat(3, 1fr); gap: 6px; }
.vela-color-values label { min-width: 0; display: flex; flex-direction: column; gap: 6px; }
.vela-color-values label > span { color: var(--win-text-3); text-align: center; font-size: 10px; letter-spacing: .08em; }
.vela-color-values input { width: 100%; min-width: 0; height: 30px; padding: 4px 3px; text-align: center; border: 1px solid var(--win-control-stroke); border-bottom-color: var(--win-control-stroke-bottom); border-radius: 5px; background: var(--win-control); color: var(--win-text); font-size: 11px; font-variant-numeric: tabular-nums; }
.vela-color-values input::-webkit-inner-spin-button { appearance: none; }
.vela-color-values input[aria-invalid=true] { border-color: var(--win-danger); }
.vela-color-error { margin: 8px 0 0; color: var(--win-danger); font-size: 11px; }
.vela-color-footer { display: flex; align-items: center; justify-content: flex-end; gap: 7px; padding-top: 13px; margin-top: 14px; border-top: 1px solid var(--win-card-stroke); }
.vela-color-comparison { display: flex; align-items: center; margin-right: auto; }
.vela-color-comparison > span { width: 12px; height: 22px; box-shadow: inset 0 0 0 1px #80808025; }
.vela-color-comparison > span:first-child { border-radius: 4px 0 0 4px; }
.vela-color-comparison > span:nth-child(2) { border-radius: 0 4px 4px 0; }
.vela-color-comparison small { margin-left: 6px; color: var(--win-text-3); font-size: 10px; }
.vela-color-action { min-height: 29px; padding: 4px 10px; border: 1px solid var(--win-control-stroke); border-bottom-color: var(--win-control-stroke-bottom); border-radius: 5px; color: var(--win-text); background: var(--win-control); cursor: pointer; font-size: 12px; }
.vela-color-action:hover { background: var(--win-control-hover); }
.vela-color-action.primary { border-color: transparent; background: var(--accent); color: var(--accent-contrast); }
.vela-color-action:disabled { opacity: .45; cursor: default; }
@keyframes color-picker-in { from { opacity: 0; transform: translateY(-4px) scale(.98); } to { opacity: 1; transform: none; } }
@media (prefers-reduced-motion: reduce) { .vela-color-flyout { animation: none; } .vela-color-trigger { transition: none; } }
</style>
