<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import AppIcon from "./AppIcon.vue";
import CalendarSchedule from "./CalendarSchedule.vue";
import CalendarEventEditor from "./CalendarEventEditor.vue";
import { eventsForDay } from "../lib/calendarEvents";
import WidgetSizeMenuRow from "./WidgetSizeMenuRow.vue";
import { isNativeApp, openManager, showWidgetContextMenu } from "../lib/backend";
import { setCalendarSettings, setWidgetEnabled, setWidgetLayer, setWidgetSize, snapshot } from "../lib/store";
import { useWindowBounds } from "../lib/useWindowBounds";
import type { CalendarEvent, WidgetSize } from "../types";
import { holidayLabel, visibleHoliday, isCalendarRestDay } from "../lib/holidays";

interface CalendarCell {
  date: Date;
  key: string;
  inMonth: boolean;
}

const today = ref(new Date());
const visibleMonth = ref(new Date(today.value.getFullYear(), today.value.getMonth(), 1));
const selectedDate = ref(dateKey(today.value));
const menu = ref<{ x: number; y: number } | null>(null);
const notice = ref("");
const eventEditor = ref(false);
const editingEvent = ref<CalendarEvent | undefined>();
function openEvent(event?: CalendarEvent) { editingEvent.value=event;eventEditor.value=true; }
const selectedEvents = computed(()=>eventsForDay(snapshot.value.settings.calendar.events,selectedDate.value));
async function changeStyle(style: 'agenda'|'list') { try { await setCalendarSettings({...snapshot.value.settings.calendar,style}); } catch { showNotice('样式没有保存成功。'); } }
let noticeTimer: ReturnType<typeof setTimeout> | undefined;
let clockTimer: ReturnType<typeof setInterval> | undefined;

function dateKey(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function isoWeek(date: Date): number {
  const target = new Date(Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()));
  const day = target.getUTCDay() || 7;
  target.setUTCDate(target.getUTCDate() + 4 - day);
  const yearStart = new Date(Date.UTC(target.getUTCFullYear(), 0, 1));
  return Math.ceil(((target.getTime() - yearStart.getTime()) / 86400000 + 1) / 7);
}

const widget = computed(() => snapshot.value.settings.widgets.calendar);
const size = computed(() => widget.value.size);
// Medium reserves the right panel for month navigation; the today panel drags.
const dragRegion = computed(() => (widget.value.locked ? undefined : "deep"));
const previewSize = computed(() => isNativeApp() ? undefined : { width: `${size.value === "small" ? 170 : 364}px`, height: `${size.value === "large" ? 384 : 170}px` });
const todayKey = computed(() => dateKey(today.value));
const holidaySettings = computed(() => snapshot.value.settings.calendar);
const holidayMap = computed(() => new Map(snapshot.value.holidays.data.years.flatMap(y => y.days.map(day => [day.date, day] as const))));
function restDay(date: Date, key = dateKey(date)) { return isCalendarRestDay(date, holidayMap.value.get(key)); }
function holidayFor(key: string) { return visibleHoliday(holidayMap.value.get(key), holidaySettings.value); }
function holidayText(key: string): string { const day = holidayFor(key); return day ? holidayLabel(day) : ""; }
const todayHoliday = computed(() => holidayText(todayKey.value));
const selectedHoliday = computed(() => holidayText(selectedDate.value));
const visibleYearKnown = computed(() => snapshot.value.holidays.data.years.some(y => y.year === visibleMonth.value.getFullYear()));
const showHolidayData = computed(() => holidaySettings.value.showHolidays || holidaySettings.value.showWorkdays);
function cellLabel(cell: CalendarCell): string {
  return [new Intl.DateTimeFormat('zh-CN', { dateStyle: 'full' }).format(cell.date), holidayText(cell.key), ...eventsForDay(snapshot.value.settings.calendar.events,cell.key).map(e=>`${e.title} ${e.allDay?'全天':e.startTime}`)].filter(Boolean).join('，');
}
const weekStartsMonday = computed(() => snapshot.value.settings.weekStartsMonday);

const weekdayLabels = computed(() =>
  weekStartsMonday.value ? ["一", "二", "三", "四", "五", "六", "日"] : ["日", "一", "二", "三", "四", "五", "六"],
);

