<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import type { ThemeMode } from "../types";

const props = defineProps<{ theme: ThemeMode; selected: boolean }>();
const systemThemeQuery = window.matchMedia("(prefers-color-scheme: dark)");
const reducedMotionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
const systemIsDark = ref(systemThemeQuery.matches);
const reducedMotion = ref(reducedMotionQuery.matches);
const targetTheme = computed<"light" | "dark">(() => {
  if (props.theme === "system") return systemIsDark.value ? "dark" : "light";
  return props.theme;
});
const visualTheme = ref<"light" | "dark">(targetTheme.value === "light" ? "dark" : "light");
const motionEnabled = ref(false);
const entering = ref(true);
let animationFrameOne = 0;
let animationFrameTwo = 0;

function playThemeTransition(): void {
  window.cancelAnimationFrame(animationFrameOne);
  window.cancelAnimationFrame(animationFrameTwo);
  if (reducedMotion.value) {
    visualTheme.value = targetTheme.value;
    motionEnabled.value = false;
    entering.value = false;
    return;
  }

  motionEnabled.value = false;
  entering.value = true;
  visualTheme.value = targetTheme.value === "light" ? "dark" : "light";
  void nextTick(() => {
    animationFrameOne = window.requestAnimationFrame(() => {
      animationFrameTwo = window.requestAnimationFrame(() => {
        motionEnabled.value = true;
        visualTheme.value = targetTheme.value;
        entering.value = false;
      });
    });
  });
}

function onSystemThemeChange(event: MediaQueryListEvent): void {
  systemIsDark.value = event.matches;
}
function onReducedMotionChange(event: MediaQueryListEvent): void {
  reducedMotion.value = event.matches;
}

onMounted(() => {
  systemThemeQuery.addEventListener("change", onSystemThemeChange);
  reducedMotionQuery.addEventListener("change", onReducedMotionChange);
  playThemeTransition();
});
watch(() => props.selected, (selected, wasSelected) => {
  if (selected && !wasSelected) playThemeTransition();
});
watch([targetTheme, reducedMotion], playThemeTransition);

onUnmounted(() => {
  systemThemeQuery.removeEventListener("change", onSystemThemeChange);
  reducedMotionQuery.removeEventListener("change", onReducedMotionChange);
  window.cancelAnimationFrame(animationFrameOne);
  window.cancelAnimationFrame(animationFrameTwo);
});
</script>

<template>
  <span
    class="widget-theme-preview"
    :class="[`theme-${theme}`, `mini-${visualTheme}`, { 'motion-enabled': motionEnabled, entering }]"
    aria-hidden="true"
    inert
  >
    <span class="preview-wallpaper wallpaper-light"></span>
    <span class="preview-wallpaper wallpaper-dark"></span>
    <span class="abstract-ghost">
      <i></i><i></i><i></i>
    </span>
    <span class="abstract-widget">
      <span class="abstract-sheen"></span>
      <span class="abstract-header">
        <span class="abstract-brand"><i></i><b></b></span>
        <i class="abstract-action"></i>
      </span>
      <span class="abstract-divider"></span>
      <span class="abstract-row" :class="{ 'row-featured': row === 1 }" v-for="row in 3" :key="row">
        <i class="abstract-symbol" :class="`symbol-${row}`"></i>
        <i class="abstract-line" :class="`line-${row}`"></i>
        <i class="abstract-endpoint"></i>
      </span>
      <span class="abstract-footer"><i></i><i></i></span>
    </span>
    <svg class="preview-pointer" viewBox="0 0 20 24" aria-hidden="true">
      <path d="M2 1v18l4.7-4.7 3.2 7.1 3-1.3-3.2-7.1h6.8L2 1z" fill="#fff" stroke="rgba(18,25,36,.78)" stroke-linejoin="round" stroke-width="1.25" />
      <path d="M2 1v18l4.7-4.7 3.2 7.1 3-1.3-3.2-7.1h6.8L2 1z" fill="none" stroke="rgba(255,255,255,.82)" stroke-linejoin="round" stroke-width=".45" />
    </svg>
  </span>
</template>

