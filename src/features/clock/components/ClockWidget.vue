<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import ClockThemeFace from "./ClockThemeFace.vue";
import AnalogClock from "./AnalogClock.vue";
import ClockToolsPanel from "./ClockToolsPanel.vue";
import { clockModes, type ClockMode } from "../../../shared/types";
import WidgetFrame from "../../../shared/widgets/WidgetFrame.vue";
import AppIcon from "../../../shared/ui/AppIcon.vue";
import { snapshot, clockAction } from "../../../app/store";
import { useWidgetClock } from "../../../shared/composables/useWidgetClock";
const tools = computed(() => snapshot.value.settings.clockTools);
const compactClock = computed(() => snapshot.value.settings.widgets.clock.size === "small");
const modeButton = ref<HTMLButtonElement | null>(null);
const modeMenu = ref(false); const modeError = ref("");
async function closeModeMenu() { modeMenu.value = false; await nextTick(); modeButton.value?.focus(); }
function dismissModeMenu() { modeMenu.value = false; if (document.activeElement === modeButton.value) modeButton.value?.blur(); }
function onOutsidePointer(event: PointerEvent) {
  if (!modeMenu.value || (event.target instanceof Element && event.target.closest('.clock-mode-menu, .clock-mode-picker'))) return;
  dismissModeMenu();
}
onMounted(() => { document.addEventListener('pointerdown', onOutsidePointer, true); window.addEventListener('blur', dismissModeMenu); });
onUnmounted(() => { document.removeEventListener('pointerdown', onOutsidePointer, true); window.removeEventListener('blur', dismissModeMenu); });
async function chooseMode(mode: ClockMode) { modeMenu.value = false; modeError.value = ""; try { await clockAction({action:"mode",mode}); await nextTick(); modeButton.value?.focus(); } catch(e) { modeError.value = typeof e === "string" ? e : "模式切换失败。"; } }
const runningHint = computed(() => [tools.value.timer.deadline !== null ? "计时器运行中" : "", tools.value.stopwatch.startedAt !== null ? "秒表运行中" : "", tools.value.alarms.some(a=>a.enabled||a.snoozeAt!==null) ? "有待响闹钟" : ""].filter(Boolean).join(" · "));
const settings = computed(() => snapshot.value.settings.clock);
const seconds = ref(true);
const now = useWidgetClock(seconds);
function time(zone?: string) { return new Intl.DateTimeFormat("en-GB", { hour: "2-digit", minute: "2-digit", ...(settings.value.showSeconds ? { second: "2-digit" } : {}), hour12: settings.value.hour12, ...(zone ? { timeZone: zone } : {}) }).format(now.value); }
const localTime = computed(() => time());
const dateLabel = computed(() => new Intl.DateTimeFormat("zh-CN", { month: "long", day: "numeric", weekday: "long" }).format(now.value));
const cities = computed(() => settings.value.cities.map((city) => {
  const parts = new Intl.DateTimeFormat("en-GB", { timeZone: city.timeZone, hour: "numeric", minute: "numeric", hourCycle: "h23" }).formatToParts(now.value);
  const hour = Number(parts.find((p) => p.type === "hour")?.value);
  const minute = Number(parts.find((p) => p.type === "minute")?.value);
  // Short UTC offsets come from the time-zone database, including daylight saving.
  function offset(zone?: string) { const text = new Intl.DateTimeFormat("en-US", { timeZone: zone, timeZoneName: "shortOffset" }).formatToParts(now.value).find((p) => p.type === "timeZoneName")?.value ?? "GMT"; const match = /GMT([+-])(\d+)(?::(\d+))?/.exec(text); return match ? (match[1] === "-" ? -1 : 1) * (Number(match[2]) + Number(match[3] ?? 0) / 60) : 0; }
  const delta = offset(city.timeZone) - offset();
  return { ...city, display: new Intl.DateTimeFormat("en-GB", { timeZone: city.timeZone, hour: "2-digit", minute: "2-digit", hour12: settings.value.hour12 }).format(now.value), daylight: hour >= 6 && hour < 18, difference: delta === 0 ? "同一时刻" : `${delta > 0 ? '+' : '−'}${Math.abs(delta)} 小时`, hour, minute };
}));

