<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import AppIcon from "./AppIcon.vue";
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
      { id: "appearance", label: "外观", icon: "sun" },
      { id: "behavior", label: "组件行为", icon: "sliders" },
      { id: "startup", label: "启动", icon: "power" },
      { id: "data", label: "数据与关于", icon: "database" },
    ],
  },
];

const widgetDescriptions: Record<WidgetKind, string> = {
  calendar: "日期与月历，打开桌面即可查看今天。",
  todo: "轻量记录，随手勾选和整理下一步。",
};

const enabledCount = computed(
  () => Object.values(snapshot.value.settings.widgets).filter((widget) => widget.enabled).length,
);
const openTodoCount = computed(
  () => snapshot.value.todos.filter((todo) => !todo.completed).length,
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
        : "预览状态已更新；运行 npm start 才会创建桌面窗口",
  );
}

function onThemeChange(theme: ThemeMode): void {
  void run(() => setTheme(theme), "外观已更新");
}

function onAccentChange(event: Event): void {
  const color = (event.target as HTMLInputElement).value;
  void run(() => setAccentColor(color), "强调色已更新");
}

function onTransparencyInput(event: Event): void {
  transparencyDraft.value = Number((event.target as HTMLInputElement).value);
}

function onTransparencyChange(): void {
  void run(
    () => setWidgetAppearance(transparencyDraft.value, cornerRadiusDraft.value),
    "组件透明度已更新",
  );
}

function onCornerRadiusInput(event: Event): void {
  cornerRadiusDraft.value = Number((event.target as HTMLInputElement).value);
}

function onCornerRadiusChange(): void {
  void run(
    () => setWidgetAppearance(transparencyDraft.value, cornerRadiusDraft.value),
    "组件圆角已更新",
  );
}

function onWeekStartChange(event: Event): void {
  const monday = (event.target as HTMLSelectElement).value === "monday";
  void run(() => setWeekStartsMonday(monday), "日历设置已保存");
}