<style scoped>
.widget-theme-preview {
  --theme-preview-delay: 0ms;
  --theme-preview-duration: 900ms;
  --pointer-duration: 6.4s;
  --glass-line: rgba(34, 50, 72, .27);
  --glass-muted: rgba(36, 54, 78, .43);
  --glass-outline: rgba(255, 255, 255, .78);
  --glass-surface: rgba(246, 249, 253, .39);
  --glass-shadow: 0 8px 22px rgba(25, 38, 58, .19), inset 0 1px rgba(255, 255, 255, .74);
  position: relative;
  display: block;
  width: 142px;
  height: 90px;
  overflow: hidden;
  pointer-events: none;
  border: 1px solid rgba(125, 139, 158, .25);
  border-radius: 8px;
  background: #23262a;
  isolation: isolate;
}

.theme-dark { --theme-preview-delay: 160ms; --theme-preview-duration: 1100ms; --pointer-duration: 7.4s; }
.theme-system { --theme-preview-delay: 320ms; --theme-preview-duration: 1250ms; --pointer-duration: 8.4s; }

.preview-wallpaper {
  position: absolute;
  inset: 0;
  transition: opacity var(--theme-preview-duration) ease;
}
.wallpaper-light {
  opacity: 0;
  background:
    radial-gradient(ellipse at 76% 20%, rgba(96, 139, 198, .48), transparent 45%),
    radial-gradient(ellipse at 20% 88%, rgba(160, 181, 211, .48), transparent 42%),
    linear-gradient(135deg, #dfe7f1, #c2cedd);
}
.wallpaper-dark {
  opacity: 1;
  background:
    radial-gradient(ellipse at 76% 20%, rgba(68, 101, 157, .43), transparent 47%),
    radial-gradient(ellipse at 17% 88%, rgba(88, 103, 127, .3), transparent 42%),
    linear-gradient(135deg, #30343a, #1b1d20);
}
.mini-light .wallpaper-light { opacity: 1; }
.mini-light .wallpaper-dark { opacity: 0; }

.abstract-ghost {
  position: absolute;
  top: 19px;
  left: 35px;
  display: grid;
  width: 77px;
  height: 48px;
  align-content: center;
  gap: 5px;
  padding: 0 9px;
  border: 1px solid rgba(255, 255, 255, .19);
  border-radius: 9px;
  background: rgba(230, 236, 245, .08);
  box-shadow: inset 0 1px rgba(255, 255, 255, .16);
  opacity: .57;
  transform: rotate(7deg);
  transition: background-color var(--theme-preview-duration) ease, border-color var(--theme-preview-duration) ease,
    opacity var(--theme-preview-duration) ease;
}
.abstract-ghost i { width: 40%; height: 2px; border-radius: 3px; background: rgba(255, 255, 255, .44); }
.abstract-ghost i:nth-child(2) { width: 62%; }
.abstract-ghost i:nth-child(3) { width: 47%; }

.abstract-widget {
  position: absolute;
  top: 11px;
  left: 18px;
  display: flex;
  width: 104px;
  height: 68px;
  flex-direction: column;
  overflow: hidden;
  padding: 8px 9px 6px;
  border: 1px solid var(--glass-outline);
  border-radius: 11px;
  background: var(--glass-surface);
  box-shadow: var(--glass-shadow);
  color: var(--glass-line);
  -webkit-backdrop-filter: blur(15px) saturate(155%);
  backdrop-filter: blur(15px) saturate(155%);
  transform: scale(1);
  transition: background-color var(--theme-preview-duration) ease,
    border-color var(--theme-preview-duration) ease,
    box-shadow var(--theme-preview-duration) ease,
    color var(--theme-preview-duration) ease,
    opacity var(--theme-preview-duration) ease,
    transform var(--theme-preview-duration) cubic-bezier(.2,.75,.25,1);
}
.entering .abstract-widget { opacity: .86; transform: translateY(2px) scale(.94); }
.abstract-sheen {
  position: absolute;
  inset: 0 0 auto;
  height: 24px;
  background: linear-gradient(180deg, rgba(255,255,255,.25), transparent);
  pointer-events: none;
}
.abstract-header {
  position: relative;
  display: flex;
  height: 8px;
  flex: none;
  align-items: center;
  justify-content: space-between;
}
.abstract-brand { display: inline-flex; align-items: center; gap: 5px; }
.abstract-brand > i { width: 5px; height: 5px; border-radius: 50%; background: #3b67b8; box-shadow: 0 0 0 2px rgba(59,103,184,.13); }
.abstract-brand > b { width: 24px; height: 3px; border-radius: 3px; background: var(--glass-line); }
.abstract-action { width: 11px; height: 3px; border-radius: 3px; background: var(--glass-muted); }
.abstract-divider { height: 1px; flex: none; margin: 5px 0 4px; background: var(--glass-outline); opacity: .65; }
.abstract-row { display: flex; height: 9px; flex: none; align-items: center; gap: 5px; }
.abstract-symbol { width: 5px; height: 5px; flex: none; border: 1px solid var(--glass-muted); border-radius: 2px; }
.abstract-row.row-featured .abstract-symbol { border-color: #3b67b8; background: #3b67b8; }
.symbol-2 { border-radius: 50%; }
.abstract-line { height: 2px; border-radius: 4px; background: var(--glass-line); }
.line-1 { width: 39px; }
.line-2 { width: 51px; opacity: .76; }
.line-3 { width: 32px; opacity: .59; }
.abstract-endpoint { width: 7px; height: 3px; margin-left: auto; border-radius: 3px; background: var(--glass-muted); opacity: .68; }
.abstract-footer { display: flex; align-items: center; gap: 4px; margin-top: auto; }
.abstract-footer i { width: 14px; height: 2px; border-radius: 3px; background: var(--glass-line); opacity: .54; }
.abstract-footer i:last-child { width: 8px; background: #3b67b8; opacity: .88; }
.abstract-brand > b,
.abstract-action,
.abstract-divider,
.abstract-symbol,
.abstract-line,
.abstract-endpoint,
.abstract-footer i {
  transition: background-color var(--theme-preview-duration) ease,
    border-color var(--theme-preview-duration) ease,
    box-shadow var(--theme-preview-duration) ease;
}

.preview-pointer {
  position: absolute;
  top: 39px;
  left: 27px;
  z-index: 5;
  width: 16px;
  height: 20px;
  overflow: visible;
  filter: drop-shadow(0 2px 2px rgba(0, 0, 0, .4));
  opacity: 0;
  pointer-events: none;
}
.motion-enabled .preview-pointer {
  animation: pointer-cycle var(--pointer-duration) cubic-bezier(.38,.02,.2,1) infinite;
  animation-delay: var(--theme-preview-delay);
}
.motion-enabled .abstract-row.row-featured .abstract-symbol {
  animation: marker-pulse var(--pointer-duration) ease-in-out infinite;
  animation-delay: var(--theme-preview-delay);
}

@keyframes pointer-cycle {
  0%, 7% { opacity: 0; transform: translate(75px, 34px) scale(.82); }
  13% { opacity: 1; transform: translate(75px, 34px) scale(.9); }
  38%, 42% { opacity: 1; transform: translate(0, 0) scale(1); }
  45% { opacity: 1; transform: translate(0, 0) scale(.79); }
  48% { opacity: 1; transform: translate(0, 0) scale(1); }
  56% { opacity: .96; transform: translate(36px, 13px) scale(1); }
  66%, 100% { opacity: 0; transform: translate(75px, 34px) scale(.82); }
}
@keyframes marker-pulse {
  0%, 35%, 54%, 100% { box-shadow: 0 0 0 0 rgba(59, 103, 184, 0); }
  41% { box-shadow: 0 0 0 4px rgba(59, 103, 184, .3); }
  48% { box-shadow: 0 0 0 2px rgba(59, 103, 184, .17); }
}

.mini-light {
  --glass-line: rgba(34, 50, 72, .34);
  --glass-muted: rgba(36, 54, 78, .48);
  --glass-outline: rgba(255, 255, 255, .78);
  --glass-surface: rgba(246, 249, 253, .39);
  --glass-shadow: 0 8px 22px rgba(25, 38, 58, .19), inset 0 1px rgba(255, 255, 255, .74);
}
.mini-dark {
  --glass-line: rgba(242, 245, 250, .76);
  --glass-muted: rgba(226, 231, 239, .48);
  --glass-outline: rgba(255, 255, 255, .24);
  --glass-surface: rgba(47, 50, 55, .38);
  --glass-shadow: 0 8px 22px rgba(0, 0, 0, .27), inset 0 1px rgba(255, 255, 255, .19);
}

.motion-enabled .preview-wallpaper,
.motion-enabled .abstract-ghost,
.motion-enabled .abstract-widget,
.motion-enabled .abstract-widget * {
  transition-delay: var(--theme-preview-delay);
}

@media (prefers-reduced-motion: reduce) {
  .preview-wallpaper,
  .abstract-ghost,
  .abstract-widget,
  .abstract-widget *,
  .preview-pointer { transition: none !important; animation: none !important; }
}
</style>
