<script setup lang="ts">
import { computed, defineAsyncComponent, ref } from "vue";
import ClockThemePreferences from "../../features/clock/components/ClockThemePreferences.vue";
import ClockToolsPanel from "../../features/clock/components/ClockToolsPanel.vue";
import { clockModes } from "../types";
import AppIcon from "../ui/AppIcon.vue";
const NoteRetention = defineAsyncComponent(() => import("../../features/notes/components/NoteRetention.vue"));
import { snapshot, setClockSettings, setNoteColor, saveCountdown, deleteCountdown } from "../../app/store";
import { localDateKey } from "../../features/countdown/countdown";
import type { ClockMode, CountdownItem, WidgetKind } from "../types";
defineProps<{ kind: WidgetKind }>();
const error = ref("");
const busy = ref(false);
const editor = ref(false);
const editingId = ref<number | null>(null);
const title = ref("");
const date = ref(localDateKey(new Date()));
const yearly = ref(false);
const countUp = ref(false);
const createdDate = ref(localDateKey(new Date()));
const clockToolModes = clockModes.filter(mode => mode.value !== "clock");
const clockMode = ref<ClockMode | null>(null);
const clock = computed(() => snapshot.value.settings.clock);
const cities = [
  { name: "北京", timeZone: "Asia/Shanghai" }, { name: "东京", timeZone: "Asia/Tokyo" }, { name: "新加坡", timeZone: "Asia/Singapore" }, { name: "迪拜", timeZone: "Asia/Dubai" }, { name: "伦敦", timeZone: "Europe/London" }, { name: "巴黎", timeZone: "Europe/Paris" }, { name: "纽约", timeZone: "America/New_York" }, { name: "洛杉矶", timeZone: "America/Los_Angeles" }, { name: "悉尼", timeZone: "Australia/Sydney" }, { name: "奥克兰", timeZone: "Pacific/Auckland" },
];
const colors = [
  { value: "#3b67b8", label: "蓝色" }, { value: "#805eb2", label: "紫色" },
  { value: "#c14289", label: "粉色" }, { value: "#d4484d", label: "红色" },
  { value: "#dc812e", label: "橙色" }, { value: "#c39a27", label: "黄色" },
  { value: "#399466", label: "绿色" },
];
const isCustomNoteColor = computed(() => !colors.some(color => color.value === snapshot.value.settings.note.color.toLowerCase()));
function onCustomNoteColor(event: Event) { const color = (event.target as HTMLInputElement).value; void run(() => setNoteColor(color)); }
const selectedNoteColor = computed(() => colors.find(color => color.value === snapshot.value.settings.note.color.toLowerCase())?.label ?? "自定义");
async function run(action: () => Promise<void>) { error.value = ""; busy.value = true; try { await action(); } catch(e) { error.value = typeof e === "string" ? e : "没有保存成功，请重试。"; } finally { busy.value = false; } }
function toggleCity(city: typeof cities[number]) {
  const selected = clock.value.cities.some((c) => c.timeZone === city.timeZone);
  if (!selected && clock.value.cities.length >= 4) { error.value = "最多选择四个城市。"; return; }
  void run(() => setClockSettings({ ...clock.value, cities: selected ? clock.value.cities.filter((c) => c.timeZone !== city.timeZone) : [...clock.value.cities, city] }));
}
function edit(item?: CountdownItem) { editor.value = true; editingId.value = item?.id ?? null; title.value = item?.title ?? ""; date.value = item?.date ?? localDateKey(new Date()); yearly.value = item?.yearly ?? false; countUp.value = item?.countUp ?? false; createdDate.value = item?.createdDate ?? localDateKey(new Date()); }
async function submit() {
  if (!title.value.trim()) { error.value = "请填写事件名称。"; return; }
  await run(async () => { await saveCountdown({ id: editingId.value, title: title.value.trim(), date: date.value, yearly: yearly.value, countUp: countUp.value, createdDate: createdDate.value }); editor.value = false; });
}
const deleting = ref<number | null>(null);
</script>
<template>
  <div v-if="error" class="info-bar" role="alert">{{ error }}</div>
  <template v-if="kind === 'clock'">
    <ClockThemePreferences />
    <h2 class="section-title">时间显示</h2><div class="settings-group">
      <div class="settings-card"><AppIcon class="card-icon" name="clock" :size="18" /><div class="card-text"><strong>12 小时制</strong><span>关闭时使用 24 小时制</span></div><button class="toggle-switch" :class="{ on: clock.hour12 }" role="switch" :aria-checked="clock.hour12" aria-label="12 小时制" :disabled="busy" @click="run(() => setClockSettings({ ...clock, hour12: !clock.hour12 }))"><span></span></button></div>
      <div class="settings-card"><AppIcon class="card-icon" name="clock" :size="18" /><div class="card-text"><strong>显示秒</strong><span>数字时间显示秒数；指针表盘始终显示秒针</span></div><button class="toggle-switch" :class="{ on: clock.showSeconds }" role="switch" :aria-checked="clock.showSeconds" aria-label="显示秒" :disabled="busy" @click="run(() => setClockSettings({ ...clock, showSeconds: !clock.showSeconds }))"><span></span></button></div>
      <div class="settings-card stacked"><div class="card-text"><strong>世界时钟</strong><span>最多四个城市，时差自动随夏令时变化</span></div><div class="city-options"><button v-for="city in cities" :key="city.timeZone" class="win-button" :class="{ selected: clock.cities.some(c => c.timeZone === city.timeZone) }" :aria-pressed="clock.cities.some(c => c.timeZone === city.timeZone)" :disabled="busy" @click="toggleCity(city)">{{ city.name }}</button></div></div>
    </div>
    <h2 class="section-title clock-tools-heading">时钟工具<button type="button" class="holiday-info-button" aria-label="时钟工具说明" data-tooltip="切换设置页工具不会改变桌面时钟模式，也不会中断计时。&#10;&#10;Vela 运行时到时弹窗并播放系统提示音；隐藏组件仍会提醒。退出期间到期的提醒在下次启动时补显。"><AppIcon name="info" :size="16" /></button></h2>
    <div class="settings-card stacked clock-management">
      <nav class="clock-manager-tabs" aria-label="时钟工具模式"><button v-for="mode in clockToolModes" :key="mode.value" class="win-button" :class="{selected:clockMode===mode.value}" :aria-pressed="clockMode===mode.value" :disabled="busy" @click="clockMode=clockMode===mode.value ? null : mode.value"><AppIcon :name="mode.icon" :size="15" />{{ mode.label }}</button></nav>
      <ClockToolsPanel v-if="clockMode" :mode="clockMode" manager />
    </div>
  </template>
  <template v-else-if="kind === 'note'">
    <h2 class="section-title">便签</h2>
    <div class="settings-group">
      <div class="settings-card stacked note-color-card">
        <div class="card-row">
          <AppIcon class="card-icon" name="spark" :size="18" />
          <div class="card-text"><strong>便签强调色</strong><span>为标题和图标选择颜色，点击后自动保存</span></div>
          <span class="note-color-current" aria-live="polite">{{ selectedNoteColor }}</span>
        </div>
        <div class="note-color-layout">
          <div class="note-color-options" role="group" aria-label="便签强调色">
            <button v-for="color in colors" :key="color.value" type="button" class="note-color-option" :class="{ selected: snapshot.settings.note.color === color.value }" :aria-label="`选择${color.label}`" :aria-pressed="snapshot.settings.note.color === color.value" :disabled="busy" @click="run(() => setNoteColor(color.value))">
              <span class="note-color-dot" :style="{ backgroundColor: color.value }"><AppIcon v-if="snapshot.settings.note.color === color.value" name="tick" :size="15" /></span>
              <span>{{ color.label }}</span>
            </button>
            <label class="note-color-option note-color-custom" :class="{ selected: isCustomNoteColor, 'is-busy': busy }" data-tooltip="打开调色盘">
              <span class="note-color-dot note-color-rainbow"><AppIcon v-if="isCustomNoteColor" name="tick" :size="15" /></span>
              <span>自定义</span>
              <input type="color" :value="snapshot.settings.note.color" aria-label="自定义便签颜色" :disabled="busy" @change="onCustomNoteColor" />
            </label>
          </div>
          <div class="note-color-preview" :style="{ '--note-preview-color': snapshot.settings.note.color }" role="img" :aria-label="`${selectedNoteColor}便签标题预览`">
            <div class="note-color-preview-title"><AppIcon name="note" :size="16" /><strong>便签</strong><AppIcon class="note-preview-edit" name="edit" :size="13" /></div>
            <p>记下今天的小想法</p><span>留一点时间给自己。</span>
            <small>效果预览</small>
          </div>
        </div>
      </div>
      <NoteRetention />
      <div class="settings-card"><AppIcon class="card-icon" name="note" :size="18" /><div class="card-text"><strong>桌面多便签</strong><span>桌面直接新建、编辑、删除，右键查看列表；第一行自动生成标题。支持 Markdown、公式与图表，Ctrl+Enter 完成编辑。</span></div></div>
    </div>
  </template>
  <template v-else-if="kind === 'countdown'">
    <div class="settings-section-heading"><h2 class="section-title">重要的日子</h2><button class="win-button" :disabled="busy" @click="edit()">添加日子</button></div>
    <form v-if="editor" class="countdown-editor settings-card stacked" @submit.prevent="submit">
      <label>事件名称<input v-model="title" class="win-text-input" maxlength="80" placeholder="例如：旅行、生日、考试" required /></label>
      <label>日期<input v-model="date" class="win-text-input" type="date" min="0001-01-01" max="9999-12-31" required /></label>
      <label class="inline-check"><input v-model="yearly" type="checkbox" />每年重复（2 月 29 日在平年按 2 月 28 日计算）</label>
      <label class="inline-check"><input v-model="countUp" type="checkbox" />正数：记录已经过去的天数</label>
      <div class="editor-actions"><button type="button" class="win-button" :disabled="busy" @click="editor = false">取消</button><button class="win-button primary" :disabled="busy">{{ busy ? '正在保存' : '保存' }}</button></div>
    </form>
    <div class="settings-group"><div v-for="item in snapshot.countdowns" :key="item.id" class="settings-card"><AppIcon class="card-icon" name="hourglass" :size="18" /><div class="card-text"><strong>{{ item.title }}</strong><span>{{ item.date }} · {{ item.countUp ? '正数' : '倒数' }}{{ item.yearly ? ' · 每年重复' : '' }}</span></div><div class="card-control"><template v-if="deleting === item.id"><button class="win-button" :disabled="busy" @click="run(async () => { await deleteCountdown(item.id); deleting = null; if (editingId === item.id) editor = false; })">确认删除</button><button class="win-button" @click="deleting = null">取消</button></template><template v-else><button class="win-button" :disabled="busy" @click="edit(item)">编辑</button><button class="win-button" :disabled="busy" @click="deleting = item.id">删除</button></template></div></div><div v-if="!snapshot.countdowns.length" class="settings-card"><div class="card-text"><strong>还没有添加日子</strong><span>添加后即可在桌面查看，小尺寸展示距离今天最近的事件</span></div></div></div>
  </template>
</template>