</script>
<template>
  <WidgetFrame kind="clock" header-only class="clock-immersive clock-themed" :class="{ 'clock-compact': compactClock }">
    <template #default="{ widget, drag }">
    <div class="clock-mode-reveal-zone">
    <header class="clock-mode-header" :class="{ 'compact-mode-header': compactClock, 'menu-open': modeMenu }" @keydown.esc.stop="closeModeMenu" :data-tauri-drag-region="compactClock ? undefined : drag"><button ref="modeButton" v-if="widget.size === 'small'" class="clock-mode-picker" aria-label="切换时钟模式" :data-tooltip="`当前：${clockModes.find(m => m.value === tools.mode)?.label} · 切换模式`" aria-haspopup="menu" :aria-expanded="modeMenu" @click="modeMenu=!modeMenu"><AppIcon v-if="compactClock" name="more" :size="15" /><template v-else><AppIcon :name="clockModes.find(m=>m.value===tools.mode)?.icon??'clock'" :size="14" />{{ clockModes.find(m=>m.value===tools.mode)?.label }}<AppIcon name="chevron-down" :size="11" /></template></button><nav v-else class="clock-mode-tabs" aria-label="时钟模式"><button v-for="mode in clockModes" :key="mode.value" class="widget-icon-button" :class="{selected:tools.mode===mode.value}" :aria-label="mode.label" :aria-pressed="tools.mode===mode.value" @click="chooseMode(mode.value)"><AppIcon :name="mode.icon" :size="16" /></button></nav><span v-if="runningHint" class="clock-running-dot" :data-tooltip="runningHint" :aria-label="runningHint"></span><span v-if="widget.size !== 'small'" class="extra-caption">{{ clockModes.find(m=>m.value===tools.mode)?.label }}</span></header>
    </div>
    <div v-if="modeMenu" class="clock-mode-menu" role="menu" @keydown.esc.stop="closeModeMenu"><button v-for="mode in clockModes" :key="mode.value" role="menuitem" :aria-current="tools.mode === mode.value ? 'true' : undefined" @click="chooseMode(mode.value)"><AppIcon :name="mode.icon" :size="14" /><span>{{ mode.label }}</span><AppIcon v-if="tools.mode === mode.value" name="tick" :size="12" /></button></div>
    <ClockToolsPanel v-if="tools.mode !== 'clock'" :key="tools.mode" :size="widget.size" :drag="drag" />
    <div v-else class="clock-display" :data-tauri-drag-region="widget.locked ? undefined : ''">
    <ClockThemeFace v-if="(settings.theme !== 'default' || widget.size === 'medium')" :now="now" :settings="settings" :size="widget.size" :data-tauri-drag-region="drag" />
    <template v-else>
    <div v-if="widget.size === 'small'" class="clock-analog-small" :data-tauri-drag-region="drag">
      <AnalogClock square :now="now" :label="`本地时间 ${localTime}，${dateLabel}`" />

    </div>
    <template v-else>
      <AnalogClock :now="now" :label="`本地时间 ${localTime}`" :data-tauri-drag-region="drag" />
      <span class="clock-large-date extra-caption" :data-tauri-drag-region="drag">{{ localTime }} · {{ dateLabel }}</span>
      <div class="clock-cities large"><div v-for="city in cities" :key="city.timeZone" class="clock-city" :data-tauri-drag-region="drag"><div><span>{{ city.name }}</span><small><AppIcon :name="city.daylight ? 'sun' : 'moon'" :size="11" />{{ city.difference }}</small></div><strong>{{ city.display }}</strong></div><p v-if="!cities.length" class="extra-empty-caption">此刻，也是新的一刻。</p></div>
    </template>
    </template>
    </div>
    <p v-if="modeError" class="extra-notice" role="alert">{{ modeError }}</p>
    </template>
    <template #context-actions="{dismiss}"><button v-for="mode in clockModes" :key="mode.value" role="menuitem" @click="chooseMode(mode.value).then(dismiss)"><AppIcon :name="mode.icon" :size="14" />{{ mode.label }}</button><div class="context-divider"></div></template>
  </WidgetFrame>
</template>