function onLayerChange(event: Event): void {
  if (!settingKind.value) return;
  const alwaysOnTop = (event.target as HTMLSelectElement).value === "top";
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
          <small>DESKTOP COMPANION</small>
        </div>
      </div>

      <label class="sidebar-search">
        <AppIcon name="search" :size="17" />
        <input ref="searchInput" v-model="search" aria-label="搜索偏好设置" placeholder="搜索偏好设置" />
        <kbd>Ctrl K</kbd>
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

      <div class="sidebar-footer">
        <span class="local-mark"><i></i> 本地运行</span>
        <span class="sidebar-version">EARLY ACCESS · 0.1</span>
      </div>
    </aside>

    <main class="manager-main">
      <header class="manager-header">
        <div class="breadcrumbs"><span>VELA</span><b>/</b><span>{{ pageTitle }}</span></div>
        <button class="text-button exit-button" @click="void exitVela()">
          <AppIcon name="power" :size="15" />
          退出 Vela
        </button>
      </header>

      <div class="manager-scroll">
        <div v-if="!nativeApp" class="preview-mode-note" role="status">
          浏览器预览模式：开关只保存预览状态，不会创建桌面窗口。请运行 <code>npm start</code> 使用原生组件与拖动功能。
        </div>
        <div v-if="storeError" class="store-error"><AppIcon name="info" :size="17" />{{ storeError }}</div>

        <template v-if="isHome">
          <section class="page-intro">
            <div class="eyebrow"><span class="eyebrow-line"></span> YOUR DESKTOP</div>
            <div class="intro-row">
              <div>
                <h1>桌面组件</h1>
                <p>把常用信息放在桌面上，简洁地看见今天。</p>
              </div>
              <div class="enabled-count"><b>{{ enabledCount }}</b><span>/ 2 已启用</span></div>
            </div>
          </section>

          <section class="home-section">
            <div class="section-heading">
              <div>
                <span class="section-kicker">YOUR SPACE</span>
                <h2>我的组件</h2>
              </div>
              <span class="quiet-label"><i class="live-dot"></i>状态已同步</span>
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
                  <p>{{ kind === 'todo' && snapshot.settings.widgets.todo.enabled ? `${openTodoCount} 项未完成 · ` : '' }}{{ widgetDescriptions[kind] }}</p>
                  <div class="component-meta">
                    <span><AppIcon name="arrow-up-right" :size="12" /> 独立窗口</span>
                    <span>{{ snapshot.settings.widgets[kind].alwaysOnTop ? '置顶' : '普通层级' }}</span>
                  </div>
                </div>
                <div class="component-actions">
                  <button class="configure-button" @click="activePage = kind">配置 <AppIcon name="chevron-right" :size="14" /></button>
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

          <section class="entry-note">
            <div class="note-symbol"><AppIcon name="info" :size="18" /></div>
            <div><strong>中控只在需要时出现</strong><p>在桌面组件上右键，选择“Vela 偏好设置”即可管理组件和全局设置。</p></div>
            <span class="note-spark"><AppIcon name="spark" :size="17" /></span>
          </section>

          <div class="manager-footnote"><span>DESIGNED TO STAY OUT OF THE WAY</span><span>Vela Widgets · 0.1</span></div>
        </template>

        <template v-else-if="activePage === 'appearance'">
          <section class="page-intro settings-intro">
            <div class="eyebrow"><span class="eyebrow-line"></span> PERSONALIZE</div>
            <h1>外观</h1>
            <p>为组件选择适合桌面的明暗与色彩。</p>
          </section>
          <section class="settings-panel">
            <div class="settings-panel-heading"><div><span class="section-kicker">SURFACE</span><h2>界面主题</h2></div><span class="panel-caption">即时生效</span></div>
            <div class="theme-options">
              <button v-for="option in ([{ id: 'light', label: '浅色', desc: '白 · 蓝' }, { id: 'dark', label: '深色', desc: '蓝灰 · 靛蓝' }, { id: 'system', label: '跟随系统', desc: '自动切换' }] as const)" :key="option.id" class="theme-option" :class="{ selected: snapshot.settings.theme === option.id }" @click="onThemeChange(option.id)">
                <span class="theme-swatch" :class="option.id"><i></i><i></i><i></i></span>
                <strong>{{ option.label }}</strong><small>{{ option.desc }}</small>
              </button>
            </div>
            <div class="setting-divider"></div>
            <div class="setting-line">
              <div><strong>强调色</strong><p>用于选中状态和组件中的重点操作。</p></div>
              <div class="accent-control">
                <span class="accent-dot" :style="{ backgroundColor: snapshot.settings.accentColor }"></span>
                <input type="color" :value="snapshot.settings.accentColor" aria-label="自定义强调色" @change="onAccentChange" />
                <span>自定义</span>
              </div>
            </div>
          </section>
          <section class="settings-panel compact-panel">
            <div class="setting-line range-setting-line">
              <div><strong>组件圆角</strong><p>应用到所有桌面组件。</p></div>
              <div class="range-control"><input type="range" min="8" max="30" step="1" :value="cornerRadiusDraft" aria-label="组件圆角" @input="onCornerRadiusInput" @change="onCornerRadiusChange" /><output>{{ cornerRadiusDraft }} px</output></div>
            </div>
            <div class="setting-divider"></div>
            <div class="setting-line range-setting-line">
              <div><strong>背景透明度</strong><p>原生毛玻璃；0% 不透明，100% 背景完全透明。</p></div>
              <div class="range-control"><input type="range" min="0" max="100" step="1" :value="transparencyDraft" aria-label="组件背景透明度" @input="onTransparencyInput" @change="onTransparencyChange" /><output>{{ transparencyDraft }}%</output></div>
            </div>
          </section>
        </template>

        <template v-else-if="settingKind && currentWidget">
          <section class="page-intro settings-intro">
            <div class="eyebrow"><span class="eyebrow-line"></span> WIDGET SETTINGS</div>
            <h1>{{ settingKind === 'calendar' ? '日历' : '待办' }}</h1>
            <p>{{ widgetDescriptions[settingKind] }}</p>
          </section>
          <section class="settings-panel">
            <div class="setting-line">
              <div><strong>在桌面显示</strong><p>打开后会创建独立、可移动的组件窗口。</p></div>
              <button class="switch-control" :class="{ on: currentWidget.enabled }" role="switch" :aria-checked="currentWidget.enabled" @click="toggleCurrentWidget"><span></span></button>
            </div>
            <div class="setting-divider"></div>
            <div class="setting-line">
              <div><strong>窗口层级</strong><p>置顶会显示在其他普通窗口上方。</p></div>
              <select :value="currentWidget.alwaysOnTop ? 'top' : 'normal'" class="select-control" @change="onLayerChange"><option value="normal">普通层级</option><option value="top">始终置顶</option></select>
            </div>
            <div class="setting-divider"></div>
            <div class="setting-line">
              <div><strong>锁定位置和大小</strong><p>锁定后仍可使用组件中的按钮和输入框。</p></div>
              <input class="checkbox-control" type="checkbox" :checked="currentWidget.locked" @change="onLockChange" />
            </div>
            <template v-if="settingKind === 'calendar'">
              <div class="setting-divider"></div>
              <div class="setting-line">
                <div><strong>一周从哪天开始</strong><p>选择月历中的第一列。</p></div>
                <select :value="snapshot.settings.weekStartsMonday ? 'monday' : 'sunday'" class="select-control" @change="onWeekStartChange"><option value="monday">星期一</option><option value="sunday">星期日</option></select>
              </div>
            </template>
          </section>
          <section class="settings-panel mini-stat-panel">
            <div class="mini-stat-icon"><AppIcon :name="settingKind === 'calendar' ? 'calendar' : 'check'" :size="19" /></div>
            <div><strong>独立桌面窗口</strong><p>可以单独移动和调整大小；窗口状态会保存在本机。</p></div>
          </section>
        </template>

        <template v-else-if="activePage === 'behavior'">
          <section class="page-intro settings-intro"><div class="eyebrow"><span class="eyebrow-line"></span> DESKTOP BEHAVIOR</div><h1>组件行为</h1><p>让组件在桌面上保持顺手，也容易找回。</p></section>
          <section class="settings-panel">
            <div class="settings-panel-heading"><div><span class="section-kicker">ACCESS</span><h2>中控入口</h2></div></div>
            <div class="entry-preview"><span class="entry-preview-icon"><AppIcon name="more" :size="18" /></span><div><strong>Vela 偏好设置</strong><p>在日历或待办组件上右键打开中控。</p></div><span class="entry-preview-tag">组件菜单</span></div>
            <div class="setting-divider"></div>
            <div class="setting-line"><div><strong>桌面快捷菜单</strong><p>入口固定在组件的右键菜单中。</p></div><span class="status-pill">已启用</span></div>
          </section>
          <section class="settings-panel note-panel"><AppIcon name="info" :size="18" /><p>组件置底属于 Windows 兼容模式，当前首版暂不提供，避免把普通窗口层级误认为桌面壁纸层。</p></section>
        </template>

        <template v-else-if="activePage === 'startup'">
          <section class="page-intro settings-intro"><div class="eyebrow"><span class="eyebrow-line"></span> STARTUP</div><h1>启动</h1><p>选择 Vela 在 Windows 登录后的行为。</p></section>
          <section class="settings-panel">
            <div class="setting-line disabled-setting"><div><strong>登录后自动启动</strong><p>启用后恢复上次开启的组件，不自动打开中控。</p></div><span class="coming-soon">后续接入</span></div>
            <div class="setting-divider"></div>
            <div class="launch-summary"><AppIcon name="clock" :size="18" /><span>当前不会随 Windows 登录自动启动。</span></div>
          </section>
          <div class="settings-footnote">启动项会通过 Windows 用户启动项管理；接入前不会展示可误触的开关。</div>
        </template>

        <template v-else-if="activePage === 'data'">
          <section class="page-intro settings-intro"><div class="eyebrow"><span class="eyebrow-line"></span> LOCAL DATA</div><h1>数据与关于</h1><p>组件配置和待办数据保存在这台电脑上。</p></section>
          <section class="settings-panel data-card">
            <div class="data-icon"><AppIcon name="database" :size="21" /></div>
            <div><strong>本地 SQLite 数据库</strong><p>窗口设置、外观偏好和待办内容由 Rust 后端统一保存。</p><span class="data-location">用户应用数据目录 · vela.sqlite3</span></div>
            <span class="data-tag">LOCAL</span>
          </section>
          <section class="settings-panel compact-panel">
            <div class="setting-line"><div><strong>版本</strong><p>Vela Widgets · 早期开发版</p></div><span class="static-value">0.1.0</span></div>
            <div class="setting-divider"></div>
            <div class="setting-line"><div><strong>备份与导出</strong><p>导出待办和迁移数据将在后续版本加入。</p></div><span class="coming-soon">规划中</span></div>
          </section>
        </template>
      </div>

      <transition name="toast"><div v-if="toast" class="toast-message"><span class="toast-check">✓</span>{{ toast }}</div></transition>
    </main>
  </div>
</template>
