<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import AppIcon from "./AppIcon.vue";
import WidgetSizeMenuRow from "./WidgetSizeMenuRow.vue";
import { isNativeApp, openManager, showWidgetContextMenu } from "../lib/backend";
import { setWidgetEnabled, setWidgetLayer, setWidgetSize, snapshot } from "../lib/store";
import { useWindowBounds } from "../lib/useWindowBounds";
import type { WidgetSize } from "../types";

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
// Small and medium layouts have no scrolling content, so the whole surface drags.
const dragRegion = computed(() => (widget.value.locked ? undefined : "deep"));
const todayKey = computed(() => dateKey(today.value));
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
// The medium layout always shows the current month and drops trailing empty weeks.
const todayMonthCells = computed(() => {
  const month = new Date(today.value.getFullYear(), today.value.getMonth(), 1);
  const all = monthCells(month, 6);
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
  <main
    class="widget-window calendar-widget"
    :class="[`size-${size}`, { 'widget-draggable': !widget.locked }]"
    :data-tauri-drag-region="size === 'large' ? undefined : dragRegion"
    @contextmenu.prevent.stop="openContextMenu"
  >
    <!-- Small: today at a glance -->
    <template v-if="size === 'small'">
      <div class="cal-small">
        <span class="cal-eyebrow">{{ todayWeekday }}</span>
        <strong class="cal-hero-day">{{ today.getDate() }}</strong>
        <span class="cal-caption">{{ todayMonthLabel }}</span>
        <span class="cal-caption subtle">{{ todayWeekLabel }}</span>
      </div>
    </template>

    <!-- Medium: today beside the current month -->
    <template v-else-if="size === 'medium'">
      <div class="cal-medium">
        <div class="cal-today-panel">
          <span class="cal-eyebrow">{{ todayWeekday }}</span>
          <strong class="cal-hero-day">{{ today.getDate() }}</strong>
          <span class="cal-caption">{{ todayMonthLabel }}</span>
          <span class="cal-caption subtle">{{ todayWeekLabel }}</span>
        </div>
        <div class="cal-mini-month" aria-label="本月月历">
          <div class="cal-mini-grid">
            <span v-for="weekday in weekdayLabels" :key="`w-${weekday}`" class="cal-mini-weekday">{{ weekday }}</span>
            <span
              v-for="cell in todayMonthCells"
              :key="cell.key"
              class="cal-mini-day"
              :class="{ outside: !cell.inMonth, today: cell.key === todayKey }"
            >{{ cell.inMonth ? cell.date.getDate() : "" }}</span>
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
            <button v-if="!showingToday" class="widget-chip" aria-label="回到今天" @click="goToToday">今天</button>
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
            :class="{ outside: !cell.inMonth, today: cell.key === todayKey, selected: cell.key === selectedDate }"
            :aria-label="new Intl.DateTimeFormat('zh-CN', { dateStyle: 'full' }).format(cell.date)"
            :aria-pressed="cell.key === selectedDate"
            @click="selectedDate = cell.key"
          ><span>{{ cell.date.getDate() }}</span></button>
        </div>
      </section>

      <footer class="calendar-footer" aria-label="已选择日期">
        <strong>{{ selectedSummary.label }}</strong>
        <span class="calendar-relative" :class="{ current: selectedSummary.relative === '今天' }">{{ selectedSummary.relative }}</span>
      </footer>
    </template>

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
