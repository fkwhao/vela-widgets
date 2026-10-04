<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import { getLocalTimeZone, parseDate, today } from "@internationalized/date";
import {
  CalendarCell,
  CalendarCellTrigger,
  CalendarGrid,
  CalendarGridBody,
  CalendarGridHead,
  CalendarGridRow,
  CalendarHeadCell,
  CalendarHeader,
  CalendarHeading,
  CalendarNext,
  CalendarPrev,
  CalendarRoot,
} from "reka-ui";
import AppIcon from "./AppIcon.vue";
import { snapshot } from "../../app/store";

// System-style calendar layout with Fluent surfaces and month/year drill-down.
const props = defineProps<{ modelValue: string; min?: string; max?: string; clearable?: boolean; compact?: boolean }>();
const emit = defineEmits<{ select: [value: string]; clear: [] }>();
const value = computed(() => { try { return props.modelValue ? parseDate(props.modelValue) : undefined; } catch { return undefined; } });
const todayValue = today(getLocalTimeZone());
const placeholder = ref(value.value ?? todayValue);
const minValue = computed(() => { try { return props.min ? parseDate(props.min) : undefined; } catch { return undefined; } });
const maxValue = computed(() => { try { return props.max ? parseDate(props.max) : undefined; } catch { return undefined; } });
const view = ref<"days" | "months" | "years">("days");
const yearStart = ref(1);
const chooser = ref<HTMLElement | null>(null);
const years = computed(() => Array.from({ length: 12 }, (_, index) => yearStart.value + index).filter(year => year <= 9999));
const months = Array.from({ length: 12 }, (_, index) => index + 1);
function monthAllowed(year: number, month: number) {
  const start = placeholder.value.set({ year, month, day: 1 });
  const end = start.set({ day: 31 });
  return (!minValue.value || end.compare(minValue.value) >= 0) && (!maxValue.value || start.compare(maxValue.value) <= 0);
}
function yearAllowed(year: number) { return (!minValue.value || year >= minValue.value.year) && (!maxValue.value || year <= maxValue.value.year); }
function focusChoice() {
  const button = chooser.value?.querySelector<HTMLButtonElement>('button[aria-pressed="true"]:not(:disabled)') ?? chooser.value?.querySelector<HTMLButtonElement>('button:not(:disabled)');
  button?.focus();
}
async function showChooser(next: "months" | "years") {
  view.value = view.value === next ? "days" : next;
  yearStart.value = Math.floor((placeholder.value.year - 1) / 12) * 12 + 1;
  await nextTick();
  focusChoice();
}
async function chooseYear(year: number) { placeholder.value = placeholder.value.set({ year }); view.value = "months"; await nextTick(); focusChoice(); }
async function chooseMonth(month: number) { placeholder.value = placeholder.value.set({ month }); view.value = "days"; await nextTick(); document.querySelector<HTMLElement>('.vela-calendar-day[data-selected]:not([data-outside-view]), .vela-calendar-day:not([data-disabled]):not([data-outside-view])')?.focus(); }
const canPrevious = computed(() => view.value === "years" ? yearStart.value > 1 && (!minValue.value || yearStart.value > minValue.value.year) : placeholder.value.year > (minValue.value?.year ?? 1));
const canNext = computed(() => view.value === "years" ? yearStart.value + 12 <= (maxValue.value?.year ?? 9999) : placeholder.value.year < (maxValue.value?.year ?? 9999));
function movePeriod(delta: number) {
  if (view.value === "years") yearStart.value = Math.max(1, Math.min(9997, yearStart.value + delta * 12));
  else placeholder.value = placeholder.value.set({ year: placeholder.value.year + delta });
}
function chooserKeys(event: KeyboardEvent) {
  const buttons = Array.from(chooser.value?.querySelectorAll<HTMLButtonElement>('button') ?? []);
  const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
  const delta = ({ ArrowLeft: -1, ArrowRight: 1, ArrowUp: -3, ArrowDown: 3 } as Record<string, number>)[event.key];
  if (delta === undefined || index < 0) return;
  event.preventDefault();
  for (let next = index + delta; next >= 0 && next < buttons.length; next += delta) {
    if (!buttons[next].disabled) { buttons[next].focus(); break; }
  }
}
const weekStartsOn = computed(() => snapshot.value.settings.weekStartsMonday ? 1 : 0);
function onSelect(next: unknown) { if (next && typeof next === "object" && "toString" in next) emit("select", String(next)); }
function goToday() { const t = today(getLocalTimeZone()); placeholder.value = t; view.value = "days"; if ((!minValue.value || t.compare(minValue.value) >= 0) && (!maxValue.value || t.compare(maxValue.value) <= 0)) emit("select", t.toString()); }
function onWheel(event: WheelEvent) {
  event.preventDefault();
  const delta = event.deltaY;
  if (Math.abs(delta) < 10) return;
  if (view.value !== "days") { if (delta > 0 ? canNext.value : canPrevious.value) movePeriod(delta > 0 ? 1 : -1); return; }
  const current = placeholder.value;
  placeholder.value = delta > 0 ? current.add({ months: 1 }) : current.subtract({ months: 1 });
}
</script>

