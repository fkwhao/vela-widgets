<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import AppIcon from "./AppIcon.vue";
import WidgetThemePreview from "./WidgetThemePreview.vue";
import VelaSelect from "./VelaSelect.vue";
import VelaSlider from "./VelaSlider.vue";
import { exitVela, isNativeApp } from "../lib/backend";
import {
  setAccentColor,
  setTheme,
  setWeekStartsMonday,
  setWidgetEnabled,
  setWidgetAppearance,
  setWidgetLayer,
  setWidgetLocked,
  snapshot,
  storeError,
} from "../lib/store";
import type { ThemeMode, WidgetKind } from "../types";

const search = ref("");
const searchInput = ref<HTMLInputElement | null>(null);
const activePage = ref("home");
const toast = ref("");
const transparencyDraft = ref(snapshot.value.settings.widgetTransparency);
const cornerRadiusDraft = ref(snapshot.value.settings.widgetCornerRadius);
const nativeApp = isNativeApp();
let toastTimer: ReturnType<typeof setTimeout> | undefined;

watch(
  () => [snapshot.value.settings.widgetTransparency, snapshot.value.settings.widgetCornerRadius],
  ([transparency, radius]) => {
    transparencyDraft.value = transparency;
    cornerRadiusDraft.value = radius;
  },
);

const navGroups = [
  {
    label: "组件",
    items: [
      { id: "home", label: "我的组件", icon: "grid" },
      { id: "calendar", label: "日历", icon: "calendar" },
      { id: "todo", label: "待办", icon: "check" },
    ],
  },
  {
    label: "常规",
    items: [
      { id: "appearance", label: "全局设置", icon: "sun" },
      { id: "behavior", label: "组件行为", icon: "sliders" },
      { id: "startup", label: "启动", icon: "power" },
      { id: "data", label: "数据与关于", icon: "database" },
    ],
  },
];

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
const pageTitle = computed(() => {
  const item = navGroups.flatMap((group) => group.items).find((entry) => entry.id === activePage.value);
  return item?.label ?? "我的组件";
});
const visibleGroups = computed(() =>
  navGroups
    .map((group) => ({
      ...group,
      items: group.items.filter((item) => item.label.toLowerCase().includes(search.value.trim().toLowerCase())),
    }))
    .filter((group) => group.items.length > 0),
);
const settingKind = computed<WidgetKind | null>(() =>
  activePage.value === "calendar" || activePage.value === "todo" ? activePage.value : null,
);
const currentWidget = computed(() =>
  settingKind.value ? snapshot.value.settings.widgets[settingKind.value] : null,
);
const isHome = computed(() => activePage.value === "home");

function onGlobalKeydown(event: KeyboardEvent): void {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
    event.preventDefault();
    searchInput.value?.focus();
  }
}

function widgetIsEnabled(id: string): boolean {
  if (id === "calendar" || id === "todo") return snapshot.value.settings.widgets[id].enabled;
  return false;
}

function toggleCurrentWidget(): void {
  if (settingKind.value) void toggleWidget(settingKind.value);
}

function showToast(message: string): void {
  toast.value = message;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (toast.value = ""), 2600);
}

async function run(action: () => Promise<void>, successMessage?: string): Promise<void> {
  try {
    await action();
    if (successMessage) showToast(successMessage);
  } catch (error) {
    showToast(typeof error === "string" ? error : "没有保存成功，请稍后再试。 ");
  }
}

async function toggleWidget(kind: WidgetKind): Promise<void> {
  const enabled = snapshot.value.settings.widgets[kind].enabled;
  if (enabled && enabledCount.value <= 1) {
    showToast("至少保留一个桌面组件，才能从桌面打开 Vela 偏好设置。");
    return;
  }
  await run(
    () => setWidgetEnabled(kind, !enabled),
    enabled
      ? "组件已关闭"
      : nativeApp
        ? "组件已添加到桌面"
        : "预览已开启",
  );
}

function onThemeChange(theme: ThemeMode): void {
  void run(() => setTheme(theme), "外观已更新");
}

function onAccentChange(event: Event): void {
  const color = (event.target as HTMLInputElement).value;
  void run(() => setAccentColor(color), "强调色已更新");
}

function chooseAccentColor(color: string): void {
  void run(() => setAccentColor(color), "强调色已更新");
}

function onTransparencyInput(value: number): void {
  transparencyDraft.value = value;
}

function onTransparencyChange(): void {
  void run(
    () => setWidgetAppearance(transparencyDraft.value, cornerRadiusDraft.value),
    "组件透明度已更新",
  );
}

function onCornerRadiusInput(value: number): void {
  cornerRadiusDraft.value = value;
}

