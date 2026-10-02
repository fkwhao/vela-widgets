<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import AppIcon from "./AppIcon.vue";
import { isNativeApp, openManager, showWidgetContextMenu } from "../lib/backend";
import { setWidgetEnabled, setWidgetLayer, snapshot } from "../lib/store";
import { useWindowBounds } from "../lib/useWindowBounds";

interface CalendarCell {
  date: Date;
  key: string;
  inMonth: boolean;
}

const now = new Date();
const visibleMonth = ref(new Date(now.getFullYear(), now.getMonth(), 1));
const selectedDate = ref(dateKey(now));
const menu = ref<{ x: number; y: number } | null>(null);
const notice = ref("");
let noticeTimer: ReturnType<typeof setTimeout> | undefined;

function dateKey(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

const todayKey = dateKey(now);
const monthLabel = computed(() =>
  new Intl.DateTimeFormat("zh-CN", { year: "numeric", month: "long" }).format(visibleMonth.value),
);
const weekdayLabels = computed(() =>
  snapshot.value.settings.weekStartsMonday
    ? ["一", "二", "三", "四", "五", "六", "日"]
    : ["日", "一", "二", "三", "四", "五", "六"],
);
const cells = computed<CalendarCell[]>(() => {
  const first = new Date(visibleMonth.value.getFullYear(), visibleMonth.value.getMonth(), 1);
  const offset = snapshot.value.settings.weekStartsMonday
    ? (first.getDay() + 6) % 7
    : first.getDay();
  const start = new Date(first.getFullYear(), first.getMonth(), 1 - offset);
  return Array.from({ length: 42 }, (_, index) => {
    const date = new Date(start.getFullYear(), start.getMonth(), start.getDate() + index);
    return { date, key: dateKey(date), inMonth: date.getMonth() === visibleMonth.value.getMonth() };
  });
});
const selectedLabel = computed(() => {
  const date = new Date(`${selectedDate.value}T12:00:00`);
  return new Intl.DateTimeFormat("zh-CN", { month: "long", day: "numeric", weekday: "long" }).format(date);
});

function moveMonth(delta: number): void {
  visibleMonth.value = new Date(visibleMonth.value.getFullYear(), visibleMonth.value.getMonth() + delta, 1);
}

function goToToday(): void {
  const today = new Date();
  visibleMonth.value = new Date(today.getFullYear(), today.getMonth(), 1);
  selectedDate.value = dateKey(today);
}

async function openContextMenu(event: MouseEvent): Promise<void> {
  if (isNativeApp()) {
    const x = Math.max(6, Math.min(event.clientX, window.innerWidth - 194));
    const y = Math.max(6, Math.min(event.clientY, window.innerHeight - 116));
    try {
      await showWidgetContextMenu("calendar", x, y);
    } catch {
      menu.value = {
        x: Math.max(6, Math.min(event.clientX, window.innerWidth - 194)),
        y: Math.max(6, Math.min(event.clientY, window.innerHeight - 116)),
      };
    }
    return;
  }
  menu.value = {
    x: Math.max(6, Math.min(event.clientX, window.innerWidth - 196)),
    y: Math.max(6, Math.min(event.clientY, window.innerHeight - 120)),
  };
}

function showNotice(message: string): void {
  notice.value = message;
  if (noticeTimer) clearTimeout(noticeTimer);
  noticeTimer = setTimeout(() => (notice.value = ""), 2200);
}

async function toggleLayer(): Promise<void> {
  try {
    await setWidgetLayer("calendar", !snapshot.value.settings.widgets.calendar.alwaysOnTop);
    menu.value = null;
  } catch {
    showNotice("层级设置暂时没有保存。 ");
  }
}

async function closeWidget(): Promise<void> {
  try {
    await setWidgetEnabled("calendar", false);
  } catch (error) {
    showNotice(typeof error === "string" ? error : "没有关闭组件。 ");
  }
  menu.value = null;
}

function dismissMenu(): void {
  menu.value = null;
}

useWindowBounds("calendar");
onMounted(() => window.addEventListener("pointerdown", dismissMenu));
onUnmounted(() => {
  window.removeEventListener("pointerdown", dismissMenu);
  if (noticeTimer) clearTimeout(noticeTimer);
});
</script>

<template>
  <main class="widget-window calendar-widget" @contextmenu.prevent.stop="openContextMenu">
    <header class="widget-header" :class="{ 'widget-header-draggable': !snapshot.settings.widgets.calendar.locked }" :data-tauri-drag-region="snapshot.settings.widgets.calendar.locked ? undefined : 'deep'">
      <div class="widget-brandline">
        <span class="widget-brand-dot"></span>
        <span>日历</span>
      </div>
      <button class="widget-icon-button today-button" title="回到今天" aria-label="回到今天" @pointerdown.stop @click="goToToday">今天</button>
    </header>

    <section class="calendar-monthbar">
      <div class="calendar-month-copy">
        <h1>{{ monthLabel }}</h1>
        <span>{{ new Intl.DateTimeFormat('zh-CN', { weekday: 'long' }).format(new Date(`${selectedDate}T12:00:00`)) }} · {{ selectedDate.split('-')[2] }} 日</span>
      </div>
      <div class="month-actions">
        <button class="widget-icon-button" aria-label="上个月" @click="moveMonth(-1)"><AppIcon name="chevron-left" :size="16" /></button>
        <button class="widget-icon-button" aria-label="下个月" @click="moveMonth(1)"><AppIcon name="chevron-right" :size="16" /></button>
      </div>
    </section>

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
        >
          <span>{{ cell.date.getDate() }}</span>
          <i v-if="cell.key === todayKey" class="today-mark"></i>
        </button>
      </div>
    </section>

    <footer class="calendar-footer" aria-label="已选择日期">
      <div class="selected-date">
        <span class="selected-date-marker"></span>
        <strong>{{ selectedLabel }}</strong>
      </div>
    </footer>

    <transition name="notice"><div v-if="notice" class="widget-notice">{{ notice }}</div></transition>

    <div v-if="menu" class="widget-context-menu" :style="{ left: `${menu.x}px`, top: `${menu.y}px` }" @pointerdown.stop>
      <button @click="openManager(); menu = null"><AppIcon name="sliders" :size="16" />Vela 偏好设置</button>
      <button @click="toggleLayer"><AppIcon name="arrow-up-right" :size="16" />{{ snapshot.settings.widgets.calendar.alwaysOnTop ? '取消置顶' : '始终置顶' }}</button>
      <div class="context-divider"></div>
      <button class="context-danger" @click="closeWidget"><AppIcon name="close" :size="16" />关闭日历组件</button>
    </div>
  </main>
</template>
