<script setup lang="ts">
import { computed } from "vue";
const props = defineProps<{ now: Date; label: string; square?: boolean }>();
const hands = computed(() => ({
  hour: (props.now.getHours() % 12) * 30 + props.now.getMinutes() / 2 + props.now.getSeconds() / 120,
  minute: props.now.getMinutes() * 6 + props.now.getSeconds() / 10,
  second: props.now.getSeconds() * 6,
}));
</script>
<template>
  <svg viewBox="0 0 200 200" class="analog-clock" :class="{ square }" role="img" :aria-label="label">
    <rect v-if="square" width="200" height="200" class="analog-dial" /><template v-else><circle cx="100" cy="100" r="98" class="analog-rim" /><circle cx="100" cy="100" r="96.5" class="analog-dial" /></template>
    <g class="analog-ticks"><line v-for="n in 60" :key="n" x1="100" :y1="n % 5 === 0 ? 9 : 10" x2="100" :y2="n % 5 === 0 ? 18 : 13.5" :transform="`rotate(${n * 6} 100 100)`" :class="{ major: n % 5 === 0 }" /></g>
    <text v-for="(num, i) in [12,1,2,3,4,5,6,7,8,9,10,11]" :key="num" :x="100 + (square ? 76 : 68) * Math.sin(i * Math.PI / 6)" :y="100 - (square ? 76 : 68) * Math.cos(i * Math.PI / 6)" class="analog-number">{{ num }}</text>
    <g :transform="`rotate(${hands.hour} 100 100)`"><line x1="100" y1="109" x2="100" y2="58" class="analog-hour" /></g>
    <g :transform="`rotate(${hands.minute} 100 100)`"><line x1="100" y1="112" x2="100" y2="28" class="analog-minute" /></g>
    <circle cx="100" cy="100" r="5.2" class="analog-hub" />
    <g :transform="`rotate(${hands.second} 100 100)`" class="analog-second"><line x1="100" y1="121" x2="100" y2="20" /><circle cx="100" cy="100" r="3.3" /></g>
    <circle cx="100" cy="100" r="1.35" class="analog-cap" />
  </svg>
</template>