function monthCells(month: Date, rows: number): CalendarCell[] {
  const first = new Date(month.getFullYear(), month.getMonth(), 1);
  const offset = weekStartsMonday.value ? (first.getDay() + 6) % 7 : first.getDay();
  const start = new Date(first.getFullYear(), first.getMonth(), 1 - offset);
  return Array.from({ length: rows * 7 }, (_, index) => {
    const date = new Date(start.getFullYear(), start.getMonth(), start.getDate() + index);
    return { date, key: dateKey(date), inMonth: date.getMonth() === month.getMonth() };
  });
}

const cells = computed(() => monthCells(visibleMonth.value, 6));
// Medium follows the same browsed month as large and drops trailing empty weeks.
const miniMonthCells = computed(() => {
  const all = cells.value;
  return all.slice(0, all[35].inMonth ? 42 : 35);
});

const monthTitle = computed(() => `${visibleMonth.value.getMonth() + 1}月`);
const yearTitle = computed(() => `${visibleMonth.value.getFullYear()}`);
const todayWeekday = computed(() => new Intl.DateTimeFormat("zh-CN", { weekday: "long" }).format(today.value));
const todayMonthLabel = computed(() => `${today.value.getFullYear()}年${today.value.getMonth() + 1}月`);
const todayWeekLabel = computed(() => `第 ${isoWeek(today.value)} 周`);
const showingToday = computed(
  () =>
    selectedDate.value === todayKey.value &&
    visibleMonth.value.getFullYear() === today.value.getFullYear() &&
    visibleMonth.value.getMonth() === today.value.getMonth(),
);

const selectedSummary = computed(() => {
  const date = new Date(`${selectedDate.value}T12:00:00`);
  const label = new Intl.DateTimeFormat("zh-CN", { month: "long", day: "numeric", weekday: "long" }).format(date);
  const base = new Date(`${todayKey.value}T12:00:00`);
  const days = Math.round((date.getTime() - base.getTime()) / 86400000);
  const relative = days === 0 ? "今天" : days === 1 ? "明天" : days === -1 ? "昨天" : days > 0 ? `${days} 天后` : `${-days} 天前`;
  return { label, relative };
});

function moveMonth(delta: number): void {
  visibleMonth.value = new Date(visibleMonth.value.getFullYear(), visibleMonth.value.getMonth() + delta, 1);
}

function goToToday(): void {
  visibleMonth.value = new Date(today.value.getFullYear(), today.value.getMonth(), 1);
  selectedDate.value = todayKey.value;
}

async function openContextMenu(event: MouseEvent): Promise<void> {
  if (isNativeApp()) {
    try {
      // The native popup clamps itself to the monitor, not to this small window.
      await showWidgetContextMenu("calendar", event.clientX, event.clientY);
      return;
    } catch {
      // Fall through to the in-window menu.
    }
  }
  menu.value = {
    x: Math.max(4, Math.min(event.clientX, window.innerWidth - 192)),
    y: Math.max(4, Math.min(event.clientY, window.innerHeight - 152)),
  };
}

function showNotice(message: string): void {
  notice.value = message;
  if (noticeTimer) clearTimeout(noticeTimer);
  noticeTimer = setTimeout(() => (notice.value = ""), 2200);
}

async function toggleLayer(): Promise<void> {
  try {
    await setWidgetLayer("calendar", !widget.value.alwaysOnTop);
    menu.value = null;
  } catch {
    showNotice("层级设置暂时没有保存。");
  }
}

async function chooseSize(next: WidgetSize): Promise<void> {
  menu.value = null;
  try {
    await setWidgetSize("calendar", next);
  } catch {
    showNotice("尺寸暂时没有保存。");
  }
}

async function closeWidget(): Promise<void> {
  try {
    await setWidgetEnabled("calendar", false);
  } catch (error) {
    showNotice(typeof error === "string" ? error : "没有关闭组件。");
  }
  menu.value = null;
}

function dismissMenu(): void {
  menu.value = null;
}

function tickClock(): void {
  const now = new Date();
  if (dateKey(now) !== todayKey.value) today.value = now;
}

