<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, onUnmounted, ref, watch } from "vue";
const WidgetPreferences = defineAsyncComponent(() => import("./WidgetPreferences.vue"));
const CalendarSchedulePreferences = defineAsyncComponent(() => import("./CalendarSchedulePreferences.vue"));
const CalendarPreferences = defineAsyncComponent(() => import("./CalendarPreferences.vue"));
import AppIcon from "./AppIcon.vue";
import WidgetThemePreview from "./WidgetThemePreview.vue";
import VelaSelect from "./VelaSelect.vue";
import VelaSlider from "./VelaSlider.vue";
import { exitVela } from "../lib/backend";
import {
  setAccentColor,
  setTheme,
  setWeekStartsMonday,
  setWidgetEnabled,
  setWidgetAppearance,
  setWidgetLayer,
  setWidgetLocked,
  setWidgetSize,
  snapshot,
  storeError,
} from "../lib/store";
import { widgetSizeOptions, widgetKinds, widgetRegistry, isWidgetKind } from "../types";
import type { ThemeMode, WidgetKind } from "../types";

const search = ref("");
const searchInput = ref<HTMLInputElement | null>(null);
const activePage = ref("home");
const errorMessage = ref("");
const transparencyDraft = ref(snapshot.value.settings.widgetTransparency);
const cornerRadiusDraft = ref(snapshot.value.settings.widgetCornerRadius);
let errorTimer: ReturnType<typeof setTimeout> | undefined;

watch(
  () => [snapshot.value.settings.widgetTransparency, snapshot.value.settings.widgetCornerRadius],
  ([transparency, radius]) => {
    transparencyDraft.value = transparency;
    cornerRadiusDraft.value = radius;
  },
);

const navGroups = [
  [
    { id: "home", label: "我的组件", icon: "grid" },
    ...widgetKinds.map((kind) => ({ id: kind, label: widgetRegistry[kind].label, icon: widgetRegistry[kind].icon })),
  ],
  [
    { id: "appearance", label: "外观", icon: "sun" },
    { id: "startup", label: "启动", icon: "power" },
    { id: "about", label: "关于", icon: "info" },
  ],
];

const widgetMeta = widgetRegistry;

const themeOptions = [
  { id: "light", label: "浅色" },
  { id: "dark", label: "深色" },
  { id: "system", label: "跟随系统" },
] as const;

const accentColors = [
  "#3b67b8",
  "#805eb2",
  "#c14289",
  "#d4484d",
  "#dc812e",
  "#c39a27",
  "#399466",
  "#858585",
];

const enabledCount = computed(
  () => Object.values(snapshot.value.settings.widgets).filter((widget) => widget.enabled).length,
);
const pageTitle = computed(
  () => navGroups.flat().find((entry) => entry.id === activePage.value)?.label ?? "我的组件",
);
const visibleGroups = computed(() => {
  const query = search.value.trim().toLowerCase();
  return navGroups
    .map((group) => group.filter((item) => item.label.toLowerCase().includes(query)))
    .filter((group) => group.length > 0);
});
const settingKind = computed<WidgetKind | null>(() =>
  isWidgetKind(activePage.value) ? activePage.value : null,
);
const currentWidget = computed(() =>
  settingKind.value ? snapshot.value.settings.widgets[settingKind.value] : null,
);

function onGlobalKeydown(event: KeyboardEvent): void {
  const key = event.key.toLowerCase();
  if ((event.ctrlKey || event.metaKey) && (key === "f" || key === "k")) {
    event.preventDefault();
    searchInput.value?.focus();
  }
}

// Settings apply immediately, as in Windows Settings; only failures are surfaced.
function showError(message: string): void {
  errorMessage.value = message;
  if (errorTimer) clearTimeout(errorTimer);
  errorTimer = setTimeout(() => (errorMessage.value = ""), 4200);
}

async function run(action: () => Promise<void>): Promise<void> {
  try {
    await action();
  } catch (error) {
    showError(typeof error === "string" ? error : "没有保存成功，请稍后再试。");
  }
}

