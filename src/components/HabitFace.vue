<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { HabitItem, HabitRecord, HabitSettings, WidgetSize } from "../types";
import { cardForeground, isComplete, nextIncomplete, recordFor, scheduled, selectedHabits, todayHabits, weekDates } from "../lib/habits";
import WidgetPageControls from "./WidgetPageControls.vue";
const props = defineProps<{ habits: HabitItem[]; records: HabitRecord[]; settings: HabitSettings; size: WidgetSize; today: string; monday: boolean; busy?: boolean; drag?: string; preview?: boolean }>();
const emit = defineEmits<{ check: [habit: HabitItem]; manage: [] }>();
const all = computed(() => selectedHabits(props.habits, props.settings));
const todayItems = computed(() => todayHabits(props.habits, props.settings, props.today));
const current = computed(() => nextIncomplete(todayItems.value, props.records, props.today));
const completed = computed(() => todayItems.value.filter(h => isComplete(recordFor(props.records, h.id, props.today))).length);
const week = computed(() => weekDates(props.today, props.monday));
const weekLabels = computed(() => props.monday ? ["一", "二", "三", "四", "五", "六", "日"] : ["日", "一", "二", "三", "四", "五", "六"]);
const page = ref(0);
const capacity = computed(() => props.settings.style === "report" ? (props.size === "large" ? 10 : 4) : props.size === "small" ? 3 : props.size === "medium" ? 6 : 12);
const pages = computed(() => Math.max(1, Math.ceil(all.value.length / capacity.value)));
const visible = computed(() => all.value.slice(page.value * capacity.value, (page.value + 1) * capacity.value));
watch(() => [props.settings.style, props.size, props.today], () => { page.value = 0; });
watch(pages, n => { page.value = Math.min(page.value, n - 1); });
function count(h: HabitItem, day = props.today) { return recordFor(props.records, h.id, day)?.count ?? 0; }
function target(h: HabitItem, day = props.today) { return recordFor(props.records, h.id, day)?.target ?? h.dailyTarget; }
function color(h: HabitItem) { return { '--habit-color': h.color, '--habit-ink': cardForeground(h.color) }; }
function cellClass(h: HabitItem, day: string) { return { done: isComplete(recordFor(props.records, h.id, day)), partial: count(h, day) > 0 && !isComplete(recordFor(props.records, h.id, day)), rest: !scheduled(h, day), future: day > props.today, today: day === props.today }; }
function status(h: HabitItem, day: string) { return `${h.title} · ${day} · ${!scheduled(h, day) ? '非执行日' : day > props.today ? '尚未到来' : `${count(h, day)}/${target(h, day)} 次`}`; }
</script>

