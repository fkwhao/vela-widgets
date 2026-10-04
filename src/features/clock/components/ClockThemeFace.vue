<script setup lang="ts">
import { computed } from 'vue';
import AnalogClock from './AnalogClock.vue';
import ClockThemeDial from './ClockThemeDial.vue';
import { clockCityTime, defaultClockCities } from '../clockThemes';
import type { ClockSettings, ClockTheme, WidgetSize } from '../../../shared/types';
const props = defineProps<{ now: Date; settings: ClockSettings; theme?: ClockTheme; size: WidgetSize }>();
const theme = computed(() => props.theme ?? props.settings.theme);
const timeParts = computed(() => new Intl.DateTimeFormat('en-GB', { hour: '2-digit', minute: '2-digit', hour12: props.settings.hour12 }).formatToParts(props.now));
const time = computed(() => timeParts.value.filter(p=>p.type!=='dayPeriod').map(p=>p.value).join('').trim());
const meridiem = computed(() => timeParts.value.find(p=>p.type==='dayPeriod')?.value.toUpperCase());
const seconds = computed(() => String(props.now.getSeconds()).padStart(2,'0'));
const date = computed(() => new Intl.DateTimeFormat('zh-CN', { month:'long',day:'numeric',weekday:'long' }).format(props.now));
const cities = computed(() => (props.settings.cities.length ? props.settings.cities : props.theme ? defaultClockCities : []).slice(0,4).map(c => clockCityTime(props.now,c)));
const ticks = Array.from({length:60},(_,i)=>{
  const a=i*Math.PI/30, x=Math.sin(a), y=-Math.cos(a);
  const radius=88/Math.pow(Math.abs(x)**5+Math.abs(y)**5,1/5);
  return { x1:100+x*radius,y1:100+y*radius,x2:100+x*(radius-6),y2:100+y*(radius-6) };
});
</script>
<template>
  <div class="clock-theme-face" :class="[`theme-${theme}`,`theme-size-${size}`]">
    <div v-if="theme==='default'" class="theme-default-face"><AnalogClock :now="now" :square="size==='small'" :label="`本地时间 ${time}`" /><div v-if="size!=='small'" class="theme-default-cities"><span v-for="city in settings.cities" :key="city.timeZone">{{ city.name }} {{ new Intl.DateTimeFormat('en-GB',{timeZone:city.timeZone,hour:'2-digit',minute:'2-digit'}).format(now) }}</span><small v-if="!settings.cities.length">{{ date }}</small></div></div>
    <template v-else-if="theme==='digital'">
      <svg class="theme-digital-face" viewBox="0 0 200 200" preserveAspectRatio="none" aria-hidden="true"><rect width="200" height="200" rx="25" fill="#fff" /><line v-for="(tick,i) in ticks" :key="i" v-bind="tick" :class="{ passed:i<=now.getSeconds() }" /></svg>
      <div class="theme-digital-readout" role="img" :aria-label="`本地时间 ${time}${settings.showSeconds ? ':'+seconds : ''}`"><strong>{{ time }}</strong><span v-if="meridiem" class="theme-digital-meridiem">{{ meridiem }}</span><span v-if="settings.showSeconds">{{ seconds }}</span><small v-if="size!=='small'">{{ date }}</small></div>
    </template>
    <template v-else-if="theme==='classic'">
      <div class="theme-classic-dial"><ClockThemeDial :now="now" :label="`本地时间 ${time}`" /></div>
      <span v-if="size!=='small'" class="theme-classic-date">{{ date }}</span>
    </template>
    <p v-else-if="!cities.length" class="theme-world-empty">在偏好设置中选择地区</p>
    <div v-else class="theme-world-grid" :class="{ row:size==='medium' }">
      <div v-for="city in cities" :key="city.timeZone" class="theme-world-city">
        <ClockThemeDial :now="now" :time-zone="city.timeZone" :dark="!city.daylight" :city="size==='small' ? city.name : undefined" :label="`${city.name} ${city.hour}:${String(city.minute).padStart(2,'0')}，${city.dayLabel}，${city.difference}`" />
        <div v-if="size!=='small'" class="theme-world-caption"><strong>{{ city.name }}</strong><span>{{ city.dayLabel }}</span><small>{{ city.difference }}</small></div>
      </div>
    </div>
  </div>
</template>
