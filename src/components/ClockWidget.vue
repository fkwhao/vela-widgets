<script setup lang="ts">
import { computed, toRef, ref } from "vue";
import ClockToolsPanel from "./ClockToolsPanel.vue";
import { clockModes, type ClockMode } from "../types";
import WidgetFrame from "./WidgetFrame.vue";
import AppIcon from "./AppIcon.vue";
import { snapshot, clockAction } from "../lib/store";
import { useWidgetClock } from "../lib/useWidgetClock";
const tools = computed(() => snapshot.value.settings.clockTools);
const modeMenu = ref(false); const modeError = ref("");
async function chooseMode(mode: ClockMode) { modeMenu.value = false; modeError.value = ""; try { await clockAction({action:"mode",mode}); } catch(e) { modeError.value = typeof e === "string" ? e : "模式切换失败。"; } }
const runningHint = computed(() => [tools.value.timer.deadline !== null ? "计时器运行中" : "", tools.value.stopwatch.startedAt !== null ? "秒表运行中" : "", tools.value.alarms.some(a=>a.enabled||a.snoozeAt!==null) ? "有待响闹钟" : ""].filter(Boolean).join(" · "));
const settings = computed(() => snapshot.value.settings.clock);
const seconds = toRef(() => settings.value.showSeconds);
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
const hands = computed(() => ({ hour: (now.value.getHours() % 12) * 30 + now.value.getMinutes() / 2, minute: now.value.getMinutes() * 6, second: now.value.getSeconds() * 6 }));
</script>
<template>
  <WidgetFrame kind="clock" header-only>
    <template #default="{ widget, drag }">
    <header class="clock-mode-header" @keydown.esc.stop="modeMenu=false" :data-tauri-drag-region="drag"><button v-if="widget.size === 'small'" class="clock-mode-picker" aria-label="切换时钟模式" aria-haspopup="menu" :aria-expanded="modeMenu" @click="modeMenu=!modeMenu"><AppIcon :name="clockModes.find(m=>m.value===tools.mode)?.icon??'clock'" :size="14" />{{ clockModes.find(m=>m.value===tools.mode)?.label }}<AppIcon name="chevron-down" :size="11" /></button><nav v-else class="clock-mode-tabs" aria-label="时钟模式"><button v-for="mode in clockModes" :key="mode.value" class="widget-icon-button" :class="{selected:tools.mode===mode.value}" :aria-label="mode.label" :aria-pressed="tools.mode===mode.value" @click="chooseMode(mode.value)"><AppIcon :name="mode.icon" :size="16" /></button></nav><span v-if="runningHint" class="clock-running-dot" :data-tooltip="runningHint" :aria-label="runningHint"></span><span v-if="widget.size !== 'small'" class="extra-caption">{{ clockModes.find(m=>m.value===tools.mode)?.label }}</span></header>
    <div v-if="modeMenu" class="clock-mode-menu" role="menu" @keydown.esc.stop="modeMenu=false"><button v-for="mode in clockModes" :key="mode.value" role="menuitem" @click="chooseMode(mode.value)"><AppIcon :name="mode.icon" :size="14" />{{ mode.label }}</button></div>
    <ClockToolsPanel v-if="tools.mode !== 'clock'" :key="tools.mode" :size="widget.size" />
    <div v-else class="clock-display">
    <div v-if="widget.size !== 'large'" class="clock-digital" :class="{ 'clock-split': widget.size === 'medium' }">
      <div class="clock-local"><span class="extra-eyebrow">本地时间</span><strong class="clock-time" :class="{ 'with-seconds': settings.showSeconds, 'hour-12': settings.hour12 }">{{ localTime }}</strong><span class="extra-caption">{{ dateLabel }}</span></div>
      <div v-if="widget.size === 'medium'" class="clock-cities">
        <div v-for="city in cities" :key="city.timeZone" class="clock-city"><div><span>{{ city.name }}</span><small><AppIcon :name="city.daylight ? 'sun' : 'moon'" :size="11" />{{ city.difference }}</small></div><strong>{{ city.display }}</strong></div>
        <p v-if="!cities.length" class="extra-empty-caption">在偏好设置中添加城市<br />看看远方的时间</p>
      </div>
    </div>
    <template v-else>
      <div class="clock-face" role="img" :aria-label="`本地时间 ${localTime}`">
        <i v-for="n in 12" :key="n" class="clock-tick" :style="{ transform: `rotate(${n * 30}deg)` }"></i>
        <span class="clock-face-label twelve">12</span><span class="clock-face-label three">3</span><span class="clock-face-label six">6</span><span class="clock-face-label nine">9</span>
        <i class="clock-hand hour" :style="{ transform: `rotate(${hands.hour}deg)` }"></i><i class="clock-hand minute" :style="{ transform: `rotate(${hands.minute}deg)` }"></i><i v-if="settings.showSeconds" class="clock-hand second" :style="{ transform: `rotate(${hands.second}deg)` }"></i><i class="clock-pin"></i>
      </div>
      <span class="clock-large-date extra-caption">{{ dateLabel }}</span>
      <div class="clock-cities large"><div v-for="city in cities" :key="city.timeZone" class="clock-city"><div><span>{{ city.name }}</span><small><AppIcon :name="city.daylight ? 'sun' : 'moon'" :size="11" />{{ city.difference }}</small></div><strong>{{ city.display }}</strong></div><p v-if="!cities.length" class="extra-empty-caption">此刻，也是新的一刻。</p></div>
    </template>
    </div>
    <p v-if="modeError" class="extra-notice" role="alert">{{ modeError }}</p>
    </template>
    <template #context-actions="{dismiss}"><button v-for="mode in clockModes" :key="mode.value" role="menuitem" @click="chooseMode(mode.value).then(dismiss)"><AppIcon :name="mode.icon" :size="14" />{{ mode.label }}</button><div class="context-divider"></div></template>
  </WidgetFrame>
</template>
