<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref, watch } from 'vue';
import type { HabitItem, HabitRecord } from '../types';
const props = defineProps<{ habit: HabitItem; record?: HabitRecord }>();
const root = ref<HTMLElement | null>(null), visible = ref(false), fraction = ref(0), motionKey = ref(0);
const moodEmoji = computed(() => ({ 开心: '😊', 平静: '😌', 疲惫: '😴', 低落: '😔' }[props.record?.mood ?? ''] ?? '😶'));
let observer: IntersectionObserver | undefined, frame = 0;
let reduced: MediaQueryList | undefined;
function animate() {
  cancelAnimationFrame(frame);
  fraction.value = reduced?.matches ? 1 : 0;
  motionKey.value++;
  if (reduced?.matches) return;
  const start = performance.now();
  const tick = (time: number) => {
    const progress = Math.min(1, (time - start) / 900);
    fraction.value = 1 - (1 - progress) ** 3;
    if (progress < 1) frame = requestAnimationFrame(tick);
  };
  frame = requestAnimationFrame(tick);
}
function motionPreference() { if (reduced?.matches) { cancelAnimationFrame(frame); fraction.value = 1; } }
watch(() => [props.record?.count, props.record?.rating, props.record?.mood, props.record?.result], () => { if (visible.value) animate(); });
onMounted(() => {
  reduced = window.matchMedia('(prefers-reduced-motion: reduce)');
  reduced.addEventListener('change', motionPreference);
  observer = new IntersectionObserver(entries => {
    const inView = entries[0]?.isIntersecting ?? false;
    if (inView && !visible.value) { visible.value = true; animate(); }
    else if (!inView) { visible.value = false; cancelAnimationFrame(frame); fraction.value = 0; }
  }, { threshold: .2 });
  if (root.value) observer.observe(root.value);
});
onBeforeUnmount(() => { observer?.disconnect(); cancelAnimationFrame(frame); reduced?.removeEventListener('change', motionPreference); });
</script>
<template>
  <div ref="root" class="habit-record-visuals" :class="{ 'is-visible': visible }">
    <div class="habit-record-ring"><div class="habit-ring" role="img" :aria-label="`打卡 ${record?.count ?? 0} / ${record?.target ?? habit.dailyTarget} 次`" :style="{ '--ring-progress': `${Math.min(100, (record?.count ?? 0) / (record?.target ?? habit.dailyTarget) * 100) * fraction}%`, '--ring-color': habit.color }"><strong>{{ record?.count ?? 0 }}<small>/ {{ record?.target ?? habit.dailyTarget }}</small></strong></div><span>打卡进度</span></div>
    <div v-if="habit.trackRating || record?.rating != null" class="habit-record-ring"><div class="habit-ring" role="img" :aria-label="`评分 ${record?.rating ?? '未记录'} / 5`" :style="{ '--ring-progress': `${(record?.rating ?? 0) * 20 * fraction}%`, '--ring-color': '#ecb547' }"><strong>{{ record?.rating ?? '—' }}<small>/ 5</small></strong></div><span>给自己的评分</span></div>
    <div v-if="habit.trackMood || record?.mood" class="habit-record-ring"><div class="habit-mood-display"><span :key="motionKey" class="habit-mood-emoji">{{ moodEmoji }}</span></div><span>{{ record?.mood ?? '未记录心情' }}</span></div>
    <div v-if="habit.trackResult || record?.result != null" class="habit-record-ring"><div class="habit-result-display">{{ record?.result ?? '—' }}</div><span>数值成绩</span></div>
  </div>
</template>
