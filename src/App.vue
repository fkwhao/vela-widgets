<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, onUnmounted, watch } from "vue";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import CalendarWidget from "./components/CalendarWidget.vue";
import HoverHint from "./components/HoverHint.vue";
import ContextMenuWindow from "./components/ContextMenuWindow.vue";
import ManagerView from "./components/ManagerView.vue";
import ClockWidget from "./components/ClockWidget.vue";
const NoteWidget = defineAsyncComponent(() => import("./components/NoteWidget.vue"));
import CountdownWidget from "./components/CountdownWidget.vue";
import TodoWidget from "./components/TodoWidget.vue";
import { openManager, checkHolidayUpdates } from "./lib/backend";
import { applySnapshot, refreshSnapshot, snapshot } from "./lib/store";
import { isNativeApp } from "./lib/backend";
import type { AppSnapshot } from "./types";

const query = new URLSearchParams(window.location.search);
const view = query.get("view") ?? "manager";
// Set by the native side only when the manager window actually has Mica.
if (query.get("backdrop") === "mica") document.documentElement.dataset.backdrop = "mica";
const isManager = computed(() => view === "manager");
const systemThemeQuery = window.matchMedia?.("(prefers-color-scheme: dark)");
let pixelRatioQuery: MediaQueryList | undefined;
let unlistenSnapshot: (() => void) | undefined;
let holidayTimer: ReturnType<typeof setInterval> | undefined;
function onPreviewHolidays(event: Event): void { applySnapshot((event as CustomEvent<AppSnapshot>).detail); }
function checkPreviewHolidays(): void { if (!isNativeApp()) void checkHolidayUpdates(true).catch(() => undefined); }
watch(() => snapshot.value.settings.calendar.autoUpdate, enabled => { if (enabled) checkPreviewHolidays(); });

function syncWidgetPixelGrid(): void {
  const scale = window.devicePixelRatio || 1;
  // Keep the complete outline one physical pixel inside the WebView bounds.
  document.documentElement.style.setProperty("--widget-edge-inset", `${1 / scale}px`);
  pixelRatioQuery?.removeEventListener("change", syncWidgetPixelGrid);
  pixelRatioQuery = window.matchMedia(`(resolution: ${scale}dppx)`);
  pixelRatioQuery.addEventListener("change", syncWidgetPixelGrid);
}

function applyTheme(): void {
  const configured = snapshot.value.settings.theme;
  const systemDark = systemThemeQuery?.matches ?? false;
  const resolved = configured === "system" ? (systemDark ? "dark" : "light") : configured;
  document.documentElement.dataset.theme = resolved;
  if (isManager.value && isNativeApp()) {
    // Keep the native title bar and Mica tint in step with Vela's own theme.
    void getCurrentWindow().setTheme(configured === "system" ? null : resolved).catch(() => undefined);
  }
  document.documentElement.style.setProperty("--accent", snapshot.value.settings.accentColor);
  const transparency = Math.min(100, Math.max(0, snapshot.value.settings.widgetTransparency));
  const cornerRadius = Math.min(30, Math.max(8, snapshot.value.settings.widgetCornerRadius));
  document.documentElement.style.setProperty("--widget-background-alpha", String(1 - transparency / 100));
  document.documentElement.style.setProperty("--widget-radius", `${cornerRadius}px`);
}

function refreshOnFocus(): void {
  void refreshSnapshot();
}

function openManagerPreview(): void {
  void openManager();
}

watch(() => snapshot.value.settings, applyTheme, { deep: true, immediate: true });

onMounted(() => {
  syncWidgetPixelGrid();
  window.addEventListener("resize", syncWidgetPixelGrid);
  window.addEventListener("focus", refreshOnFocus);
  systemThemeQuery?.addEventListener("change", applyTheme);
  if (isNativeApp()) {
    void listen<AppSnapshot>("vela://snapshot-updated", (event) => applySnapshot(event.payload))
      .then((unlisten) => {
        unlistenSnapshot = unlisten;
        return refreshSnapshot();
      })
      .catch(() => refreshSnapshot());
  } else {
    window.addEventListener("vela:holidays-updated", onPreviewHolidays);
    window.addEventListener("storage", refreshOnFocus);
    void refreshSnapshot().then(checkPreviewHolidays);
    holidayTimer = setInterval(checkPreviewHolidays, 60_000);
  }
});

onUnmounted(() => {
  window.removeEventListener("resize", syncWidgetPixelGrid);
  pixelRatioQuery?.removeEventListener("change", syncWidgetPixelGrid);
  window.removeEventListener("focus", refreshOnFocus);
  systemThemeQuery?.removeEventListener("change", applyTheme);
  unlistenSnapshot?.();
  window.removeEventListener("vela:holidays-updated", onPreviewHolidays);
  window.removeEventListener("storage", refreshOnFocus);
  if (holidayTimer) clearInterval(holidayTimer);
});
</script>

<template>
  <HoverHint :manager="isManager" />
  <ManagerView v-if="isManager" />
  <CalendarWidget v-else-if="view === 'calendar'" />
  <TodoWidget v-else-if="view === 'todo'" />
  <ClockWidget v-else-if="view === 'clock'" />
  <NoteWidget v-else-if="view === 'note'" />
  <CountdownWidget v-else-if="view === 'countdown'" />
  <ContextMenuWindow v-else-if="view === 'context-menu'" />
  <main v-else class="unknown-view">
    <p>找不到这个 Vela 窗口。</p>
    <button class="button-primary" @click="openManagerPreview">打开中控</button>
  </main>
</template>