function onCornerRadiusChange(): void {
  void run(
    () => setWidgetAppearance(transparencyDraft.value, cornerRadiusDraft.value),
    "组件圆角已更新",
  );
}

function onWeekStartChange(value: string): void {
  const monday = value === "monday";
  void run(() => setWeekStartsMonday(monday), "日历设置已保存");
}

function onLayerChange(value: string): void {
  if (!settingKind.value) return;
  const alwaysOnTop = value === "top";
  void run(() => setWidgetLayer(settingKind.value!, alwaysOnTop), "窗口层级已更新");
}

function onLockChange(event: Event): void {
  if (!settingKind.value) return;
  const locked = (event.target as HTMLInputElement).checked;
  void run(() => setWidgetLocked(settingKind.value!, locked), locked ? "组件位置已锁定" : "组件位置已解锁");
}

onMounted(() => window.addEventListener("keydown", onGlobalKeydown));
onUnmounted(() => window.removeEventListener("keydown", onGlobalKeydown));
</script>

<template>
  <div class="manager-shell">
    <aside class="manager-sidebar">
      <div class="brand-lockup">
        <div class="brand-mark"><span></span><span></span><span></span></div>
        <div>
          <strong>Vela</strong>
        </div>
      </div>

      <label class="sidebar-search">
        <AppIcon name="search" :size="17" />
        <input ref="searchInput" v-model="search" aria-label="搜索偏好设置" placeholder="搜索偏好设置" />
      </label>

      <nav class="sidebar-nav" aria-label="中控导航">
        <section v-for="group in visibleGroups" :key="group.label" class="nav-group">
          <div class="nav-group-title">{{ group.label }}</div>
          <button
            v-for="item in group.items"
            :key="item.id"
            class="nav-item"
            :class="{ active: activePage === item.id }"
            @click="activePage = item.id"
          >
            <AppIcon :name="item.icon" :size="17" />
            <span>{{ item.label }}</span>
              <i v-if="item.id === 'calendar' || item.id === 'todo'" class="nav-status" :class="{ enabled: widgetIsEnabled(item.id) }"></i>
          </button>
        </section>
        <div v-if="visibleGroups.length === 0" class="nav-empty">没有匹配的设置</div>
      </nav>

    </aside>

    <main class="manager-main">
      <header class="manager-header">
        <div class="breadcrumbs">{{ pageTitle }}</div>
        <button class="text-button exit-button" @click="void exitVela()">
          <AppIcon name="power" :size="15" />
          退出 Vela
        </button>
      </header>

      <div class="manager-scroll">
        <div v-if="storeError" class="store-error"><AppIcon name="info" :size="17" />{{ storeError }}</div>

        <template v-if="isHome">
          <section class="page-intro">
            <div class="intro-row">
              <div>
                <h1>桌面组件</h1>
              </div>
              <div class="enabled-count"><b>{{ enabledCount }}</b><span>个已开启</span></div>
            </div>
          </section>

          <section class="home-section">
            <div class="section-heading">
              <div>
                <h2>我的组件</h2>
              </div>
            </div>

            <div class="component-list">
              <article v-for="kind in (['calendar', 'todo'] as const)" :key="kind" class="component-row">
                <div class="component-icon" :class="kind">
                  <AppIcon :name="kind === 'calendar' ? 'calendar' : 'check'" :size="21" />
                </div>
                <div class="component-copy">
                  <div class="component-titleline">
                    <h3>{{ kind === 'calendar' ? '日历' : '待办' }}</h3>
                    <span class="component-state" :class="{ running: snapshot.settings.widgets[kind].enabled }">
                      <i></i>{{ snapshot.settings.widgets[kind].enabled ? (nativeApp ? '桌面上运行' : '预览已开启') : '未启用' }}
                    </span>
                  </div>
                </div>
                <div class="component-actions">
                  <button class="configure-button" @click="activePage = kind">设置 <AppIcon name="chevron-right" :size="14" /></button>
                  <button
                    class="switch-control"
                    :class="{ on: snapshot.settings.widgets[kind].enabled }"
                    role="switch"
                    :aria-checked="snapshot.settings.widgets[kind].enabled"
                    :aria-label="`${snapshot.settings.widgets[kind].enabled ? '关闭' : '开启'}${kind === 'calendar' ? '日历' : '待办'}`"
                    @click="toggleWidget(kind)"
                  ><span></span></button>
                </div>
              </article>
            </div>
          </section>

        </template>

        <template v-else-if="activePage === 'appearance'">
          <section class="settings-panel">
            <div class="settings-panel-heading"><h2>全局界面颜色模式</h2></div>
            <div class="theme-options">
              <button v-for="option in ([{ id: 'light', label: '浅色' }, { id: 'dark', label: '深色' }, { id: 'system', label: '跟随系统' }] as const)" :key="option.id" class="theme-option" :class="{ selected: snapshot.settings.theme === option.id }" @click="onThemeChange(option.id)">
                <WidgetThemePreview :theme="option.id" :selected="snapshot.settings.theme === option.id" />
                <strong>{{ option.label }}</strong>
              </button>
            </div>
            <div class="setting-divider"></div>
            <div class="setting-line">
              <div><strong>全局界面强调色</strong></div>
              <div class="accent-control">
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
                <label class="accent-custom" title="自定义强调色">
                  <input type="color" :value="snapshot.settings.accentColor" aria-label="自定义强调色" @change="onAccentChange" />
                </label>
              </div>
            </div>
          </section>
          <section class="settings-panel compact-panel">
            <div class="setting-line range-setting-line">
              <div><strong>组件圆角</strong></div>
              <div class="range-control"><VelaSlider :model-value="cornerRadiusDraft" :min="8" :max="30" :step="1" label="组件圆角" @update:model-value="onCornerRadiusInput" @value-commit="onCornerRadiusChange" /><output>{{ cornerRadiusDraft }} px</output></div>
            </div>
            <div class="setting-divider"></div>
            <div class="setting-line range-setting-line">
              <div><strong>背景透明度</strong></div>
              <div class="range-control"><VelaSlider :model-value="transparencyDraft" :min="0" :max="100" :step="1" label="背景透明度" @update:model-value="onTransparencyInput" @value-commit="onTransparencyChange" /><output>{{ transparencyDraft }}%</output></div>
            </div>
          </section>
        </template>

        <template v-else-if="settingKind && currentWidget">
          <section class="settings-panel">
            <div class="setting-line">
              <div><strong>在桌面显示</strong></div>
              <button class="switch-control" :class="{ on: currentWidget.enabled }" role="switch" :aria-checked="currentWidget.enabled" @click="toggleCurrentWidget"><span></span></button>
            </div>
            <div class="setting-divider"></div>
            <div class="setting-line">
              <div><strong>窗口层级</strong></div>
              <VelaSelect :model-value="currentWidget.alwaysOnTop ? 'top' : 'normal'" label="窗口层级" :options="[{ value: 'normal', label: '普通层级' }, { value: 'top', label: '始终置顶' }]" @update:model-value="onLayerChange" />
            </div>
            <div class="setting-divider"></div>
            <div class="setting-line">
              <div><strong>锁定位置和大小</strong></div>
              <input class="checkbox-control" type="checkbox" :checked="currentWidget.locked" @change="onLockChange" />
            </div>
            <template v-if="settingKind === 'calendar'">
              <div class="setting-divider"></div>
              <div class="setting-line">
                <div><strong>一周从哪天开始</strong></div>
                <VelaSelect :model-value="snapshot.settings.weekStartsMonday ? 'monday' : 'sunday'" label="一周从哪天开始" :options="[{ value: 'monday', label: '星期一' }, { value: 'sunday', label: '星期日' }]" @update:model-value="onWeekStartChange" />
              </div>
            </template>
          </section>
        </template>

        <template v-else-if="activePage === 'behavior'">
          <section class="settings-panel">
            <div class="settings-panel-heading"><h2>中控入口</h2></div>
            <div class="entry-preview"><span class="entry-preview-icon"><AppIcon name="more" :size="18" /></span><div><strong>Vela 偏好设置</strong></div></div>
            <div class="setting-divider"></div>
            <div class="setting-line"><div><strong>桌面快捷菜单</strong></div><span class="status-pill">已启用</span></div>
          </section>
        </template>

        <template v-else-if="activePage === 'startup'">
          <section class="settings-panel">
            <div class="setting-line disabled-setting"><div><strong>登录后自动启动</strong></div><span class="coming-soon">后续接入</span></div>
          </section>
        </template>

        <template v-else-if="activePage === 'data'">
          <section class="settings-panel data-card">
            <div class="data-icon"><AppIcon name="database" :size="21" /></div>
            <div><strong>本地 SQLite 数据库</strong></div>
          </section>
          <section class="settings-panel compact-panel">
            <div class="setting-line"><div><strong>版本</strong></div><span class="static-value">0.1.0</span></div>
            <div class="setting-divider"></div>
            <div class="setting-line"><div><strong>备份与导出</strong></div><span class="coming-soon">规划中</span></div>
          </section>
        </template>
      </div>

      <transition name="toast"><div v-if="toast" class="toast-message"><span class="toast-check">✓</span>{{ toast }}</div></transition>
    </main>
  </div>
</template>
