<script setup lang="ts">
import { computed, toRef } from "vue";
import WidgetFrame from "./WidgetFrame.vue";
import AppIcon from "./AppIcon.vue";
import { snapshot } from "../lib/store";
import { useWidgetClock } from "../lib/useWidgetClock";
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
  <WidgetFrame kind="clock" v-slot="{ widget, drag }">
    <header v-if="widget.size === 'large'" class="extra-header" :data-tauri-drag-region="drag"><h1>时钟</h1><span class="extra-caption">本地时间</span></header>
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
  </WidgetFrame>
</template>