async function toggleWidget(kind: WidgetKind): Promise<void> {
  const enabled = snapshot.value.settings.widgets[kind].enabled;
  if (enabled && enabledCount.value <= 1) {
    showError("至少保留一个桌面组件，才能从桌面打开 Vela 偏好设置。");
    return;
  }
  await run(() => setWidgetEnabled(kind, !enabled));
}

function onThemeChange(theme: ThemeMode): void {
  void run(() => setTheme(theme));
}

function onAccentChange(event: Event): void {
  const color = (event.target as HTMLInputElement).value;
  void run(() => setAccentColor(color));
}

function chooseAccentColor(color: string): void {
  void run(() => setAccentColor(color));
}

function commitAppearance(): void {
  void run(() => setWidgetAppearance(transparencyDraft.value, cornerRadiusDraft.value));
}

function onWeekStartChange(value: string): void {
  void run(() => setWeekStartsMonday(value === "monday"));
}

function onLayerChange(value: string): void {
  const kind = settingKind.value;
  if (kind) void run(() => setWidgetLayer(kind, value === "top"));
}

function onSizeChange(value: string): void {
  const kind = settingKind.value;
  const size = widgetSizeOptions.find((option) => option.value === value)?.value;
  if (kind && size) void run(() => setWidgetSize(kind, size));
}

function toggleLocked(): void {
  const kind = settingKind.value;
  if (kind) void run(() => setWidgetLocked(kind, !snapshot.value.settings.widgets[kind].locked));
}

onMounted(() => window.addEventListener("keydown", onGlobalKeydown));
onUnmounted(() => {
  window.removeEventListener("keydown", onGlobalKeydown);
  if (errorTimer) clearTimeout(errorTimer);
});
</script>

