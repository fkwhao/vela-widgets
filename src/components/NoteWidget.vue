<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, emitTo } from "@tauri-apps/api/event";
import WidgetFrame from "./WidgetFrame.vue";
import WidgetPageControls from "./WidgetPageControls.vue";
import NoteMarkdown from "./NoteMarkdown.vue";
import AppIcon from "./AppIcon.vue";
import { isNativeApp } from "../lib/backend";
import { snapshot, ready, saveNote, createNote, selectNote, deleteNote, restoreNote, refreshSnapshot } from "../lib/store";
import { noteTitle } from "../lib/markdown";
import { noteRetentionHours } from "../lib/notes";
import type { NoteItem } from "../types";
const notes = computed(() => snapshot.value.settings.note.notes);
const active = computed(() => notes.value.find(n => n.id === snapshot.value.settings.note.activeId));
const page = computed(() => Math.max(0, notes.value.findIndex(n => n.id === active.value?.id)));
const color = computed(() => active.value?.color ?? snapshot.value.settings.note.color);
const draft = ref(""); const saved = ref(""); const draftId = ref(0);
const editing = ref(false); const busy = ref(false); const composing = ref(false);
const status = ref(""); const listOpen = ref(false); const direction = ref(1);
const input = ref<HTMLTextAreaElement | null>(null);
const listPanel = ref<HTMLElement | null>(null);
const deleted = ref<NoteItem | null>(null);
let timer: ReturnType<typeof setTimeout> | undefined;
let undoTimer: ReturnType<typeof setTimeout> | undefined;
let expiryTimer: ReturnType<typeof setTimeout> | undefined;
let pending: Promise<void> | undefined;
const unlisteners: (() => void)[] = [];
let disposed = false;
function scheduleExpiry() {
  if (expiryTimer) clearTimeout(expiryTimer);
  const deadlines = notes.value.flatMap(n => { const hours = noteRetentionHours(n, snapshot.value.settings.note.defaultDeleteAfterHours); return hours === null ? [] : [n.createdAt + hours * 3600000]; });
  if (!deadlines.length || disposed) return;
  const delay = Math.max(1000, Math.min(2147483647, Math.min(...deadlines) - Date.now()));
  expiryTimer = setTimeout(() => { void refreshSnapshot().finally(scheduleExpiry); }, delay);
}
watch([notes, () => snapshot.value.settings.note.defaultDeleteAfterHours], scheduleExpiry, { immediate: true });
watch(listOpen, async open => { if (open) { await nextTick(); listPanel.value?.querySelector<HTMLButtonElement>('button')?.focus(); } });
function listKeys(event: KeyboardEvent) {
  if (event.key !== 'Tab') return;
  const buttons = Array.from(listPanel.value?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') ?? []);
  const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
  if (!buttons.length) return;
  if (event.shiftKey && index <= 0) { event.preventDefault(); buttons.at(-1)?.focus(); }
  else if (!event.shiftKey && index === buttons.length - 1) { event.preventDefault(); buttons[0]?.focus(); }
}
watch(() => [ready.value, active.value?.id, active.value?.text] as const, ([loaded, id, text]) => {
  if (!loaded) return;
  if ((id ?? 0) !== draftId.value) {
    if (timer) clearTimeout(timer); timer = undefined;
    draftId.value = id ?? 0; draft.value = text ?? ""; saved.value = draft.value; editing.value = false; status.value = "";
  } else if (draft.value === saved.value) { draft.value = text ?? ""; saved.value = draft.value; }
}, { immediate: true });
async function flush(): Promise<void> {
  if (timer) { clearTimeout(timer); timer = undefined; }
  if (pending) { await pending; if (draft.value !== saved.value) await flush(); return; }
  if (draft.value === saved.value || !draftId.value) return;
  pending = (async () => {
    while (draft.value !== saved.value) {
      const text = draft.value, id = draftId.value; status.value = "正在保存";
      try { await saveNote(id, text); if (draftId.value === id) { saved.value = text; status.value = "已保存"; } }
      catch (error) { status.value = "未保存，点击重试"; throw error; }
    }
  })();
  try { await pending; } finally { pending = undefined; }
}
function queueSave() {
  status.value = "正在编辑"; if (timer) clearTimeout(timer);
  if (!composing.value) timer = setTimeout(() => { timer = undefined; void flush().catch(() => undefined); }, 450);
}
function startComposition() { composing.value = true; if (timer) clearTimeout(timer); }
async function run(action: () => Promise<void>) {
  if (busy.value) return; busy.value = true;
  try { await action(); } catch (error) { status.value = typeof error === "string" ? error : error instanceof Error ? error.message : "操作未完成，请重试。"; }
  finally { busy.value = false; }
}
async function edit() {
  if (!active.value) { await newNote(); return; }
  editing.value = true; listOpen.value = false; await nextTick(); input.value?.focus();
}
async function finish() { await run(async () => { await flush(); editing.value = false; }); }
async function newNote() {
  await run(async () => { await flush(); direction.value = 1; await createNote(); await nextTick(); editing.value = true; listOpen.value = false; await nextTick(); input.value?.focus(); });
}
async function choose(id: number) {
  if (editing.value || busy.value) return;
  const index = notes.value.findIndex(n => n.id === id); direction.value = index < page.value ? -1 : 1;
  await run(async () => { await flush(); await selectNote(id); listOpen.value = false; });
}
function move(step: number) { const item = notes.value[page.value + step]; if (item) void choose(item.id); }
async function remove() {
  if (!active.value) return;
  await run(async () => {
    await flush(); const item = { ...active.value! }; direction.value = 1;
    await deleteNote(item.id); deleted.value = item; editing.value = false; listOpen.value = false;
    if (undoTimer) clearTimeout(undoTimer); undoTimer = setTimeout(() => deleted.value = null, 10000);
  });
}
async function undo() { if (deleted.value) await run(async () => { await flush(); await restoreNote(deleted.value!); deleted.value = null; if (undoTimer) clearTimeout(undoTimer); }); }
function insert(before: string, after = "", placeholder = "文字") {
  const node = input.value; if (!node) return;
  const start = node.selectionStart, end = node.selectionEnd, selection = draft.value.slice(start, end) || placeholder;
  draft.value = draft.value.slice(0, start) + before + selection + after + draft.value.slice(end); queueSave();
  void nextTick(() => { node.focus(); node.setSelectionRange(start + before.length, start + before.length + selection.length); });
}
function contextAction(action: string) {
  if (action === "new") void newNote();
  else if (action === "delete") void remove();
  else if (action === "list") { if (editing.value) status.value = "完成编辑后可切换便签"; else listOpen.value = true; }
}
async function addListener<T>(name: string, handler: Parameters<typeof listen<T>>[1]) {
  const stop = await listen<T>(name, handler); if (disposed) stop(); else unlisteners.push(stop);
}
onMounted(async () => {
  scheduleExpiry();
  if (!isNativeApp()) return;
  await addListener<{ requestId: string }>("vela://note-flush", async ({ payload }) => {
    let ok = true; try { await flush(); } catch { ok = false; }
    await Promise.allSettled(["manager", "context-menu", "note"].map(label => emitTo(label, "vela://note-flushed", { requestId: payload.requestId, ok })));
  });
  await addListener("vela://note-close-requested", async () => { try { await flush(); await invoke("set_widget_enabled", { kind: "note", enabled: false }); } catch { status.value = "未保存，点击重试"; } });
  await addListener<string>("vela://note-action", ({ payload }) => contextAction(payload));
});
onUnmounted(() => { disposed = true; if (timer) clearTimeout(timer); if (undoTimer) clearTimeout(undoTimer); if (expiryTimer) clearTimeout(expiryTimer); unlisteners.forEach(stop => stop()); });
</script>
<template>
  <WidgetFrame kind="note" header-only :before-close="flush"><template #default="{ widget, drag }">
    <header class="extra-header note-header" :inert="listOpen">
      <h1 class="note-title" :style="{ '--note-color': color }" :data-tooltip="noteTitle(draft)" :data-tauri-drag-region="drag"><AppIcon name="note" :size="15" /><span>{{ active ? noteTitle(draft) : '便签' }}</span></h1>
      <div class="note-actions"><button class="widget-icon-button" aria-label="新建便签" data-tooltip="新建便签" :disabled="busy" @click="newNote"><AppIcon name="plus" :size="14" /></button><button class="widget-icon-button" aria-label="删除当前便签" data-tooltip="删除当前便签" :disabled="busy || !active" @click="remove"><AppIcon name="trash" :size="14" /></button><button class="widget-icon-button" :aria-label="editing ? '完成编辑' : '编辑便签'" :data-tooltip="editing ? '完成编辑' : '编辑便签'" :disabled="busy" @click="editing ? finish() : edit()"><AppIcon :name="editing ? 'tick' : 'edit'" :size="14" /></button></div>
    </header>
    <div v-if="editing && widget.size === 'large'" class="note-toolbar"><button class="widget-chip" aria-label="插入标题" @click="insert('# ')">H</button><button class="widget-chip" aria-label="插入加粗文字" @click="insert('**', '**')"><b>B</b></button><button class="widget-chip" aria-label="插入列表" @click="insert('- ')">列表</button><button class="widget-chip" aria-label="插入任务" @click="insert('- [ ] ')">待办</button><button class="widget-chip" aria-label="插入代码" @click="insert('```\n', '\n```', '代码')">代码</button><span class="extra-muted">Markdown</span></div>
    <div class="note-stack" :class="{ 'stack-up': direction < 0 }">
      <Transition name="note-card" @before-leave="el => { el.setAttribute('aria-hidden', 'true'); el.setAttribute('inert', ''); }"><div :key="draftId" class="note-sheet">
        <textarea v-if="editing" ref="input" v-model="draft" class="note-editor" maxlength="20000" aria-label="便签 Markdown 内容" placeholder="第一行作为标题&#10;&#10;支持 Markdown、表格、公式与 Mermaid 图表…" @input="queueSave" @keydown.ctrl.enter.prevent="finish" @keydown.ctrl.b.prevent="insert('**', '**')" @keydown.tab.prevent="insert('  ', '', '')" @keydown.esc="finish" @compositionstart="startComposition" @compositionend="composing = false; queueSave()"></textarea>
        <div v-else class="note-content" :class="{ empty: !draft }" role="region" aria-label="便签内容" tabindex="0"><NoteMarkdown v-if="draft" :text="draft" :note-id="draftId" /><span v-else>{{ active ? '记下一点什么。' : '还没有便签。' }}<small>{{ active ? '点击右上角编辑按钮' : '点击 + 新建便签' }}</small></span></div>
      </div></Transition>
    </div>
    <footer class="note-footer" :inert="listOpen"><button v-if="deleted" class="note-save-status" :disabled="busy" aria-label="撤销删除便签" @click="undo">已删除 · 撤销</button><button v-else class="note-save-status" :data-tooltip="status || '自动保存'" aria-live="polite" @click="flush().catch(() => undefined)">{{ status || (editing ? '自动保存' : `${draft.length} 字`) }}</button><WidgetPageControls :page="page" :count="notes.length" label="便签" :editing="editing || busy" @move="move" /></footer>
    <section v-if="listOpen" ref="listPanel" class="note-list-panel" role="dialog" aria-modal="true" aria-label="便签列表" @keydown="listKeys" @keydown.esc.stop="listOpen = false"><header><strong>便签列表 <small>{{ notes.length }}</small></strong><button class="widget-icon-button" aria-label="关闭便签列表" @click="listOpen = false"><AppIcon name="close" :size="14" /></button></header><div class="note-list-items"><button v-for="note in notes" :key="note.id" :aria-pressed="note.id === active?.id" :disabled="busy" @click="choose(note.id)"><i :style="{ background: note.color }"></i><span>{{ noteTitle(note.text) }}</span><AppIcon v-if="note.id === active?.id" name="tick" :size="12" /></button><p v-if="!notes.length" class="extra-muted">点击 + 创建第一篇便签</p></div></section>
    </template><template #context-actions="{ dismiss }"><button class="context-primary" role="menuitem" :disabled="busy" @click="dismiss(); newNote()"><AppIcon name="plus" :size="14" />新建便签</button><button role="menuitem" :disabled="editing || busy" @click="dismiss(); listOpen = true"><AppIcon name="note" :size="14" />便签列表</button><button class="context-danger" role="menuitem" :disabled="!active || busy" @click="dismiss(); remove()"><AppIcon name="trash" :size="14" />删除当前便签</button><div class="context-divider"></div></template>
  </WidgetFrame>
</template>
