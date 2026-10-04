<script setup lang="ts">
import { computed, onUnmounted, ref } from "vue";
import WidgetFrame from "../../../shared/widgets/WidgetFrame.vue";
import HabitFace from "./HabitFace.vue";
import { adjustHabitRecord, setHabitSettings, snapshot } from "../../../app/store";
import { openManager } from "../../../infrastructure/backend";
import { dateKey } from "../habits";
import { useWidgetClock } from "../../../shared/composables/useWidgetClock";
import type { HabitItem, HabitStyle } from "../../../shared/types";
const now = useWidgetClock(undefined, true);
const today = computed(() => dateKey(now.value));
const busy = ref(false), error = ref('');
const last = ref<{ id: number; date: string } | null>(null);
let timer: ReturnType<typeof setTimeout> | undefined;
async function check(h: HabitItem) {
  if (busy.value) return; busy.value = true; error.value = '';
  const day = dateKey();
  try { await adjustHabitRecord({ habitId: h.id, date: day, delta: 1 }); last.value = { id: h.id, date: day }; if (timer) clearTimeout(timer); timer = setTimeout(() => { last.value = null; }, 6000); }
  catch (e) { error.value = typeof e === 'string' ? e : '打卡没有保存，请重试。'; }
  finally { busy.value = false; }
}
async function undo() {
  if (!last.value || busy.value) return; busy.value = true;
  try { await adjustHabitRecord({ habitId: last.value.id, date: last.value.date, delta: -1 }); last.value = null; error.value = ''; }
  catch { error.value = '撤销没有保存，请重试。'; } finally { busy.value = false; }
}
async function style(value: HabitStyle) { try { await setHabitSettings({ ...snapshot.value.settings.habit, style: value }); } catch { error.value = '样式没有保存，请重试。'; } }
onUnmounted(() => { if (timer) clearTimeout(timer); });
</script>
<template>
  <WidgetFrame kind="habit" header-only>
    <template #default="{ widget, drag }">
      <HabitFace :habits="snapshot.habits" :records="snapshot.habitRecords" :settings="snapshot.settings.habit" :size="widget.size" :today="today" :monday="snapshot.settings.weekStartsMonday" :busy="busy" :drag="drag" @check="check" @manage="openManager('habit')" />
      <div v-if="error" class="habit-widget-feedback" role="alert">{{ error }}<button aria-label="关闭提示" @click="error = ''">×</button></div>
      <div v-else-if="last" class="habit-widget-feedback saved" role="status">已打卡<button :disabled="busy" @click="undo">撤销</button></div>
    </template>
    <template #context-actions="{ dismiss }"><button v-for="s in ([['card','习惯卡片'],['list','习惯列表'],['report','本周成绩单']] as const)" :key="s[0]" role="menuitem" @click="style(s[0]); dismiss()">{{ s[1] }}{{ snapshot.settings.habit.style === s[0] ? ' ✓' : '' }}</button><div class="context-divider"></div></template>
  </WidgetFrame>
</template>