useWindowBounds("calendar");
onMounted(() => {
  window.addEventListener("pointerdown", dismissMenu);
  clockTimer = setInterval(tickClock, 60_000);
});
onUnmounted(() => {
  window.removeEventListener("pointerdown", dismissMenu);
  if (noticeTimer) clearTimeout(noticeTimer);
  if (clockTimer) clearInterval(clockTimer);
});
</script>

<template>
  <CalendarSchedule v-if="holidaySettings.style !== 'month'" />
  <main
    v-else
    class="widget-window calendar-widget"
    :style="previewSize"
    :class="[`size-${size}`, { 'widget-draggable': !widget.locked }]"
    :data-tauri-drag-region="undefined"
    @contextmenu.prevent.stop="openContextMenu"
  >
    <div class="cal-style-shortcuts"><button class="widget-icon-button" aria-label="查看日程" @click="changeStyle('agenda')"><AppIcon name="clock" :size="14" /></button><button class="widget-icon-button" aria-label="查看日程列表" @click="changeStyle('list')"><AppIcon name="grid" :size="14" /></button><button class="widget-icon-button" aria-label="新建日程" @click="openEvent()"><AppIcon name="plus" :size="14" /></button></div>
    <!-- Small: today at a glance -->
    <template v-if="size === 'small'">
      <div class="cal-small" :class="{ 'rest-day': restDay(today) }" :data-tauri-drag-region="dragRegion">
        <span class="cal-eyebrow">{{ todayWeekday }}</span>
        <strong class="cal-hero-day">{{ today.getDate() }}</strong>
        <span class="cal-caption">{{ todayMonthLabel }}</span>
        <span class="cal-caption subtle" :class="{ 'cal-holiday-caption': todayHoliday }">{{ todayHoliday || todayWeekLabel }}</span>
      </div>
    </template>

    <!-- Medium: today beside a browsable month -->
    <template v-else-if="size === 'medium'">
      <div class="cal-medium">
        <div class="cal-today-panel" :class="{ 'rest-day': restDay(today) }" :data-tauri-drag-region="dragRegion">
          <span class="cal-eyebrow">{{ todayWeekday }}</span>
          <strong class="cal-hero-day">{{ today.getDate() }}</strong>
          <span class="cal-caption">{{ todayMonthLabel }}</span>
          <span class="cal-caption subtle" :class="{ 'cal-holiday-caption': todayHoliday }" :data-tooltip="todayHoliday">{{ todayHoliday || todayWeekLabel }}</span>
        </div>
        <div class="cal-mini-month" :aria-label="`${yearTitle}年${monthTitle}月历`">
          <header class="cal-mini-header">
            <strong aria-live="polite" :data-tooltip="showHolidayData && !visibleYearKnown ? '该年份的假期安排尚未收录' : undefined">{{ yearTitle }}年{{ monthTitle }}<i v-if="showHolidayData && !visibleYearKnown" class="cal-data-missing" aria-label="假期安排尚未收录">·</i></strong>
            <div class="cal-mini-actions">
              <button v-if="!showingToday" class="widget-icon-button" aria-label="回到今天" data-tooltip="回到今天" @click="goToToday"><AppIcon name="return-today" :size="13" /></button>
              <button class="widget-icon-button" aria-label="上个月" @click="moveMonth(-1)"><AppIcon name="chevron-left" :size="13" /></button>
              <button class="widget-icon-button" aria-label="下个月" @click="moveMonth(1)"><AppIcon name="chevron-right" :size="13" /></button>
            </div>
          </header>
          <div class="cal-mini-grid">
            <span v-for="weekday in weekdayLabels" :key="`w-${weekday}`" class="cal-mini-weekday">{{ weekday }}</span>
            <span
              v-for="cell in miniMonthCells"
              :key="cell.key"
              class="cal-mini-day"
              :class="{ outside: !cell.inMonth, today: cell.key === todayKey, selected: cell.key === selectedDate, 'rest-day': restDay(cell.date, cell.key) }"
              :data-tooltip="cell.inMonth ? cellLabel(cell) : undefined"
              :aria-label="cell.inMonth ? cellLabel(cell) : undefined"
            >{{ cell.inMonth ? cell.date.getDate() : "" }}<i v-if="cell.inMonth && holidayFor(cell.key)" class="cal-holiday-dot" :class="holidayFor(cell.key)?.type" aria-hidden="true"></i></span>
          </div>
        </div>
      </div>
    </template>

    <!-- Large: the full interactive month -->
    <template v-else>
      <header class="cal-large-header" :class="{ 'widget-header-draggable': !widget.locked }" :data-tauri-drag-region="dragRegion">
        <h1 class="cal-large-title"><span class="accent">{{ monthTitle }}</span><span class="year">{{ yearTitle }}</span></h1>
        <div class="cal-large-actions">
          <transition name="fade-chip">
            <button v-if="!showingToday" class="widget-icon-button" aria-label="回到今天" data-tooltip="回到今天" @click="goToToday"><AppIcon name="return-today" :size="16" /></button>
          </transition>
          <button class="widget-icon-button" aria-label="上个月" @click="moveMonth(-1)"><AppIcon name="chevron-left" :size="16" /></button>
          <button class="widget-icon-button" aria-label="下个月" @click="moveMonth(1)"><AppIcon name="chevron-right" :size="16" /></button>
        </div>
      </header>

      <section class="calendar-grid" aria-label="月历">
        <div class="weekday-row"><span v-for="weekday in weekdayLabels" :key="weekday">{{ weekday }}</span></div>
        <div class="date-grid">
          <button
            v-for="cell in cells"
            :key="cell.key"
            class="date-cell"
            :class="{ outside: !cell.inMonth, today: cell.key === todayKey, selected: cell.key === selectedDate, 'rest-day': restDay(cell.date, cell.key) }"
            :aria-label="cellLabel(cell)"
            :data-tooltip="cellLabel(cell)"
            :aria-pressed="cell.key === selectedDate"
            @click="selectedDate = cell.key"
            @dblclick="openEvent(eventsForDay(snapshot.settings.calendar.events,cell.key)[0])"
          ><span>{{ cell.date.getDate() }}</span><i v-if="holidayFor(cell.key)" class="cal-holiday-mark" :class="holidayFor(cell.key)?.type" aria-hidden="true">{{ holidayFor(cell.key)?.type === 'holiday' ? '休' : '班' }}</i><i v-if="eventsForDay(snapshot.settings.calendar.events,cell.key).length" class="cal-local-event-dot" aria-hidden="true"></i></button>
        </div>
      </section>

      <footer class="calendar-footer" aria-label="已选择日期">
        <div class="calendar-footer-date"><strong>{{ selectedSummary.label }}</strong><small v-if="selectedHoliday" class="cal-holiday-caption">{{ selectedHoliday }}</small><small v-else-if="showHolidayData && !snapshot.holidays.data.years.some(y => y.year === Number(selectedDate.slice(0, 4)))">该年份假期安排尚未收录</small></div>
        <button v-if="selectedEvents.length" class="widget-chip" :data-tooltip="selectedEvents.map(e=>e.title).join('\n')" @click="openEvent(selectedEvents[0])">{{ selectedEvents.length }} 项日程</button><span v-else class="calendar-relative" :class="{ current: selectedSummary.relative === '今天' }">{{ selectedSummary.relative }}</span>
      </footer>
    </template>

    <CalendarEventEditor v-if="eventEditor" :event="editingEvent" :date="selectedDate" @close="eventEditor=false" />
    <transition name="notice"><div v-if="notice" class="widget-notice">{{ notice }}</div></transition>

    <div v-if="menu" class="widget-context-menu" :style="{ left: `${menu.x}px`, top: `${menu.y}px` }" @pointerdown.stop>
      <button @click="openManager(); menu = null"><AppIcon name="sliders" :size="16" />Vela 偏好设置</button>
      <button @click="toggleLayer"><AppIcon name="arrow-up-right" :size="16" />{{ widget.alwaysOnTop ? '取消置顶' : '始终置顶' }}</button>
      <div class="context-divider"></div>
      <WidgetSizeMenuRow :size="size" @choose="chooseSize" />
      <div class="context-divider"></div>
      <button class="context-danger" @click="closeWidget"><AppIcon name="close" :size="16" />关闭日历组件</button>
    </div>
  </main>
</template>
