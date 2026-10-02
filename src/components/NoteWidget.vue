<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, emitTo } from "@tauri-apps/api/event";
import WidgetFrame from "./WidgetFrame.vue";
import AppIcon from "./AppIcon.vue";
import { isNativeApp } from "../lib/backend";
import { snapshot, ready, saveNote } from "../lib/store";
const draft = ref("");
const saved = ref("");
const editing = ref(false);
const status = ref("");
const input = ref<HTMLTextAreaElement | null>(null);
let timer: ReturnType<typeof setTimeout> | undefined;
let pending: Promise<void> | undefined;
let unlistenClose: (() => void) | undefined;
let unlistenRequest: (() => void) | undefined;
let disposed = false;
const color = computed(() => snapshot.value.settings.note.color);
watch(() => [ready.value, snapshot.value.settings.note.text] as const, ([loaded, text]) => {
  if (loaded && draft.value === saved.value) { draft.value = text; saved.value = text; }
}, { immediate: true });
async function flush(): Promise<void> {
  if (timer) { clearTimeout(timer); timer = undefined; }
  if (pending) { await pending; if (draft.value !== saved.value) await flush(); return; }
  if (draft.value === saved.value) return;
  pending = (async () => {
    while (draft.value !== saved.value) {
      const text = draft.value; status.value = "正在保存";
      try { await saveNote(text); saved.value = text; status.value = "已保存"; }
      catch (e) { status.value = "未保存，点击重试"; throw e; }
    }
  })();
  try { await pending; } finally { pending = undefined; }
}
function queueSave() {
  status.value = "正在编辑";
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => { timer = undefined; void flush().catch(() => undefined); }, 450);
}
async function edit() { editing.value = true; await nextTick(); input.value?.focus(); }
function finish() { void flush().then(() => editing.value = false).catch(() => undefined); }
function bold() {
  const node = input.value; if (!node) return;
  const start = node.selectionStart, end = node.selectionEnd;
  draft.value = draft.value.slice(0, start) + "**" + (draft.value.slice(start,end) || "文字") + "**" + draft.value.slice(end);
  queueSave(); void nextTick(() => { node.focus(); node.setSelectionRange(start+2, end > start ? end+2 : start+4); });
}
function list() {
  const node = input.value; if (!node) return;
  const start = draft.value.lastIndexOf("\n", node.selectionStart - 1) + 1;
  draft.value = draft.value.slice(0,start) + "- " + draft.value.slice(start); queueSave(); void nextTick(() => node.focus());
}
const lines = computed(() => draft.value.split("\n").map((line) => ({ bullet: /^[-*] /.test(line), segments: line.replace(/^[-*] /, "").split(/(\*\*.*?\*\*)/g).map((text) => ({ bold: text.startsWith("**") && text.endsWith("**"), text: text.startsWith("**") && text.endsWith("**") ? text.slice(2,-2) : text })) })));
onMounted(async () => {
  if (isNativeApp()) {
  unlistenClose = await listen<{ requestId: string }>("vela://note-flush", async ({ payload }) => {
    try { await flush(); await emitTo("manager", "vela://note-flushed", { requestId: payload.requestId, ok: true }); await emitTo("context-menu", "vela://note-flushed", { requestId: payload.requestId, ok: true }); }
    catch { await emitTo("manager", "vela://note-flushed", { requestId: payload.requestId, ok: false }); await emitTo("context-menu", "vela://note-flushed", { requestId: payload.requestId, ok: false }); }
  });
  if (disposed) { unlistenClose(); return; }
  unlistenRequest = await listen("vela://note-close-requested", async () => {
    try { await flush(); await invoke("set_widget_enabled", { kind: "note", enabled: false }); }
    catch (e) { status.value = typeof e === "string" ? e : "未保存，点击重试"; }
  });
  if (disposed) unlistenRequest();
  }
});
onUnmounted(() => { disposed = true; if (timer) clearTimeout(timer); unlistenClose?.(); unlistenRequest?.(); });
</script>
<template>
  <WidgetFrame kind="note" header-only :before-close="flush" v-slot="{ widget, drag }">
    <header class="extra-header" :data-tauri-drag-region="drag"><h1 class="note-title" :style="{ '--note-color': color }"><AppIcon name="note" :size="16" />便签</h1><button class="widget-icon-button" :aria-label="editing ? '完成编辑' : '编辑便签'" @click="editing ? finish() : edit()"><AppIcon :name="editing ? 'tick' : 'edit'" :size="15" /></button></header>
    <div v-if="editing && widget.size === 'large'" class="note-toolbar"><button class="widget-chip" aria-label="插入加粗文字" @click="bold"><b>B</b></button><button class="widget-chip" aria-label="插入列表" @click="list">列表</button><span class="extra-muted">支持 **加粗** 与 - 列表</span></div>
    <textarea v-if="editing" ref="input" v-model="draft" class="note-editor" maxlength="20000" aria-label="便签内容" placeholder="写下此刻的想法…" @input="queueSave" @blur="flush().catch(() => undefined)" @keydown.ctrl.enter.prevent="finish" @keydown.esc="finish" @compositionend="queueSave"></textarea>
    <div v-else class="note-content" :class="{ empty: !draft }" role="region" aria-label="便签内容" tabindex="0"><template v-if="draft"><span v-for="(line,index) in lines" :key="index" class="note-line" :class="{ bullet: line.bullet }"><template v-for="(segment,i) in line.segments" :key="i"><strong v-if="segment.bold">{{ segment.text }}</strong><template v-else>{{ segment.text }}</template></template><br v-if="!line.segments.some(s => s.text)" /></span></template><span v-else>记下一点什么。<small>点击右上角编辑按钮</small></span></div>
    <footer v-if="widget.size !== 'small' || status.startsWith('未保存')" class="note-footer"><span class="extra-muted">{{ draft.length }} 字</span><button class="note-save-status" aria-live="polite" @click="flush().catch(() => undefined)">{{ status || '自动保存' }}</button></footer>
  </WidgetFrame>
</template>
