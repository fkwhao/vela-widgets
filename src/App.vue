<script setup lang="ts">
import { computed, onMounted, onUnmounted, watch } from "vue";
import { listen } from "@tauri-apps/api/event";
import CalendarWidget from "./components/CalendarWidget.vue";
import ContextMenuWindow from "./components/ContextMenuWindow.vue";
import ManagerView from "./components/ManagerView.vue";
import TodoWidget from "./components/TodoWidget.vue";
import { openManager } from "./lib/backend";
import { applySnapshot, refreshSnapshot, snapshot } from "./lib/store";
import { isNativeApp } from "./lib/backend";
import type { AppSnapshot } from "./types";

const view = new URLSearchParams(window.location.search).get("view") ?? "manager";
const isManager = computed(() => view === "manager");
const systemThemeQuery = window.matchMedia?.("(prefers-color-scheme: dark)");
let pixelRatioQuery: MediaQueryList | undefined;
let unlistenSnapshot: (() => void) | undefined;

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
    void refreshSnapshot();
  }
});

onUnmounted(() => {
  window.removeEventListener("resize", syncWidgetPixelGrid);
  pixelRatioQuery?.removeEventListener("change", syncWidgetPixelGrid);
  window.removeEventListener("focus", refreshOnFocus);
  systemThemeQuery?.removeEventListener("change", applyTheme);
  unlistenSnapshot?.();
});
</script>

<template>
  <ManagerView v-if="isManager" />
  <CalendarWidget v-else-if="view === 'calendar'" />
  <TodoWidget v-else-if="view === 'todo'" />
  <ContextMenuWindow v-else-if="view === 'context-menu'" />
  <main v-else class="unknown-view">
    <p>找不到这个 Vela 窗口。</p>
    <button class="button-primary" @click="openManagerPreview">打开中控</button>
  </main>
</template>
