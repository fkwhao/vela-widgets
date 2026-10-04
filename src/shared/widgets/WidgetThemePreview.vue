<script setup lang="ts">
import { onUnmounted, ref, watch } from "vue";
import type { ThemeMode } from "../types";

const props = defineProps<{ theme: ThemeMode; selected: boolean }>();
// Bumping the key remounts the stage so the CSS sequence always restarts from frame 0.
const playId = ref(0);
const playing = ref(false);
// After a selection, rejoin the ambient loop in its "settled" phase instead of rewinding it.
const resumed = ref(false);
let playTimer = 0;

function play(): void {
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  window.clearTimeout(playTimer);
  playId.value += 1;
  playing.value = true;
  playTimer = window.setTimeout(() => { playing.value = false; resumed.value = true; }, 1500);
}

// Only animate when the user actually picks this mode, never on page load.
watch(() => props.selected, (selected, wasSelected) => {
  if (selected && !wasSelected) play();
});
onUnmounted(() => window.clearTimeout(playTimer));
</script>

<template>
  <span class="widget-theme-preview" :class="[`theme-${theme}`, { playing, resumed }]" aria-hidden="true" inert>
    <span :key="playId" class="preview-stage">
      <span v-for="layer in ['base', 'top']" :key="layer" class="preview-scene" :class="[`scene-${layer}`, (theme === 'light') === (layer === 'top') ? 'mini-light' : 'mini-dark']">
        <span class="preview-wallpaper"></span>
        <i v-for="star in 4" :key="star" class="preview-star" :class="`star-${star}`"></i>
        <span class="preview-celestial"></span>
        <span class="abstract-ghost"><i></i><i></i><i></i></span>
        <span class="abstract-widget">
          <span class="abstract-sheen"></span>
          <span class="abstract-header">
            <span class="abstract-brand"><i></i><b></b></span>
            <i class="abstract-action"></i>
          </span>
          <span class="abstract-divider"></span>
          <span v-for="row in 3" :key="row" class="abstract-row" :class="{ 'row-featured': row === 1 }">
            <i class="abstract-symbol" :class="`symbol-${row}`"></i>
            <i class="abstract-line" :class="`line-${row}`"></i>
            <i class="abstract-endpoint"></i>
          </span>
          <span class="abstract-footer"><i></i><i></i></span>
        </span>
      </span>
      <span class="preview-ripple"></span>
      <svg class="preview-pointer" viewBox="0 0 20 24" aria-hidden="true">
        <path d="M2 1v18l4.7-4.7 3.2 7.1 3-1.3-3.2-7.1h6.8L2 1z" fill="#fff" stroke="rgba(18,25,36,.78)" stroke-linejoin="round" stroke-width="1.25" />
      </svg>
    </span>
  </span>
</template>

<style scoped>
/*
 * Two stacked scenes: "base" is the mode you are leaving, "top" is the mode
 * this card represents (system: base = day, top = night, split diagonally).
 * Light/dark loop a replay of the click-to-switch demo; system loops an
 * automatic day/night cycle with no pointer. Selecting a card plays the
 * one-shot sequence, then rejoins the loop.
 */
.widget-theme-preview {
  --reveal-x: 31px;
  --reveal-y: 42px;
  --ease-out: cubic-bezier(.16, 1, .3, 1);
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
  transition: border-color .2s ease, box-shadow .2s ease;
}
.preview-stage, .preview-scene, .preview-wallpaper { position: absolute; inset: 0; }
.preview-scene { overflow: hidden; }

