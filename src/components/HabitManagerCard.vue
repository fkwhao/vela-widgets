<script setup lang="ts">
import { nextTick, ref, watch } from 'vue';
import type { HabitItem } from '../types';
import { cardForeground } from '../lib/habits';
const props = defineProps<{ habit: HabitItem; stats: { total: number; month: number; current: number }; busy: boolean; detail?: boolean }>();
const emit = defineEmits<{ open: []; toggle: [] }>();
const root = ref<HTMLElement | null>(null);
let restoreSwitchFocus = false;
function toggle() { restoreSwitchFocus = root.value?.contains(document.activeElement) ?? false; emit('toggle'); }
watch(() => props.habit.archived, async archived => { if (!restoreSwitchFocus) return; await nextTick(); root.value?.querySelector<HTMLElement>(archived ? 'header [role="switch"]' : '.habit-card-status [role="switch"]')?.focus(); restoreSwitchFocus = false; });
</script>
<template>
  <article ref="root" class="habit-manager-card" :class="{ archived: habit.archived, 'habit-detail-card': detail }">
    <header :style="{ background: habit.color, color: cardForeground(habit.color) }">
      <button class="habit-card-open" :aria-label="`查看 ${habit.title} 的统计`" @click="$emit('open')"><span class="habit-card-emoji">{{ habit.icon }}</span><div><h3>{{ habit.title }}<small v-if="habit.archived"> · {{ stats.total }} 天</small></h3><p>{{ habit.archived ? '习惯已暂停' : habit.encouragement }}</p></div><b v-if="!habit.archived">{{ stats.total }}<small>天</small></b></button>
      <button v-if="habit.archived" class="toggle-switch" role="switch" :aria-label="`开启 ${habit.title}`" :aria-checked="false" :disabled="busy" @click="toggle"><span></span></button>
    </header>
    <div class="habit-card-expansion" :class="{ collapsed: habit.archived }" :inert="habit.archived" :aria-hidden="habit.archived"><div class="habit-card-expansion-inner">
      <button class="habit-card-body" :aria-label="`查看 ${habit.title} 的统计记录`" @click="$emit('open')"><div class="habit-manager-metrics"><div><span>本月完成天数</span><strong>{{ stats.month }}<small>天</small></strong></div><div><span>连续天数</span><strong>{{ stats.current }}<small>天</small></strong></div><div><span>坚持目标</span><strong>{{ habit.goalDays ?? '永远' }}<small v-if="habit.goalDays">天</small></strong></div></div></button>
      <footer class="habit-card-status"><div><strong>习惯进行中</strong><small>开始于 {{ habit.startDate }}</small></div><button class="toggle-switch on" role="switch" :aria-label="`暂停 ${habit.title}`" :aria-checked="true" :disabled="busy" @click="toggle"><span></span></button></footer>
      <slot />
    </div></div>
  </article>
</template>