<template>
  <CalendarRoot v-slot="{ grid, weekDays }" :placeholder="placeholder as any" class="vela-calendar" :class="{ compact }" :model-value="value as any" :min-value="minValue as any" :max-value="maxValue as any" :week-starts-on="weekStartsOn" weekday-format="narrow" locale="zh-CN" :fixed-weeks="!compact" prevent-deselect initial-focus @update:model-value="onSelect" @update:placeholder="(v: any) => placeholder = v" @wheel="onWheel">
    <CalendarHeader class="vela-calendar-header">
      <CalendarHeading class="visually-hidden" />
      <div class="vela-calendar-heading vela-calendar-period">
        <button type="button" :aria-label="view === 'days' ? '选择年月' : view === 'months' ? '选择年份' : '返回日期'" :aria-expanded="view !== 'days'" @click="view === 'years' ? view = 'days' : showChooser(view === 'days' ? 'months' : 'years')"><span>{{ view === 'years' ? `${yearStart}–${years.at(-1)}` : view === 'months' ? `${placeholder.year}年` : `${placeholder.year}年${String(placeholder.month).padStart(2, '0')}月` }}</span><AppIcon name="chevron-down" :size="10" /></button>
      </div>
      <template v-if="view === 'days'"><CalendarPrev class="vela-calendar-icon-button" aria-label="上个月"><AppIcon name="arrow-up" :size="18" /></CalendarPrev><CalendarNext class="vela-calendar-icon-button" aria-label="下个月"><AppIcon name="arrow-down" :size="18" /></CalendarNext></template>
      <template v-else><button type="button" class="vela-calendar-icon-button" :aria-label="view === 'years' ? '上一组年份' : '上一年'" :disabled="!canPrevious" @click="movePeriod(-1)"><AppIcon name="arrow-up" :size="18" /></button><button type="button" class="vela-calendar-icon-button" :aria-label="view === 'years' ? '下一组年份' : '下一年'" :disabled="!canNext" @click="movePeriod(1)"><AppIcon name="arrow-down" :size="18" /></button></template>
    </CalendarHeader>
    <div v-if="view !== 'days'" ref="chooser" class="vela-calendar-chooser" :aria-label="view === 'years' ? '年份列表' : '月份列表'" @keydown="chooserKeys"><template v-if="view === 'years'"><button v-for="year in years" :key="year" type="button" :aria-pressed="placeholder.year === year" :disabled="!yearAllowed(year)" @click="chooseYear(year)">{{ year }}</button></template><template v-else><button v-for="month in months" :key="month" type="button" :aria-pressed="placeholder.month === month" :disabled="!monthAllowed(placeholder.year,month)" @click="chooseMonth(month)">{{ month }}月</button></template></div>
    <template v-else><CalendarGrid v-for="month in grid" :key="month.value.toString()" class="vela-calendar-grid">
      <CalendarGridHead><CalendarGridRow class="vela-calendar-row"><CalendarHeadCell v-for="day in weekDays" :key="day" class="vela-calendar-weekday">{{ day.replace('周', '') }}</CalendarHeadCell></CalendarGridRow></CalendarGridHead>
      <CalendarGridBody>
        <CalendarGridRow v-for="(week, index) in month.rows" :key="`week-${index}`" class="vela-calendar-row">
          <CalendarCell v-for="date in week" :key="date.toString()" :date="date" class="vela-calendar-cell">
            <CalendarCellTrigger :day="date" :month="month.value" class="vela-calendar-day" />
          </CalendarCell>
        </CalendarGridRow>
      </CalendarGridBody>
    </CalendarGrid></template>
    <div class="vela-calendar-footer"><button v-if="clearable" type="button" class="vela-calendar-text-button" aria-label="清除日期" @click="emit('clear')">清除</button><button type="button" class="vela-calendar-text-button" @click="goToday">今天</button></div>
  </CalendarRoot>
</template>