<template>
  <div class="manager-shell">
    <aside class="manager-sidebar">
      <label class="sidebar-search">
        <input ref="searchInput" v-model="search" aria-label="查找设置" placeholder="查找设置" />
        <AppIcon name="search" :size="15" />
      </label>

      <nav class="sidebar-nav" aria-label="偏好设置导航">
        <template v-for="(group, index) in visibleGroups" :key="index">
          <div v-if="index > 0" class="nav-separator" role="separator"></div>
          <button
            v-for="item in group"
            :key="item.id"
            class="nav-item"
            :aria-label="item.label"
            :class="{ active: activePage === item.id }"
            :aria-current="activePage === item.id ? 'page' : undefined"
            @click="activePage = item.id"
          >
            <AppIcon :name="item.icon" :size="16" />
            <span>{{ item.label }}</span>
          </button>
        </template>
        <div v-if="visibleGroups.length === 0" class="nav-empty">没有匹配的设置</div>
      </nav>
    </aside>

    <main class="manager-main">
      <div class="manager-scroll">
        <div class="manager-page">
          <h1 class="page-title">{{ pageTitle }}</h1>

          <transition name="notice">
            <div v-if="errorMessage || storeError" class="info-bar" role="alert">
              <span class="info-bar-icon">!</span>
              <span>{{ errorMessage || storeError }}</span>
            </div>
          </transition>

          <template v-if="activePage === 'home'">
            <p class="page-subtitle">{{ enabledCount }} 个组件显示在桌面上</p>
            <div class="settings-group">
              <div v-for="kind in widgetKinds" :key="kind" class="settings-card clickable">
                <button class="card-link" :aria-label="`${widgetMeta[kind].label}设置`" @click="activePage = kind"></button>
                <span class="card-icon-tile" :class="kind"><AppIcon :name="widgetMeta[kind].icon" :size="18" /></span>
                <div class="card-text">
                  <strong>{{ widgetMeta[kind].label }}</strong>
                  <span>{{ widgetMeta[kind].description }}</span>
                </div>
                <div class="card-control">
                  <span class="toggle-state">{{ snapshot.settings.widgets[kind].enabled ? '开' : '关' }}</span>
                  <button
                    class="toggle-switch"
                    :class="{ on: snapshot.settings.widgets[kind].enabled }"
                    role="switch"
                    :aria-checked="snapshot.settings.widgets[kind].enabled"
                    :aria-label="`在桌面显示${widgetMeta[kind].label}`"
                    @click="toggleWidget(kind)"
                  ><span></span></button>
                  <AppIcon class="card-chevron" name="chevron-right" :size="16" />
                </div>
              </div>
            </div>

            <div class="settings-card hint-card">
              <AppIcon class="card-icon" name="info" :size="18" />
              <div class="card-text">
                <strong>从桌面打开偏好设置</strong>
                <span>右键任意组件，选择“Vela 偏好设置”即可回到这里。</span>
              </div>
            </div>
          </template>

          <template v-else-if="settingKind && currentWidget">
            <div class="settings-group">
              <div class="settings-card">
                <AppIcon class="card-icon" name="grid" :size="18" />
                <div class="card-text">
                  <strong>在桌面显示</strong>
                  <span>关闭后组件会从桌面隐藏，内容不会丢失</span>
                </div>
                <div class="card-control">
                  <span class="toggle-state">{{ currentWidget.enabled ? '开' : '关' }}</span>
                  <button class="toggle-switch" :class="{ on: currentWidget.enabled }" role="switch" :aria-checked="currentWidget.enabled" aria-label="在桌面显示" @click="toggleWidget(settingKind)"><span></span></button>
                </div>
              </div>
              <div class="settings-card">
                <AppIcon class="card-icon" name="arrow-up-right" :size="18" />
                <div class="card-text">
                  <strong>窗口层级</strong>
                  <span>置顶后组件会显示在其他窗口上方</span>
                </div>
                <div class="card-control">
                  <VelaSelect :model-value="currentWidget.alwaysOnTop ? 'top' : 'normal'" label="窗口层级" :options="[{ value: 'normal', label: '普通层级' }, { value: 'top', label: '始终置顶' }]" @update:model-value="onLayerChange" />
                </div>
              </div>
              <div class="settings-card">
                <AppIcon class="card-icon" name="grid" :size="18" />
                <div class="card-text">
                  <strong>组件尺寸</strong>
                  <span>小、中、大三种布局，也可以在组件右键菜单中切换</span>
                </div>
                <div class="card-control">
                  <VelaSelect :model-value="currentWidget.size" label="组件尺寸" :options="widgetSizeOptions" @update:model-value="onSizeChange" />
                </div>
              </div>
              <div class="settings-card">
                <AppIcon class="card-icon" name="lock" :size="18" />
                <div class="card-text">
                  <strong>锁定位置</strong>
                  <span>防止拖动时误移组件</span>
                </div>
                <div class="card-control">
                  <span class="toggle-state">{{ currentWidget.locked ? '开' : '关' }}</span>
                  <button class="toggle-switch" :class="{ on: currentWidget.locked }" role="switch" :aria-checked="currentWidget.locked" aria-label="锁定位置" @click="toggleLocked"><span></span></button>
                </div>
              </div>
            </div>

            <WidgetPreferences v-if="['clock', 'note', 'countdown'].includes(settingKind)" :key="settingKind" :kind="settingKind" />

            <template v-if="settingKind === 'calendar'">
              <CalendarSchedulePreferences />
              <h2 class="section-title">日历</h2>
              <div class="settings-group">
                <div class="settings-card">
                  <AppIcon class="card-icon" name="calendar" :size="18" />
                  <div class="card-text">
                    <strong>一周的第一天</strong>
                    <span>决定月历每一行从星期几开始</span>
                  </div>
                  <div class="card-control">
                    <VelaSelect :model-value="snapshot.settings.weekStartsMonday ? 'monday' : 'sunday'" label="一周的第一天" :options="[{ value: 'monday', label: '星期一' }, { value: 'sunday', label: '星期日' }]" @update:model-value="onWeekStartChange" />
                  </div>
                </div>
              </div>
              <CalendarPreferences />
            </template>
          </template>

          <template v-else-if="activePage === 'appearance'">
            <h2 class="section-title first">颜色</h2>
            <div class="settings-group">
              <div class="settings-card stacked">
                <div class="card-row">
                  <AppIcon class="card-icon" name="sun" :size="18" />
                  <div class="card-text">
                    <strong>颜色模式</strong>
                    <span>同时作用于桌面组件和偏好设置窗口</span>
                  </div>
                </div>
                <div class="theme-options">
                  <button v-for="option in themeOptions" :key="option.id" class="theme-option" :class="{ selected: snapshot.settings.theme === option.id }" :aria-pressed="snapshot.settings.theme === option.id" @click="onThemeChange(option.id)">
                    <WidgetThemePreview :theme="option.id" :selected="snapshot.settings.theme === option.id" />
                    <span>{{ option.label }}</span>
                  </button>
                </div>
              </div>
              <div class="settings-card">
                <AppIcon class="card-icon" name="spark" :size="18" />
                <div class="card-text">
                  <strong>强调色</strong>
                  <span>用于开关、选中状态和组件高亮</span>
                </div>
                <div class="card-control accent-control">
                  <button
                    v-for="color in accentColors"
                    :key="color"
                    class="accent-swatch"
                    :class="{ selected: snapshot.settings.accentColor.toLowerCase() === color }"
                    :style="{ backgroundColor: color }"
                    type="button"
                    :aria-label="`选择强调色 ${color}`"
                    :aria-pressed="snapshot.settings.accentColor.toLowerCase() === color"
                    @click="chooseAccentColor(color)"
                  ></button>
                  <label class="accent-custom" data-tooltip="自定义强调色">
                    <input type="color" :value="snapshot.settings.accentColor" aria-label="自定义强调色" @change="onAccentChange" />
                  </label>
                </div>
              </div>
            </div>

            <h2 class="section-title">桌面组件</h2>
            <div class="settings-group">
              <div class="settings-card">
                <AppIcon class="card-icon" name="grid" :size="18" />
                <div class="card-text">
                  <strong>圆角</strong>
                  <span>组件窗口四角的弧度</span>
                </div>
                <div class="card-control range-control">
                  <output>{{ cornerRadiusDraft }} px</output>
                  <VelaSlider v-model="cornerRadiusDraft" :min="8" :max="30" :step="1" label="组件圆角" @value-commit="commitAppearance" />
                </div>
              </div>
              <div class="settings-card">
                <AppIcon class="card-icon" name="sliders" :size="18" />
                <div class="card-text">
                  <strong>背景透明度</strong>
                  <span>数值越高，越能透过组件看到桌面</span>
                </div>
                <div class="card-control range-control">
                  <output>{{ transparencyDraft }}%</output>
                  <VelaSlider v-model="transparencyDraft" :min="0" :max="100" :step="1" label="背景透明度" @value-commit="commitAppearance" />
                </div>
              </div>
            </div>
          </template>

          <template v-else-if="activePage === 'startup'">
            <div class="settings-group">
              <div class="settings-card disabled">
                <AppIcon class="card-icon" name="power" :size="18" />
                <div class="card-text">
                  <strong>登录 Windows 时启动</strong>
                  <span>自动恢复桌面组件，不会打开这个窗口。当前版本暂不可用。</span>
                </div>
                <div class="card-control">
                  <span class="toggle-state">关</span>
                  <button class="toggle-switch" role="switch" aria-checked="false" aria-label="登录 Windows 时启动" disabled><span></span></button>
                </div>
              </div>
            </div>
          </template>

          <template v-else-if="activePage === 'about'">
            <div class="settings-group">
              <div class="settings-card about-card">
                <div class="brand-mark"><span></span><span></span><span></span></div>
                <div class="card-text">
                  <strong>Vela Widgets</strong>
                  <span>版本 0.1.0</span>
                </div>
              </div>
              <div class="settings-card">
                <AppIcon class="card-icon" name="database" :size="18" />
                <div class="card-text">
                  <strong>数据保存在这台电脑上</strong>
                  <span>便签、待办、重要日子和设置只存储在本机，不会上传到任何服务器</span>
                </div>
              </div>
            </div>

            <h2 class="section-title">Vela</h2>
            <div class="settings-group">
              <div class="settings-card">
                <AppIcon class="card-icon" name="power" :size="18" />
                <div class="card-text">
                  <strong>退出 Vela</strong>
                  <span>关闭所有桌面组件并停止后台运行</span>
                </div>
                <div class="card-control">
                  <button class="win-button" @click="run(exitVela)">退出</button>
                </div>
              </div>
            </div>
          </template>
        </div>
      </div>
    </main>
  </div>
</template>