.mini-light .preview-wallpaper {
  background:
    radial-gradient(ellipse at 76% 20%, rgba(96, 139, 198, .48), transparent 45%),
    radial-gradient(ellipse at 20% 88%, rgba(160, 181, 211, .48), transparent 42%),
    linear-gradient(135deg, #dfe7f1, #c2cedd);
}
.mini-dark .preview-wallpaper {
  background:
    radial-gradient(ellipse at 76% 20%, rgba(68, 101, 157, .43), transparent 47%),
    radial-gradient(ellipse at 17% 88%, rgba(88, 103, 127, .3), transparent 42%),
    linear-gradient(135deg, #30343a, #1b1d20);
}

/* Sun for light scenes, crescent moon for dark scenes. */
.preview-celestial { position: absolute; top: 10px; right: 9px; width: 14px; height: 14px; border-radius: 50%; }
.mini-light .preview-celestial {
  background: radial-gradient(circle, #fff6d8 0 38%, #ffd36b 62%, rgba(255, 211, 107, 0) 72%);
  box-shadow: 0 0 12px 3px rgba(255, 214, 120, .45);
}
.mini-dark .preview-celestial { box-shadow: inset -4px 3px 0 0 #e8ecf5, 0 0 10px rgba(200, 214, 255, .14); transform: rotate(-12deg); }
.theme-system .scene-base .preview-celestial { top: auto; right: auto; bottom: 7px; left: 7px; width: 11px; height: 11px; }
.preview-star { position: absolute; width: 2px; height: 2px; border-radius: 50%; background: #fff; opacity: 0; }
.mini-dark .preview-star { opacity: .55; }
.star-1 { top: 9px; right: 30px; }
.star-2 { top: 30px; right: 6px; width: 1.5px; height: 1.5px; }
.star-3 { bottom: 9px; right: 22px; }
.star-4 { bottom: 20px; right: 12px; width: 1.5px; height: 1.5px; }
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
  transition: transform .35s var(--ease-out), box-shadow .35s ease;
}
.abstract-sheen { position: absolute; inset: 0 0 auto; height: 24px; background: linear-gradient(180deg, rgba(255,255,255,.25), transparent); }
.abstract-header { position: relative; display: flex; height: 8px; flex: none; align-items: center; justify-content: space-between; }
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
.theme-system .scene-top { clip-path: polygon(64% 0, 128% 0, 128% 100%, 36% 100%); }
.preview-pointer { position: absolute; top: 41px; left: 29px; z-index: 5; width: 16px; height: 20px; opacity: 0; filter: drop-shadow(0 2px 2px rgba(0, 0, 0, .4)); }
.preview-ripple { position: absolute; top: 30px; left: 18px; z-index: 4; width: 24px; height: 24px; border: 1.5px solid rgba(255, 255, 255, .85); border-radius: 50%; opacity: 0; }

/* One-shot selection sequence (≈1.4s): pointer clicks the widget, the new mode spreads out from the click. */
.playing .preview-pointer { animation: pointer-click 1.4s var(--ease-out) both; }
.playing .preview-ripple { animation: ripple .62s .44s ease-out both; }
.playing .scene-base .abstract-widget { animation: widget-press .3s .4s ease-out both; }
.playing .scene-top { animation: reveal-circle .72s .44s var(--ease-out) both; }
.playing.theme-system .scene-top { animation: reveal-sweep .9s .44s var(--ease-out) both; }
.playing .scene-top .preview-celestial { animation: celestial-rise .8s .62s var(--ease-out) both; }
.playing .scene-top.mini-dark .preview-celestial { animation-name: moon-rise; }
.playing .scene-top .preview-star { animation: star-in .5s ease-out both; }
.playing .scene-top .star-1 { animation-delay: .78s; }
.playing .scene-top .star-2 { animation-delay: .9s; }
.playing .scene-top .star-3 { animation-delay: .98s; }
.playing .scene-top .star-4 { animation-delay: 1.08s; }

@keyframes pointer-click {
  0% { opacity: 0; transform: translate(70px, 34px); }
  12% { opacity: 1; }
  28% { transform: translate(0, 0) scale(1); }
  32% { transform: translate(0, 0) scale(.82); }
  38% { transform: translate(0, 0) scale(1); }
  70% { opacity: 1; transform: translate(10px, 14px); }
  100% { opacity: 0; transform: translate(16px, 22px); }
}
@keyframes ripple { from { opacity: .9; transform: scale(.2); } to { opacity: 0; transform: scale(1.7); } }
@keyframes widget-press { 50% { transform: scale(.965); } }
@keyframes reveal-circle { from { clip-path: circle(0px at 31px 42px); } to { clip-path: circle(130px at 31px 42px); } }
@keyframes reveal-sweep {
  from { clip-path: polygon(128% 0, 128% 0, 128% 100%, 100% 100%); }
  72% { clip-path: polygon(60% 0, 128% 0, 128% 100%, 32% 100%); }
  to { clip-path: polygon(64% 0, 128% 0, 128% 100%, 36% 100%); }
}
@keyframes celestial-rise { from { opacity: 0; transform: translateY(9px) scale(.55); } }
@keyframes moon-rise { from { opacity: 0; transform: translate(-6px, 6px) rotate(-70deg) scale(.7); } to { transform: rotate(-12deg); } }
@keyframes star-in { 0% { opacity: 0; transform: scale(0); } 60% { opacity: 1; transform: scale(1.6); } 100% { opacity: .55; transform: scale(1); } }

/*
 * Ambient loops. Delays are chosen so every card loads already showing its own
 * mode and the three cards act at different moments (~1.6s, ~3.8s, ~5s).
 */
.widget-theme-preview { --loop: 7s; --loop-delay: -4.4s; --resume-delay: -2.52s; --delay: var(--loop-delay); }
.theme-dark { --loop: 7.6s; --loop-delay: -2.8s; --resume-delay: -2.74s; }
.theme-system { --loop: 10s; --loop-delay: 3s; --resume-delay: 0s; }
.resumed { --delay: var(--resume-delay); }

/* Light / dark: replay the click demo — leave, tap, reveal, settle, fade back. */
.widget-theme-preview:not(.playing):not(.theme-system) .preview-pointer { animation: pointer-loop var(--loop) var(--delay) cubic-bezier(.38, .02, .2, 1) infinite; }
.widget-theme-preview:not(.playing):not(.theme-system) .preview-ripple { animation: ripple-loop var(--loop) var(--delay) ease-out infinite; }
.widget-theme-preview:not(.playing):not(.theme-system) .scene-base .abstract-widget { animation: press-loop var(--loop) var(--delay) ease-out infinite; }
.widget-theme-preview:not(.playing):not(.theme-system) .scene-top { animation: reveal-loop var(--loop) var(--delay) linear infinite; }
.widget-theme-preview:not(.playing):not(.theme-system) .scene-top .preview-celestial { animation: sun-loop var(--loop) var(--delay) linear infinite; }
.widget-theme-preview:not(.playing):not(.theme-system) .scene-top.mini-dark .preview-celestial { animation-name: moon-loop; }
.widget-theme-preview:not(.playing):not(.theme-system) .scene-top.mini-dark .preview-star { animation: star-loop var(--loop) var(--delay) ease-out infinite; }
.widget-theme-preview:not(.playing):not(.theme-system) .scene-top .star-2 { animation-delay: calc(var(--delay) + .12s); }
.widget-theme-preview:not(.playing):not(.theme-system) .scene-top .star-3 { animation-delay: calc(var(--delay) + .22s); }
.widget-theme-preview:not(.playing):not(.theme-system) .scene-top .star-4 { animation-delay: calc(var(--delay) + .32s); }

/* System: no pointer — night falls, dawn sweeps back across, dusk returns to the split. */
.theme-system:not(.playing) .scene-top { animation: day-night var(--loop) var(--delay) cubic-bezier(.65, 0, .35, 1) infinite; }

@keyframes pointer-loop {
  0%, 4% { opacity: 0; transform: translate(70px, 34px); }
  8% { opacity: 1; }
  16% { transform: translate(0, 0) scale(1); }
  18% { transform: translate(0, 0) scale(.82); }
  20% { transform: translate(0, 0) scale(1); }
  30% { opacity: 1; transform: translate(10px, 14px); }
  36%, 100% { opacity: 0; transform: translate(16px, 22px); }
}
@keyframes ripple-loop {
  0%, 19% { opacity: 0; transform: scale(.2); }
  19.5% { opacity: .9; transform: scale(.25); }
  29%, 100% { opacity: 0; transform: scale(1.7); }
}
@keyframes press-loop { 0%, 17%, 23%, 100% { transform: scale(1); } 20% { transform: scale(.965); } }
@keyframes reveal-loop {
  0%, 19% { opacity: 1; clip-path: circle(0px at 31px 42px); animation-timing-function: cubic-bezier(.16, 1, .3, 1); }
  31%, 86% { opacity: 1; clip-path: circle(130px at 31px 42px); animation-timing-function: ease-in-out; }
  96%, 100% { opacity: 0; clip-path: circle(130px at 31px 42px); }
}
@keyframes sun-loop {
  0%, 22% { opacity: 0; transform: translateY(9px) scale(.55); animation-timing-function: cubic-bezier(.16, 1, .3, 1); }
  36%, 100% { opacity: 1; transform: none; }
}
@keyframes moon-loop {
  0%, 22% { opacity: 0; transform: translate(-6px, 6px) rotate(-70deg) scale(.7); animation-timing-function: cubic-bezier(.16, 1, .3, 1); }
  36%, 100% { opacity: 1; transform: rotate(-12deg); }
}
@keyframes star-loop {
  0%, 26% { opacity: 0; transform: scale(0); }
  31% { opacity: 1; transform: scale(1.6); }
  34%, 100% { opacity: .55; transform: scale(1); }
}
@keyframes day-night {
  0%, 20% { clip-path: polygon(64% 0, 160% 0, 160% 100%, 36% 100%); }
  32%, 50% { clip-path: polygon(-64% 0, 160% 0, 160% 100%, -92% 100%); }
  66%, 82% { clip-path: polygon(156% 0, 160% 0, 160% 100%, 128% 100%); }
  94%, 100% { clip-path: polygon(64% 0, 160% 0, 160% 100%, 36% 100%); }
}

@media (prefers-reduced-motion: reduce) {
  .widget-theme-preview, .abstract-widget { transition: none; }
  .preview-stage * { animation: none !important; }
}
</style>
