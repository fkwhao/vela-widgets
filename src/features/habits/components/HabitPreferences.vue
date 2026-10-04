<script setup lang="ts">
import { computed, nextTick, ref, watch, toRaw } from "vue";
import HabitFace from "./HabitFace.vue";
import HabitManagerCard from "./HabitManagerCard.vue";
import HabitRecordVisuals from "./HabitRecordVisuals.vue";
import VelaDatePicker from "../../../shared/ui/VelaDatePicker.vue";
import VelaSelect from "../../../shared/ui/VelaSelect.vue";
import VelaTimeInput from "../../../shared/ui/VelaTimeInput.vue";
import AppIcon from "../../../shared/ui/AppIcon.vue";
import { adjustHabitRecord, deleteHabit, reorderHabits, saveHabit, setHabitSettings, setWidgetEnabled, setWidgetSize, setWidgetLocked, setWidgetLayer, snapshot } from "../../../app/store";
import { widgetSizeOptions } from "../../../shared/types";
import { annualWeeklyCheckins, dateKey, encouragements, habitColors, habitIcons, habitStats, isComplete, monthDates, parseHabitResult, recordFor, scheduled, shiftDate, validateHabit, weekDates } from "../habits";
import { useWidgetClock } from "../../../shared/composables/useWidgetClock";
import type { HabitItem, HabitStyle, WidgetSize } from "../../../shared/types";
const now = useWidgetClock(undefined, true);
const today = computed(() => dateKey(now.value));
const tab = ref<'habits' | 'stats' | 'widget'>('habits');
const error = ref(''), busy = ref(false);
const focusedId = ref<number | null>(null);
const focused = computed(() => all.value.find(h => h.id === focusedId.value));
const detailMode = ref<'month' | 'year'>('month');
const checkin = ref<{ habit: HabitItem; day: string } | null>(null);
const checkMood = ref(''), checkRating = ref('');
const checkResult = ref<string | number>('');
const checkDialog = ref<HTMLFormElement | null>(null);
let checkTrigger: HTMLElement | null = null;
watch(checkin, async value => { await nextTick(); if (value) checkDialog.value?.querySelector<HTMLElement>('button, input')?.focus(); else checkTrigger?.focus(); });
function trapCheckFocus(event: KeyboardEvent) { if (event.key !== 'Tab') return; const nodes = Array.from(checkDialog.value?.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled)') ?? []); const first = nodes[0], last = nodes.at(-1); if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); } else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); } }
const moods = [{ value: '开心', emoji: '😊' }, { value: '平静', emoji: '😌' }, { value: '疲惫', emoji: '😴' }, { value: '低落', emoji: '😔' }];
const draft = ref<HabitItem | null>(null);
const deleting = ref(false);
const nameInput = ref<HTMLInputElement | null>(null);
const all = computed(() => [...snapshot.value.habits].sort((a, b) => a.sortOrder - b.sortOrder || a.id - b.id));
const habits = all;
const stats = computed(() => new Map(all.value.map(h => [h.id, habitStats(h, snapshot.value.habitRecords, today.value)])));
const filter = ref('all');
const statHabits = computed(() => all.value.filter(h => focusedId.value !== null ? h.id === focusedId.value : filter.value === 'all' || String(h.id) === filter.value));
const statRecords = computed(() => snapshot.value.habitRecords.filter(r => statHabits.value.some(h => h.id === r.habitId) && r.date <= today.value));
const summary = computed(() => ({ days: new Set(statRecords.value.filter(r => r.count > 0).map(r => r.date)).size, done: statRecords.value.filter(isComplete).length, longest: Math.max(0, ...statHabits.value.map(h => stats.value.get(h.id)!.longest)) }));
const month = ref(today.value.slice(0, 7));
const monthKey = computed(() => `${month.value}-01`);
const days = computed(() => monthDates(monthKey.value));
const monthOffset = computed(() => { const d = new Date(`${monthKey.value}T12:00:00`).getDay(); return snapshot.value.settings.weekStartsMonday ? (d + 6) % 7 : d; });
const weekKey = ref(today.value);
const week = computed(() => weekDates(weekKey.value, snapshot.value.settings.weekStartsMonday));
const labels = computed(() => snapshot.value.settings.weekStartsMonday ? ['一','二','三','四','五','六','日'] : ['日','一','二','三','四','五','六']);
const selectedDay = ref(today.value);
const selectedHabit = ref<number | null>(null);
const detailHabit = computed(() => statHabits.value.find(h => h.id === selectedHabit.value) ?? statHabits.value[0]);
const detail = computed(() => detailHabit.value ? recordFor(snapshot.value.habitRecords, detailHabit.value.id, selectedDay.value) : undefined);
const previewSize = ref<WidgetSize>('small');
const styles: { value: HabitStyle; label: string; description: string }[] = [{ value: 'card', label: '习惯卡片', description: '专注一个习惯，达标后自动切换' }, { value: 'list', label: '习惯列表', description: '多个习惯并排显示，直接点击打卡' }, { value: 'report', label: '本周成绩单', description: '七天记录一目了然，今天可直接打卡' }];
const iconOpen = ref(false);
const milestones = computed(() => [1, 3, 7, 14, 21, 30, 60, 100, 365].map(days => ({ days, achieved: statHabits.value.filter(h => stats.value.get(h.id)!.longest >= days) })));
const heatYear = ref(new Date().getFullYear());
const yearMonths = computed(() => Array.from({ length: 12 }, (_, i) => monthDates(`${heatYear.value}-${String(i + 1).padStart(2, '0')}-01`)));
const chartYear = ref(new Date().getFullYear());
const chartScroll = ref<HTMLDivElement | null>(null);
const weeklyCounts = computed(() => annualWeeklyCheckins(statRecords.value, chartYear.value, snapshot.value.settings.weekStartsMonday, today.value));
const maxWeek = computed(() => Math.max(4, ...weeklyCounts.value.map(w => w.count)));
const chartArea = ref<HTMLElement | null>(null);
const chartHover = ref<{ index: number; x: number; y: number } | null>(null);
function showChartPoint(event: Event, index: number) {
  const area = chartArea.value?.getBoundingClientRect(), dot = (event.currentTarget as SVGElement).getBoundingClientRect();
  if (!area) return;
  chartHover.value = { index, x: Math.max(8, Math.min(area.width - 188, dot.left + dot.width / 2 - area.left - 90)), y: Math.max(4, dot.top - area.top - 76) };
}
watch([chartYear, focusedId, tab], () => { chartHover.value = null; });
const chartWidth = computed(() => Math.max(640, weeklyCounts.value.length * 74));
const chartPoints = computed(() => weeklyCounts.value.map((w, i) => `${38 + i * 74},${170 - w.count / maxWeek.value * 140}`).join(' '));
watch([chartYear, tab, focusedId], async () => { await nextTick(); if (chartScroll.value) chartScroll.value.scrollLeft = chartScroll.value.scrollWidth; });
function openHabit(h: HabitItem) { focusedId.value = h.id; selectedHabit.value = h.id; selectedDay.value = today.value; month.value = today.value.slice(0, 7); detailMode.value = 'month'; tab.value = 'stats'; }
function cancelEdit() { draft.value = null; deleting.value = false; if (focused.value) tab.value = 'stats'; }
function changeTab(value: 'habits' | 'stats' | 'widget') { tab.value = value; focusedId.value = null; draft.value = null; }
function toggleHabit(h: HabitItem) { void run(() => saveHabit({ ...h, archived: !h.archived })); }
function startCheck(h: HabitItem, day = today.value) {
  if (h.trackMood || h.trackRating || h.trackResult) { checkTrigger = document.activeElement as HTMLElement; checkin.value = { habit: h, day }; checkMood.value = ''; checkRating.value = ''; checkResult.value = ''; error.value = ''; }
  else adjust(1, h, day);
}
async function confirmCheck() {
  const pending = checkin.value; if (!pending) return;
  const h = pending.habit;
  await run(async () => {
    const result = h.trackResult ? parseHabitResult(checkResult.value) : null;
    if ((h.trackMood && !checkMood.value) || (h.trackRating && !checkRating.value) || (h.trackResult && result === null)) throw new Error('请填写本次打卡的记录。');
    await adjustHabitRecord({ habitId: h.id, date: pending.day, delta: 1, captureMetadata: true, mood: h.trackMood ? checkMood.value : null, rating: h.trackRating ? Number(checkRating.value) : null, result });
    checkin.value = null;
  });
}
function completedOn(day: string) { return statRecords.value.filter(r => r.date === day && isComplete(r)).length; }
function partialOn(day: string) { return statRecords.value.some(r => r.date === day && r.count > 0 && !isComplete(r)); }
async function run(action: () => Promise<void>) { if (busy.value) return; busy.value = true; error.value = ''; try { await action(); } catch (e) { error.value = typeof e === 'string' ? e : e instanceof Error ? e.message : '没有保存成功，请重试。'; } finally { busy.value = false; } }
async function edit(h?: HabitItem) {
  draft.value = h ? structuredClone(toRaw(h)) : { id: 0, title: '', encouragement: encouragements[0]!, icon: '💎', color: habitColors[0]!, startDate: today.value, weekdays: [0,1,2,3,4,5,6], dailyTarget: 1, goalDays: null, archived: false, sortOrder: all.value.length, trackMood: false, trackRating: false, trackResult: false, reminderTime: null, lastRemindedDate: null };
  iconOpen.value = false; deleting.value = false; error.value = ''; await nextTick(); nameInput.value?.focus();
}
async function submit() { if (!draft.value) return; const invalid = validateHabit(draft.value); if (invalid) { error.value = invalid; return; } const item = structuredClone(toRaw(draft.value)); await run(async () => { await saveHabit(item); draft.value = null; if (focusedId.value === item.id) tab.value = 'stats'; }); }
function toggleDay(day: number) { if (!draft.value) return; draft.value.weekdays = draft.value.weekdays.includes(day) ? draft.value.weekdays.filter(d => d !== day) : [...draft.value.weekdays, day]; }
function reorder(id: number, direction: number) { const ids = all.value.map(h => h.id), index = ids.indexOf(id), next = index + direction; if (next < 0 || next >= ids.length) return; [ids[index], ids[next]] = [ids[next]!, ids[index]!]; void run(() => reorderHabits(ids)); }
function moveMonth(step: number) { const d = new Date(`${monthKey.value}T12:00:00`); d.setMonth(d.getMonth() + step); if (d.getFullYear() >= 1900 && d.getFullYear() <= 2100) month.value = dateKey(d).slice(0, 7); }
function selectDay(day: string, id?: number) { if (id !== undefined && !focused.value) { const h = all.value.find(h => h.id === id); if (h) openHabit(h); } selectedDay.value = day; month.value = day.slice(0, 7); if (id !== undefined) selectedHabit.value = id; }
function adjust(delta: number, h = detailHabit.value, day = selectedDay.value) { if (!h) return; void run(() => adjustHabitRecord({ habitId: h.id, date: day, delta })); }
function setStyle(style: HabitStyle) { void run(() => setHabitSettings({ ...snapshot.value.settings.habit, style })); }
function selectWidgetHabit(id: number) { const settings = snapshot.value.settings.habit; const ids = settings.selectedIds ?? all.value.filter(h => !h.archived).map(h => h.id); void run(() => setHabitSettings({ ...settings, selectedIds: ids.includes(id) ? ids.filter(i => i !== id) : [...ids, id] })); }
</script>

<template>
  <section class="habit-preferences">
    <div :inert="!!checkin">
    <nav class="habit-tabs" aria-label="习惯打卡页面"><button v-for="t in ([['habits','我的习惯'],['stats','统计记录'],['widget','组件设置']] as const)" :key="t[0]" :aria-current="tab === t[0] ? 'page' : undefined" :class="{ active: tab === t[0] }" @click="changeTab(t[0])">{{ t[1] }}</button></nav>
    <p v-if="error && !checkin" class="habit-error" role="alert">{{ error }}</p>
    <template v-if="tab === 'habits'">
      <div class="habit-section-bar"><div><h2>我的习惯</h2><p>一点一滴，坚持成为更好的自己。</p></div><button class="win-button habit-primary" :disabled="busy" @click="edit()"><AppIcon name="plus" :size="14" /> 新建习惯</button></div>
      <form v-if="draft" class="habit-editor settings-card stacked" @submit.prevent="submit" @keydown.esc="cancelEdit()">
        <div class="habit-editor-heading"><span>{{ draft.id ? '编辑习惯' : '创建习惯' }}</span><button type="button" class="win-button" :disabled="busy" @click="cancelEdit()">取消</button></div>
        <label>习惯名称<input ref="nameInput" v-model="draft.title" maxlength="80" required placeholder="输入你想坚持的事情" :disabled="busy" /></label>
        <div class="habit-form-columns"><label>开始日期<VelaDatePicker v-model="draft.startDate" label="开始日期" min="1900-01-01" max="2100-12-31" variant="manager" /></label><label>每日打卡次数<input v-model.number="draft.dailyTarget" type="number" min="1" max="99" required /></label><label>坚持目标<VelaSelect :model-value="draft.goalDays === null ? 'forever' : 'days'" label="坚持目标" :options="[{ value: 'forever', label: '一直坚持' }, { value: 'days', label: '指定达标天数' }]" @update:model-value="draft.goalDays = $event === 'forever' ? null : 21" /></label><label v-if="draft.goalDays !== null">目标天数<input v-model.number="draft.goalDays" type="number" min="1" max="10000" required /></label></div>
        <fieldset><legend>时间规划</legend><div class="habit-weekday-picker"><button type="button" class="win-button" @click="draft.weekdays = [0,1,2,3,4,5,6]">每天</button><button type="button" class="win-button" @click="draft.weekdays = [1,2,3,4,5]">工作日</button><button v-for="(label, index) in ['日','一','二','三','四','五','六']" :key="label" type="button" class="habit-weekday" :aria-pressed="draft.weekdays.includes(index)" :class="{ active: draft.weekdays.includes(index) }" @click="toggleDay(index)">{{ label }}</button></div></fieldset>
        <label>写一句话鼓励自己<div class="habit-inline"><input v-model="draft.encouragement" maxlength="160" placeholder="你的鼓励语" /><button type="button" class="win-button" @click="draft.encouragement = encouragements[(encouragements.indexOf(draft.encouragement) + 1) % encouragements.length]!">换一句</button></div></label>
        <div class="habit-form-columns"><fieldset><legend>图标</legend><button type="button" class="habit-icon-choice" :aria-expanded="iconOpen" @click="iconOpen = !iconOpen">{{ draft.icon }} <small>选择图标</small></button></fieldset><fieldset><legend>卡片颜色</legend><div class="habit-color-picker"><button v-for="color in habitColors" :key="color" type="button" :style="{ background: color }" :aria-label="`选择颜色 ${color}`" :aria-pressed="draft.color === color" @click="draft.color = color">{{ draft.color === color ? '✓' : '' }}</button><input v-model="draft.color" type="color" aria-label="自定义卡片颜色" /></div></fieldset></div>
        <div v-if="iconOpen" class="habit-icon-library"><div v-for="group in habitIcons" :key="group.label"><strong>{{ group.label }}</strong><div><button v-for="icon in group.icons" :key="icon" type="button" :aria-label="`使用 ${icon} 图标`" :aria-pressed="draft.icon === icon" @click="draft.icon = icon; iconOpen = false">{{ icon }}</button></div></div><label>自定义 Emoji<input v-model="draft.icon" maxlength="16" placeholder="输入一个 Emoji" /></label></div>
        <details><summary>更多记录选项</summary><div class="habit-extra-options"><label><input v-model="draft.trackMood" type="checkbox" />记录心情</label><label><input v-model="draft.trackRating" type="checkbox" />给自己评分（1–5）</label><label><input v-model="draft.trackResult" type="checkbox" />记录数值成绩</label></div><p class="habit-muted">中控打卡时填写；点击习惯进入详情后可按日期查看。桌面打卡保持一键完成。</p></details>
        <div class="habit-reminder-field"><label><input type="checkbox" :checked="!!draft.reminderTime" @change="draft.reminderTime = draft.reminderTime ? null : '20:00'" />定时提醒</label><VelaTimeInput v-if="draft.reminderTime" :model-value="Number(draft.reminderTime.slice(0,2)) * 3600 + Number(draft.reminderTime.slice(3)) * 60" label="习惯提醒时间" @update:model-value="draft.reminderTime = `${String(Math.floor($event / 3600)).padStart(2, '0')}:${String(Math.floor($event / 60) % 60).padStart(2, '0')}`" /><p class="habit-muted">默认关闭。开启后在执行日通过 Vela 提醒窗口和提示音提醒；当日达标后不再提醒，应用需保持运行。</p></div>
        <div class="habit-editor-footer"><span class="habit-muted">次数目标对新记录生效，已有记录保留原目标。</span><button v-if="draft.id" type="button" class="habit-text-action habit-danger" @click="deleting = !deleting">删除习惯</button><button class="win-button habit-primary" type="submit" :disabled="busy">{{ busy ? '保存中…' : '完成' }}</button></div>
        <div v-if="deleting && draft.id" class="habit-delete-confirm"><p>删除“{{ draft.title }}”及全部记录？暂停可以保留历史。</p><button type="button" class="win-button" @click="deleting = false">取消</button><button type="button" class="win-button habit-danger" :disabled="busy" @click="run(async () => { await deleteHabit(draft!.id); draft = null; focusedId = null; deleting = false; })">确认删除</button></div>
      </form>
      
      <div v-if="!habits.length" class="habit-manager-empty"><span>🌱</span><h3>{{ all.length ? '没有进行中的习惯' : '从一个小习惯开始' }}</h3><p>读几页书、喝一杯水、留一点时间给自己。</p><button class="win-button" :disabled="busy" @click="edit()">创建第一个习惯</button></div>
      <div class="habit-manager-grid"><HabitManagerCard v-for="h in habits" :key="h.id" :habit="h" :stats="stats.get(h.id)!" :busy="busy" @open="openHabit(h)" @toggle="toggleHabit(h)">
        <div class="habit-manager-today"><span>{{ today < h.startDate ? '尚未开始' : !scheduled(h, today) ? '今日休息' : `今日 ${recordFor(snapshot.habitRecords, h.id, today)?.count ?? 0} / ${recordFor(snapshot.habitRecords, h.id, today)?.target ?? h.dailyTarget} 次` }}</span><button class="habit-checkin-button" :disabled="busy || !scheduled(h, today) || isComplete(recordFor(snapshot.habitRecords, h.id, today))" @click="startCheck(h)">{{ isComplete(recordFor(snapshot.habitRecords, h.id, today)) ? '已达标 ✓' : '打卡' }}</button><div class="habit-order-actions"><button class="habit-small-action" :disabled="busy || all[0]?.id === h.id" aria-label="上移习惯" @click="reorder(h.id, -1)">↑</button><button class="habit-small-action" :disabled="busy || all[all.length - 1]?.id === h.id" aria-label="下移习惯" @click="reorder(h.id, 1)">↓</button></div></div>
      </HabitManagerCard></div>
    </template>

    <template v-else-if="tab === 'stats'">
      <div v-if="!focused" class="habit-section-bar"><div><h2>我的成绩单</h2><p>每一次坚持，都留下痕迹。</p></div><VelaSelect v-model="filter" label="统计习惯" :options="[{ value: 'all', label: '全部习惯' }, ...all.map(h => ({ value: String(h.id), label: `${h.icon} ${h.title}` }))]" /></div>
      <template v-if="focused"><div class="habit-detail-heading"><button class="habit-nav-icon" aria-label="返回我的习惯" @click="changeTab('habits')"><AppIcon name="chevron-left" :size="16" /></button><h2>习惯详情</h2><button class="habit-text-action" @click="edit(focused); tab = 'habits'">✎ 编辑</button></div><HabitManagerCard :habit="focused" :stats="stats.get(focused.id)!" :busy="busy" detail @open="detailMode = 'month'" @toggle="toggleHabit(focused)" />
        <div class="habit-detail-tabs"><button :class="{ active: detailMode === 'month' }" @click="detailMode = 'month'">每月统计</button><button :class="{ active: detailMode === 'year' }" @click="detailMode = 'year'">每年统计</button></div>
      </template>
      <div v-if="!focused" class="habit-summary"><div><span>习惯数量</span><strong>{{ statHabits.length }}</strong></div><div><span>记录天数</span><strong>{{ summary.days }}</strong></div><div><span>达标次数</span><strong>{{ summary.done }}</strong></div><div><span>最长连续</span><strong>{{ summary.longest }}<small>天</small></strong></div></div>
      <section v-if="!focused" class="habit-stat-card"><div class="habit-section-bar"><h3>每周成绩单</h3><div class="habit-date-nav"><button class="habit-nav-icon" aria-label="上一周" @click="weekKey = shiftDate(weekKey, -7)"><AppIcon name="chevron-left" :size="16" /></button><span>{{ week[0]!.slice(5).replace('-', '.') }} — {{ week[6]!.slice(5).replace('-', '.') }}</span><button class="habit-nav-icon" aria-label="下一周" :disabled="week[6]! >= today" @click="weekKey = shiftDate(weekKey, 7)"><AppIcon name="chevron-right" :size="16" /></button><button class="habit-text-action" @click="weekKey = today">本周</button></div></div>
        <div class="habit-stats-week"><div class="habit-stats-week-row"><span></span><span v-for="label in labels" :key="label">{{ label }}</span></div><div v-for="h in statHabits" :key="h.id" class="habit-stats-week-row"><strong class="habit-stat-name"><span>{{ h.icon }}</span><b>{{ h.title }}</b></strong><button v-for="day in week" :key="day" :class="{ done: isComplete(recordFor(snapshot.habitRecords, h.id, day)), partial: !!recordFor(snapshot.habitRecords, h.id, day)?.count && !isComplete(recordFor(snapshot.habitRecords, h.id, day)), rest: !scheduled(h, day) }" :disabled="day > today" :aria-label="`${h.title} ${day} ${recordFor(snapshot.habitRecords, h.id, day)?.count ?? 0} 次，查看记录`" @click="selectDay(day, h.id)">{{ isComplete(recordFor(snapshot.habitRecords, h.id, day)) ? '✓' : recordFor(snapshot.habitRecords, h.id, day)?.count || (!scheduled(h, day) ? '·' : '') }}</button></div></div><p v-if="!statHabits.length" class="habit-muted">添加习惯后在这里查看成绩单。</p>
      </section>
      <section v-if="!focused || detailMode === 'month'" class="habit-stat-card" :class="{ 'habit-detail-panel': focused }"><div class="habit-section-bar"><h3>{{ month.replace('-', ' 年 ') }} 月</h3><div class="habit-date-nav"><button class="habit-nav-icon" aria-label="上个月" @click="moveMonth(-1)"><AppIcon name="chevron-left" :size="16" /></button><button class="habit-text-action" @click="month = today.slice(0, 7)">本月</button><button class="habit-nav-icon" aria-label="下个月" :disabled="month >= today.slice(0,7)" @click="moveMonth(1)"><AppIcon name="chevron-right" :size="16" /></button></div></div><div class="habit-month-calendar"><span v-for="label in labels" :key="label" class="habit-month-label">{{ label }}</span><i v-for="i in monthOffset" :key="`pad-${i}`"></i><button v-for="day in days" :key="day" :class="{ done: completedOn(day) > 0, partial: partialOn(day) && !completedOn(day), selected: selectedDay === day, today: day === today }" :disabled="day > today" :aria-label="`${day}，${completedOn(day)} 个习惯达标，查看记录`" @click="selectDay(day)"><b>{{ Number(day.slice(8)) }}</b><small>{{ completedOn(day) ? `✓ ${completedOn(day)}` : partialOn(day) ? '进行中' : '' }}</small></button></div>
        <div v-for="h in statHabits" :key="h.id" class="habit-month-strip"><span class="habit-strip-icon">{{ h.icon }}</span><div class="habit-strip-content"><div><strong>{{ h.title }}</strong><span>达标 {{ snapshot.habitRecords.filter(r => r.habitId === h.id && r.date.startsWith(month) && isComplete(r)).length }} 天</span></div><div><button v-for="day in days" :key="day" :disabled="day > today" :style="{ '--habit-color': h.color }" :class="{ done: isComplete(recordFor(snapshot.habitRecords, h.id, day)), partial: !!recordFor(snapshot.habitRecords, h.id, day)?.count && !isComplete(recordFor(snapshot.habitRecords, h.id, day)) }" :aria-label="`${h.title} ${day}，查看记录`" :data-tooltip="`${day} · ${recordFor(snapshot.habitRecords, h.id, day)?.count ?? 0} 次`" @click="selectDay(day, h.id)"></button></div></div></div>
        <p class="habit-muted">✓ 已达标 · 带点描边为部分完成 · 点号为非执行日。点击习惯的日期条进入详情，查看记录、补卡或撤销。</p>
      </section>
      <section v-if="!focused || detailMode === 'year'" class="habit-stat-card" :class="{ 'habit-detail-panel': focused }"><div class="habit-section-bar"><h3>年度坚持记录</h3><div class="habit-date-nav"><button class="habit-nav-icon" aria-label="上一年" :disabled="heatYear <= 1900" @click="heatYear--"><AppIcon name="chevron-left" :size="16" /></button><span>{{ heatYear }}</span><button class="habit-nav-icon" aria-label="下一年" :disabled="heatYear >= now.getFullYear()" @click="heatYear++"><AppIcon name="chevron-right" :size="16" /></button></div></div><div class="habit-year-heatmap"><div v-for="(dates, i) in yearMonths" :key="i"><span>{{ i + 1 }} 月</span><button v-for="day in dates" :key="day" :class="{ done: completedOn(day), partial: partialOn(day) && !completedOn(day) }" :style="completedOn(day) ? { opacity: .4 + .6 * Math.min(1, completedOn(day) / Math.max(1, statHabits.length)) } : undefined" :disabled="day > today" :aria-label="`${day}，${completedOn(day)} 个习惯达标`" :data-tooltip="`${day} · ${completedOn(day)} 个习惯达标`" @click="month = day.slice(0,7); selectDay(day)"></button></div></div></section>
      <section v-if="focused && detailHabit" class="habit-stat-card habit-record-view"><div class="habit-section-bar"><h3>{{ selectedDay }} 的记录</h3><span class="habit-muted">{{ detail ? new Date(detail.updatedAt).toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }) : '暂无打卡' }}</span></div>
        <HabitRecordVisuals :key="`${detailHabit.id}-${selectedDay}`" :habit="detailHabit" :record="detail" />
        <div class="habit-record-actions"><button class="habit-text-action" :disabled="busy || !detail?.count" @click="adjust(-1)">撤销一次</button><button class="habit-checkin-button" :disabled="busy || detailHabit.archived || selectedDay > today || (!detail?.count && !scheduled(detailHabit, selectedDay)) || isComplete(detail)" @click="startCheck(detailHabit, selectedDay)">{{ selectedDay === today ? '打卡' : '补卡' }}</button></div><p v-if="!detail?.count" class="habit-muted">选择日历中的日期查看心情、评分与成绩。</p>
      </section>
      <section class="habit-stat-card"><div class="habit-section-bar"><h3>每周打卡记录</h3><div class="habit-date-nav"><button class="habit-nav-icon" aria-label="折线图上一年" :disabled="chartYear <= 1900" @click="chartYear--"><AppIcon name="chevron-left" :size="16" /></button><span>{{ chartYear }}</span><button class="habit-nav-icon" aria-label="折线图下一年" :disabled="chartYear >= now.getFullYear()" @click="chartYear++"><AppIcon name="chevron-right" :size="16" /></button></div></div>
        <div ref="chartArea" class="habit-line-layout"><div class="habit-line-axis"><span v-for="i in [4,3,2,1,0]" :key="i">{{ Math.round(maxWeek * i / 4) }}</span></div><div ref="chartScroll" class="habit-line-scroll" @scroll="chartHover = null" tabindex="0" aria-label="每周打卡折线图，可左右滚动"><svg :width="chartWidth" height="210" role="img" :aria-label="`${chartYear} 年每周打卡次数`"><line v-for="i in [0,1,2,3,4]" :key="`grid${i}`" x1="0" :x2="chartWidth" :y1="30 + i * 35" :y2="30 + i * 35" class="habit-chart-grid" /><polyline :points="chartPoints" fill="none" class="habit-chart-line" /><g v-for="(w, i) in weeklyCounts" :key="w.start" class="habit-chart-point" tabindex="0" :aria-label="`第 ${w.index} 周，${w.start}，${w.count} 次打卡`" :aria-describedby="chartHover?.index === i ? 'habit-chart-tooltip' : undefined" @pointerenter="showChartPoint($event, i)" @pointerleave="chartHover = null" @focus="showChartPoint($event, i)" @blur="chartHover = null" @keydown.esc="chartHover = null"><rect :x="24 + i * 74" :y="156 - w.count / maxWeek * 140" width="28" height="28" fill="transparent" /><circle :cx="38 + i * 74" :cy="170 - w.count / maxWeek * 140" r="5" class="habit-chart-dot"></circle><text :x="38 + i * 74" :y="158 - w.count / maxWeek * 140" text-anchor="middle" class="habit-chart-count">{{ w.count }}</text><text :x="38 + i * 74" y="200" text-anchor="middle" class="habit-chart-label">第 {{ w.index }} 周</text></g></svg></div><Transition name="habit-chart-tip"><div v-if="chartHover && weeklyCounts[chartHover.index]" id="habit-chart-tooltip" class="habit-chart-tooltip" role="tooltip" :style="{ left: `${chartHover.x}px`, top: `${chartHover.y}px` }"><strong>第 {{ weeklyCounts[chartHover.index]!.index }} 周</strong><span>{{ weeklyCounts[chartHover.index]!.start }} — {{ shiftDate(weeklyCounts[chartHover.index]!.start, 6).slice(5) }}</span><b>{{ weeklyCounts[chartHover.index]!.count }}<small> 次打卡</small></b></div></Transition></div><p class="habit-muted">每周累计打卡次数 · 左右滑动查看全年</p>
      </section>
      <section class="habit-stat-card"><h3>坚持里程碑</h3><div class="habit-milestones"><div v-for="m in milestones" :key="m.days" :class="{ earned: m.achieved.length }"><span>{{ m.achieved.length ? '🏅' : '○' }}</span><strong>{{ m.days }} 天</strong><small>{{ m.achieved.length ? `${m.achieved.length} 个习惯达成` : '等待你的坚持' }}</small></div></div><p class="habit-muted">里程碑按单个习惯的最长连续执行日计算，休息日不打断连续记录。</p></section>
    </template>

    <template v-else>
      <div class="settings-group habit-window-settings">
        <div class="settings-card"><AppIcon class="card-icon" name="grid" :size="18" /><div class="card-text"><strong>在桌面显示</strong><span>关闭组件保留所有习惯和记录</span></div><div class="card-control"><button class="toggle-switch" :class="{ on: snapshot.settings.widgets.habit.enabled }" role="switch" :aria-checked="snapshot.settings.widgets.habit.enabled" aria-label="在桌面显示习惯打卡" :disabled="busy" @click="run(() => setWidgetEnabled('habit', !snapshot.settings.widgets.habit.enabled))"><span></span></button></div></div>
        <div class="settings-card"><AppIcon class="card-icon" name="grid" :size="18" /><div class="card-text"><strong>组件尺寸</strong><span>三档独立布局，右键菜单也可切换</span></div><VelaSelect :model-value="snapshot.settings.widgets.habit.size" label="桌面组件尺寸" :options="widgetSizeOptions" @update:model-value="run(() => setWidgetSize('habit', $event as WidgetSize))" /></div>
        <div class="settings-card"><AppIcon class="card-icon" name="arrow-up-right" :size="18" /><div class="card-text"><strong>窗口层级</strong><span>置顶后显示在其他窗口上方</span></div><VelaSelect :model-value="snapshot.settings.widgets.habit.alwaysOnTop ? 'top' : 'normal'" label="习惯窗口层级" :options="[{ value: 'normal', label: '普通层级' }, { value: 'top', label: '始终置顶' }]" @update:model-value="run(() => setWidgetLayer('habit', $event === 'top'))" /></div>
        <div class="settings-card"><AppIcon class="card-icon" name="lock" :size="18" /><div class="card-text"><strong>锁定位置</strong><span>防止拖动时误移组件</span></div><div class="card-control"><button class="toggle-switch" :class="{ on: snapshot.settings.widgets.habit.locked }" role="switch" :aria-checked="snapshot.settings.widgets.habit.locked" aria-label="锁定习惯组件位置" :disabled="busy" @click="run(() => setWidgetLocked('habit', !snapshot.settings.widgets.habit.locked))"><span></span></button></div></div>
      </div>
      <div class="habit-section-bar"><div><h2>桌面展示</h2><p>选择最适合你的打卡方式。</p></div></div>
      <div class="habit-style-options"><button v-for="s in styles" :key="s.value" :disabled="busy" :class="{ active: snapshot.settings.habit.style === s.value }" :aria-pressed="snapshot.settings.habit.style === s.value" @click="setStyle(s.value)"><span>{{ s.value === 'card' ? '💎' : s.value === 'list' ? '☷' : '▦' }}</span><strong>{{ s.label }}</strong><small>{{ s.description }}</small></button></div>
      <section class="habit-stat-card"><div class="habit-section-bar"><h3>实时预览</h3><div class="habit-preview-sizes"><button v-for="s in ([['small','小'],['medium','中'],['large','大']] as const)" :key="s[0]" :aria-pressed="previewSize === s[0]" :class="{ active: previewSize === s[0] }" @click="previewSize = s[0]">{{ s[1] }}</button></div></div><div class="habit-preview-stage"><div class="widget-window habit-preview-window" :class="`size-${previewSize}`" :style="{ width: `${previewSize === 'small' ? 170 : 364}px`, height: `${previewSize === 'large' ? 384 : 170}px` }"><HabitFace :habits="snapshot.habits" :records="snapshot.habitRecords" :settings="snapshot.settings.habit" :size="previewSize" :today="today" :monday="snapshot.settings.weekStartsMonday" preview /></div></div><p class="habit-muted">预览尺寸独立于桌面尺寸；小号卡片在达到每日次数目标后切换到下一个未完成习惯。</p></section>
      <section class="habit-stat-card"><div class="habit-section-bar"><h3>显示哪些习惯</h3><button class="win-button" :disabled="busy" @click="run(() => setHabitSettings({ ...snapshot.settings.habit, selectedIds: null }))">显示所有进行中的习惯</button></div><label v-for="h in all.filter(h => !h.archived)" :key="h.id" class="habit-selection-row"><input type="checkbox" :disabled="busy" :checked="snapshot.settings.habit.selectedIds === null || snapshot.settings.habit.selectedIds.includes(h.id)" @change="selectWidgetHabit(h.id)" /><span>{{ h.icon }}</span><strong>{{ h.title }}</strong><small>{{ h.dailyTarget }} 次 / 日</small></label><p v-if="!all.filter(h => !h.archived).length" class="habit-muted">先在“我的习惯”中添加或恢复习惯。</p><p class="habit-muted">展示顺序跟随“我的习惯”的排序；暂停的习惯不显示在桌面组件中。</p></section>
    </template>
    </div>
    <div v-if="checkin" class="habit-modal-backdrop" @click.self="!busy && (checkin = null)"><form ref="checkDialog" class="habit-checkin-dialog" @keydown="trapCheckFocus" role="dialog" aria-modal="true" aria-labelledby="habit-checkin-title" @submit.prevent="confirmCheck" @keydown.esc="!busy && (checkin = null)"><div class="habit-section-bar"><div><h2 id="habit-checkin-title">{{ checkin.habit.icon }} {{ checkin.habit.title }}</h2><p>{{ checkin.day }} · 记录这一次坚持</p></div><button type="button" class="habit-nav-icon" aria-label="取消打卡" :disabled="busy" @click="checkin = null">×</button></div><p v-if="error" class="habit-error" role="alert">{{ error }}</p><fieldset v-if="checkin.habit.trackMood" :disabled="busy"><legend>现在的心情</legend><div class="habit-mood-picker"><button v-for="m in moods" :key="m.value" type="button" :aria-pressed="checkMood === m.value" :class="{ active: checkMood === m.value }" @click="checkMood = m.value"><span>{{ m.emoji }}</span>{{ m.value }}</button></div></fieldset><fieldset v-if="checkin.habit.trackRating" :disabled="busy"><legend>给自己打个分</legend><div class="habit-rating-picker"><button v-for="n in 5" :key="n" type="button" :aria-label="`${n} 分`" :aria-pressed="checkRating === String(n)" :class="{ active: Number(checkRating) >= n }" @click="checkRating = String(n)">★</button><span>{{ checkRating ? `${checkRating} / 5` : '请选择' }}</span></div></fieldset><label v-if="checkin.habit.trackResult" class="habit-result-input">本次成绩<input v-model="checkResult" type="number" step="any" placeholder="输入数值" required /></label><div class="habit-dialog-actions"><button type="button" class="win-button" :disabled="busy" @click="checkin = null">取消</button><button type="submit" class="win-button habit-primary" :disabled="busy">{{ busy ? '保存中…' : '完成打卡' }}</button></div></form></div>
  </section>
</template>
