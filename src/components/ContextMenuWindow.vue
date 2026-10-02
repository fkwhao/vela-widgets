<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { emitTo, listen } from "@tauri-apps/api/event";
import AppIcon from "./AppIcon.vue";
import WidgetSizeMenuRow from "./WidgetSizeMenuRow.vue";
import { getSnapshot, openManager } from "../lib/backend";
import { refreshSnapshot, setWidgetEnabled, setWidgetLayer, setWidgetSize } from "../lib/store";
import type { WidgetKind, WidgetSize } from "../types";

const widgetKind = ref<WidgetKind>("calendar");
const alwaysOnTop = ref(false);
const widgetSize = ref<WidgetSize>("large");
const sizeFailed = ref(false);
const closeBlocked = ref(false);
const layerFailed = ref(false);
let unlistenKind: (() => void) | undefined;
let closeBlockedTimer: ReturnType<typeof setTimeout> | undefined;
let layerFailedTimer: ReturnType<typeof setTimeout> | undefined;

function isWidgetKind(value: unknown): value is WidgetKind {
  return value === "calendar" || value === "todo";
}

const widgetTitle = () => widgetKind.value === "calendar" ? "日历" : "待办";

async function refreshLayerState(): Promise<void> {
  try {
    const current = await getSnapshot();
    alwaysOnTop.value = current.settings.widgets[widgetKind.value].alwaysOnTop;
    widgetSize.value = current.settings.widgets[widgetKind.value].size;
  } catch {
    // Keep the last known menu state if the local store is temporarily busy.
  }
}

async function dismissMenu(): Promise<void> {
  try {
    await invoke("dismiss_context_menu");
  } catch {
    // The popup may already have been dismissed by losing focus.
  }
}

async function showPreferences(): Promise<void> {
  await dismissMenu();
  await openManager();
}

async function composeTodo(): Promise<void> {
  await dismissMenu();
  await emitTo("todo", "vela://todo-compose");
}

async function toggleLayer(): Promise<void> {
  const kind = widgetKind.value;
  try {
    const current = await getSnapshot();
    await setWidgetLayer(kind, !current.settings.widgets[kind].alwaysOnTop);
    alwaysOnTop.value = !current.settings.widgets[kind].alwaysOnTop;
    await dismissMenu();
  } catch {
    layerFailed.value = true;
    if (layerFailedTimer) clearTimeout(layerFailedTimer);
    layerFailedTimer = setTimeout(() => (layerFailed.value = false), 1800);
  }
}

async function chooseSize(size: WidgetSize): Promise<void> {
  try {
    widgetSize.value = size;
    await setWidgetSize(widgetKind.value, size);
    await dismissMenu();
  } catch {
    sizeFailed.value = true;
    void refreshLayerState();
    setTimeout(() => (sizeFailed.value = false), 1800);
  }
}

async function closeWidget(): Promise<void> {
  try {
    await setWidgetEnabled(widgetKind.value, false);
    await dismissMenu();
    closeBlocked.value = false;
  } catch {
    closeBlocked.value = true;
    if (closeBlockedTimer) clearTimeout(closeBlockedTimer);
    closeBlockedTimer = setTimeout(() => (closeBlocked.value = false), 1800);
  }
}

function onKeyDown(event: KeyboardEvent): void {
  if (event.key === "Escape") {
    event.preventDefault();
    void dismissMenu();
  }
}

onMounted(async () => {
  unlistenKind = await listen<unknown>("vela://context-menu-kind", (event) => {
    if (isWidgetKind(event.payload)) {
      widgetKind.value = event.payload;
      void refreshLayerState();
    }
    closeBlocked.value = false;
    layerFailed.value = false;
    sizeFailed.value = false;
  });
  try {
    const currentKind: unknown = await invoke("get_context_menu_widget");
    if (isWidgetKind(currentKind)) {
      widgetKind.value = currentKind;
      await refreshSnapshot();
      await nextTick();
      await refreshLayerState();
      await invoke("context_menu_ready");
    }
  } catch {
    // The popup is hidden if there is no active widget menu.
  }
  window.addEventListener("keydown", onKeyDown);
});

onUnmounted(() => {
  unlistenKind?.();
  window.removeEventListener("keydown", onKeyDown);
  if (closeBlockedTimer) clearTimeout(closeBlockedTimer);
  if (layerFailedTimer) clearTimeout(layerFailedTimer);
});
</script>

<template>
  <main class="context-menu-window">
    <div class="widget-context-menu native-context-menu" role="menu" aria-label="组件菜单">
      <template v-if="widgetKind === 'todo'">
        <button class="context-primary" role="menuitem" @click="composeTodo">
          <AppIcon name="plus" :size="14" />新建待办
        </button>
        <div class="context-divider"></div>
      </template>
      <button :class="{ 'context-primary': widgetKind !== 'todo' }" role="menuitem" @click="showPreferences">
        <AppIcon name="sliders" :size="14" />Vela 偏好设置
      </button>
      <button role="menuitem" :disabled="layerFailed" @click="toggleLayer">
        <AppIcon name="arrow-up-right" :size="14" />{{ layerFailed ? "层级设置未保存" : alwaysOnTop ? "取消置顶" : "始终置顶" }}
      </button>
      <div class="context-divider"></div>
      <WidgetSizeMenuRow :size="widgetSize" :disabled="sizeFailed" @choose="chooseSize" />
      <div class="context-divider"></div>
      <button class="context-danger" role="menuitem" :disabled="closeBlocked" @click="closeWidget">
        <AppIcon name="close" :size="14" />{{ closeBlocked ? "至少保留一个组件" : `关闭${widgetTitle()}组件` }}
      </button>
    </div>
  </main>
</template>
