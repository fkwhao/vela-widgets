<script setup lang="ts">
import { computed } from 'vue';
import { clockZoneTime } from '../lib/clockThemes';
const props = defineProps<{ now: Date; timeZone?: string; dark?: boolean; city?: string; label: string }>();
const time = computed(() => clockZoneTime(props.now, props.timeZone));
const hands = computed(() => ({ hour: time.value.hour % 12 * 30 + time.value.minute / 2 + time.value.second / 120, minute: time.value.minute * 6 + time.value.second / 10, second: time.value.second * 6 }));
</script>
<template>
  <svg viewBox="0 0 200 200" class="theme-clock-dial" :class="{ 'night-dial': dark, 'city-dial': city }" role="img" :aria-label="label">
    <circle cx="100" cy="100" r="98" class="theme-dial-face" />
    <g v-if="!city" class="theme-dial-ticks"><line v-for="n in 60" :key="n" x1="100" :y1="n % 5 === 0 ? 8 : 9" x2="100" :y2="n % 5 === 0 ? 15 : 13" :transform="`rotate(${n * 6} 100 100)`" :class="{ major: n % 5 === 0 }" /></g>
    <text v-for="(num,i) in [12,1,2,3,4,5,6,7,8,9,10,11]" :key="num" :x="100 + (city ? 81 : 71) * Math.sin(i * Math.PI / 6)" :y="100 - (city ? 81 : 71) * Math.cos(i * Math.PI / 6)" class="theme-dial-number">{{ num }}</text>
    <text v-if="city" x="100" y="66" class="theme-dial-city">{{ city }}</text>
    <g :transform="`rotate(${hands.hour} 100 100)`"><line x1="100" y1="109" x2="100" y2="55" class="theme-dial-hour" /></g>
    <g :transform="`rotate(${hands.minute} 100 100)`"><line x1="100" y1="113" x2="100" y2="17" class="theme-dial-minute" /></g>
    <circle cx="100" cy="100" r="5" class="theme-dial-hub" />
    <g :transform="`rotate(${hands.second} 100 100)`" class="theme-dial-second"><line x1="100" y1="113" x2="100" y2="8" /><circle cx="100" cy="100" r="3" /></g>
    <circle cx="100" cy="100" r="1.5" class="theme-dial-face" />
  </svg>
</template>