<template>
  <section class="habit-face" :class="[`habit-${settings.style}`, `habit-size-${size}`, { 'habit-preview': preview }]">
    <template v-if="settings.style === 'card'">
      <Transition name="habit-swap" mode="out-in">
        <article v-if="current" :key="`${current.id}-${today}`" class="habit-hero" :style="color(current)">
          <div class="habit-hero-copy" :data-tauri-drag-region="drag"><h1 :data-tooltip="current.title">{{ current.title }}</h1><p>{{ current.encouragement || '给今天的自己一点进步。' }}</p></div>
          <div v-if="size !== 'small'" class="habit-hero-week"><span>本周坚持</span><div><i v-for="(day, i) in week" :key="day" :class="cellClass(current, day)" :data-tooltip="status(current, day)">{{ isComplete(recordFor(records, current.id, day)) ? '✓' : weekLabels[i] }}</i></div></div>
          <footer><span class="habit-emoji" :data-tauri-drag-region="drag">{{ current.icon }}</span><span class="habit-hero-progress">{{ target(current) > 1 ? `${count(current)} / ${target(current)}` : `${completed + 1} / ${todayItems.length}` }}</span><button class="habit-check" :disabled="busy || preview" :aria-label="`为${current.title}打卡，当前 ${count(current)}/${target(current)} 次`" @click="emit('check', current)">{{ count(current) ? `${count(current)}/${target(current)}` : '' }}</button></footer>
        </article>
        <div v-else :key="all.length ? 'finished' : 'empty'" class="habit-celebration" :data-tauri-drag-region="drag"><span>{{ todayItems.length ? '🎉' : all.length ? '🌿' : '🌱' }}</span><strong>{{ todayItems.length ? '今日全部完成' : all.length ? '今天休息一下' : '养成一个好习惯' }}</strong><p>{{ todayItems.length ? `完成 ${completed} 个习惯，明天继续。` : all.length ? '今天没有需要打卡的习惯' : '在偏好设置中添加你的第一个习惯' }}</p><button v-if="!all.length" class="widget-text-button" :disabled="preview" @click="emit('manage')">添加习惯</button></div>
      </Transition>
    </template>
    <template v-else-if="all.length">
      <header v-if="settings.style === 'report' || size === 'large'" class="habit-face-header" :data-tauri-drag-region="drag"><strong>{{ settings.style === 'report' ? '本周成绩单' : '我的习惯' }}</strong><span>{{ settings.style === 'report' ? `${week[0]!.slice(5).replace('-', '/')} – ${week[6]!.slice(5).replace('-', '/')}` : `${completed}/${todayItems.length}` }}</span></header>
      <div v-if="settings.style === 'list'" class="habit-tile-grid">
        <article v-for="h in visible" :key="h.id" class="habit-tile" :class="{ 'is-done': isComplete(recordFor(records, h.id, today)), 'is-rest': !scheduled(h, today) }" :style="color(h)">
          <span class="habit-tile-icon" :data-tauri-drag-region="drag">{{ h.icon }}</span><div :data-tauri-drag-region="drag"><strong :data-tooltip="h.title">{{ h.title }}</strong><small v-if="!scheduled(h, today)">{{ today < h.startDate ? '尚未开始' : '今日休息' }}</small><small v-else-if="target(h) > 1">{{ count(h) }} / {{ target(h) }} 次</small></div>
          <button class="habit-check" :disabled="busy || preview || !scheduled(h, today) || isComplete(recordFor(records, h.id, today))" :aria-label="status(h, today)" @click="emit('check', h)">{{ isComplete(recordFor(records, h.id, today)) ? '✓' : count(h) || '' }}</button>
        </article>
        <i v-for="n in capacity - visible.length" :key="`empty-${n}`" class="habit-tile-placeholder" aria-hidden="true"></i>
      </div>
      <div v-else class="habit-report">
        <div class="habit-report-head"><span></span><span v-for="(label, i) in weekLabels" :key="label" :class="{ 'is-today': week[i] === today }">{{ label }}</span></div>
        <div v-for="h in visible" :key="h.id" class="habit-report-row"><span class="habit-report-name" :data-tooltip="h.title" :data-tauri-drag-region="drag"><b>{{ h.icon }}</b><strong v-if="size !== 'small'">{{ h.title }}</strong></span><button v-for="day in week" :key="day" class="habit-day-cell" :style="color(h)" :class="cellClass(h, day)" :data-tooltip="status(h, day)" :aria-label="status(h, day)" :disabled="preview || busy || day !== today || !scheduled(h, day) || isComplete(recordFor(records, h.id, day))" @click="emit('check', h)">{{ isComplete(recordFor(records, h.id, day)) ? '✓' : count(h, day) ? count(h, day) : !scheduled(h, day) ? '·' : '' }}</button></div>
      </div>
      <WidgetPageControls :page="page" :count="pages" label="习惯" :editing="busy || preview" @move="page = Math.max(0, Math.min(pages - 1, page + $event))" />
    </template>
    <div v-else class="habit-celebration" :data-tauri-drag-region="drag"><span>🌱</span><strong>还没有显示的习惯</strong><p>在偏好设置中添加或选择习惯</p><button class="widget-text-button" :disabled="preview" @click="emit('manage')">管理习惯</button></div>
  </section>
</template>
