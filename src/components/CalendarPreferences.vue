<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import { isNativeApp } from "../lib/backend";
import AppIcon from "./AppIcon.vue";
import { snapshot, setCalendarSettings, checkHolidayUpdates } from "../lib/store";
type CalendarToggle = 'showHolidays' | 'showWorkdays' | 'autoUpdate';
const calendar = computed(() => snapshot.value.settings.calendar);
const cache = computed(() => snapshot.value.holidays);
const busy = ref(false);
const message = ref("");
const error = ref("");
let resultSession = 0;
let disposed = false;
let unlistenClosed: (() => void) | undefined;
function clearResult() { resultSession++; message.value = ""; error.value = ""; }
onMounted(() => {
  if (isNativeApp()) void listen("vela://manager-closed", clearResult).then(unlisten => {
    if (disposed) unlisten(); else unlistenClosed = unlisten;
  });
});
onUnmounted(() => { disposed = true; clearResult(); unlistenClosed?.(); });
const controls: { key: CalendarToggle; label: string; description: string }[] = [
  { key: "showHolidays", label: "显示节假日", description: "显示中国大陆官方放假安排" },
  { key: "showWorkdays", label: "显示调休", description: "标记节假日调休的上班日期" },
  { key: "autoUpdate", label: "自动更新节假日", description: "开启后联网检查，最多每天一次；断网时继续使用本地数据" },
];
const coverage = computed(() => cache.value.data.years.map(y => y.year).join("、"));
function timeLabel(time: number | null): string { return time ? new Intl.DateTimeFormat("zh-CN", { dateStyle: "short", timeStyle: "short" }).format(time) : "尚未检查"; }
async function toggle(key: CalendarToggle) {
  busy.value = true; error.value = "";
  try { await setCalendarSettings({ ...calendar.value, [key]: !calendar.value[key] }); }
  catch (e) { error.value = typeof e === "string" ? e : "没有保存成功，请重试。"; }
  finally { busy.value = false; }
}
async function check() {
  busy.value = true; error.value = ""; message.value = "";
  const session = resultSession;
  const revision = cache.value.data.revision;
  try {
    await checkHolidayUpdates();
    if (session !== resultSession) return;
    if (cache.value.lastError) error.value = cache.value.lastError;
    else message.value = cache.value.data.revision > revision ? "节假日数据已更新，桌面日历已同步。" : "已是最新的节假日数据。";
  } catch (e) { if (session === resultSession) error.value = typeof e === "string" ? e : "检查没有完成，请重试。"; }
  finally { busy.value = false; }
}
</script>
<template>
  <h2 class="section-title">节假日</h2>
  <div class="settings-group">
    <div v-for="control in controls" :key="control.key" class="settings-card">
      <AppIcon class="card-icon" name="calendar" :size="18" />
      <div class="card-text"><strong>{{ control.label }}</strong><span>{{ control.description }}</span></div>
      <button class="toggle-switch" :class="{ on: calendar[control.key] }" role="switch" :aria-checked="calendar[control.key]" :aria-label="control.label" :disabled="busy" @click="toggle(control.key)"><span></span></button>
    </div>
    <div class="settings-card stacked holiday-data-card">
      <div class="card-row">
        <AppIcon class="card-icon" name="calendar" :size="18" />
        <div class="card-text">
          <div class="holiday-data-heading">
            <strong>节假日数据</strong>
            <button type="button" class="holiday-info-button" aria-label="节假日数据说明" :data-tooltip="'数据来自官方年度通知，经项目核对后发布。未收录年份不推算放假或调休。\n\n检查时只请求项目的公开假期文件，不上传便签、待办或其他本地内容。'"><AppIcon name="info" :size="16" /></button>
          </div>
          <span>已收录 {{ coverage }} 年 · 数据日期 {{ cache.data.updatedAt }}</span>
        </div>
        <button class="win-button" :disabled="busy" @click="check">{{ busy ? '请稍候…' : '立即检查' }}</button>
      </div>
      <div class="holiday-data-details"><span>最近检查：{{ timeLabel(cache.lastAttemptAt) }}</span><span v-if="cache.lastUpdatedAt">最近更新：{{ timeLabel(cache.lastUpdatedAt) }}</span></div>
      <div v-if="error" class="info-bar" role="alert">{{ error }}</div>
      <div v-else-if="message" class="info-bar success" role="status">{{ message }}</div>
    </div>
  </div>
</template>
