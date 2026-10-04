<script setup lang="ts">
import { computed, onUnmounted, ref } from "vue";
import AppIcon from "../ui/AppIcon.vue";
import WidgetSizeMenuRow from "./WidgetSizeMenuRow.vue";
import { isNativeApp, openManager, showWidgetContextMenu } from "../../infrastructure/backend";
import { snapshot, setWidgetEnabled, setWidgetLayer, setWidgetSize } from "../../app/store";
import { useWindowBounds } from "../composables/useWindowBounds";
import { widgetRegistry, type WidgetKind, type WidgetSize } from "../types";
const props = defineProps<{ kind: WidgetKind; headerOnly?: boolean; beforeClose?: () => Promise<void> }>();
const widget = computed(() => snapshot.value.settings.widgets[props.kind]);
const previewSize = computed(() => isNativeApp() ? undefined : { width: `${widget.value.size === "small" ? 170 : 364}px`, height: `${widget.value.size === "large" ? 384 : 170}px` });
const menu = ref<{ x: number; y: number } | null>(null);
const error = ref("");
let timer: ReturnType<typeof setTimeout> | undefined;
useWindowBounds(props.kind);
async function run(action: () => Promise<unknown>) { try { await action(); menu.value = null; } catch (e) { error.value = typeof e === "string" ? e : "操作未完成，请重试。"; if (timer) clearTimeout(timer); timer = setTimeout(() => error.value = "", 4000); } }
function showMenu(event: MouseEvent) {
  if ((event.target as HTMLElement).closest("input, textarea, [contenteditable]")) return;
  event.preventDefault();
  if (isNativeApp()) void run(() => showWidgetContextMenu(props.kind, event.clientX, event.clientY));
  else menu.value = { x: Math.max(4, Math.min(event.clientX, Math.min(innerWidth, widget.value.size === "small" ? 170 : 364) - 192)), y: Math.max(4, Math.min(event.clientY, Math.min(innerHeight, widget.value.size === "large" ? 384 : 170) - 150)) };
}
async function close() { await props.beforeClose?.(); await setWidgetEnabled(props.kind, false); }
function resize(size: WidgetSize) { void run(() => setWidgetSize(props.kind, size)); }
onUnmounted(() => { if (timer) clearTimeout(timer); });
</script>
<template>
  <main class="widget-window" :style="previewSize" :class="[`size-${widget.size}`, `${kind}-window`]" :data-tauri-drag-region="!headerOnly && widget.size !== 'large' && !widget.locked ? 'deep' : undefined" @contextmenu="showMenu" @click="menu = null" @keydown.esc="menu = null">
    <slot :widget="widget" :drag="widget.locked ? undefined : 'deep'" />
    <p v-if="error" class="extra-notice" role="alert">{{ error }}</p>
    <div v-if="menu" class="widget-context-menu" role="menu" :style="{ position: 'fixed', left: `${menu.x}px`, top: `${menu.y}px`, zIndex: 20 }" @click.stop>
      <slot name="context-actions" :dismiss="() => menu = null" />
      <button class="context-primary" role="menuitem" @click="run(openManager)"><AppIcon name="sliders" :size="14" />Vela 偏好设置</button>
      <button role="menuitem" @click="run(() => setWidgetLayer(kind, !widget.alwaysOnTop))"><AppIcon name="arrow-up-right" :size="14" />{{ widget.alwaysOnTop ? '取消置顶' : '始终置顶' }}</button>
      <div class="context-divider"></div><WidgetSizeMenuRow :size="widget.size" @choose="resize" /><div class="context-divider"></div>
      <button class="context-danger" role="menuitem" @click="run(close)"><AppIcon name="close" :size="14" />关闭{{ widgetRegistry[kind].label }}组件</button>
    </div>
  </main>
</template>
