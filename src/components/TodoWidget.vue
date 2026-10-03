<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import WidgetPageControls from "./WidgetPageControls.vue";
import { paginateTodos } from "../lib/todoPages";
import AppIcon from "./AppIcon.vue";
import WidgetSizeMenuRow from "./WidgetSizeMenuRow.vue";
import {
  DialogClose,
  DialogContent,
  DialogOverlay,
  DialogPortal,
  DialogRoot,
  DialogTitle,
} from "reka-ui";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isNativeApp, openManager, showWidgetContextMenu } from "../lib/backend";
import {
  createTodo,
  deleteTodo,
  setTodoCompleted,
  setWidgetEnabled,
  setWidgetLayer,
  setWidgetSize,
  snapshot,
  updateTodo,
} from "../lib/store";
import { useWindowBounds } from "../lib/useWindowBounds";
import type { TodoItem, WidgetSize } from "../types";

const filter = ref<"open" | "done">("open");
const draft = ref("");
const dueDate = ref("");
const composerOpen = ref(false);
const composerInput = ref<HTMLInputElement | null>(null);
const editingId = ref<number | null>(null);
const savingEditId = ref<number | null>(null);
const editTitle = ref("");
const editDueDate = ref("");
const menu = ref<{ x: number; y: number } | null>(null);
const notice = ref("");
let noticeTimer: ReturnType<typeof setTimeout> | undefined;

const openTodos = computed(() => snapshot.value.todos.filter((todo) => !todo.completed));
const completedTodos = computed(() => snapshot.value.todos.filter((todo) => todo.completed));
const visibleTodos = computed(() => (filter.value === "open" ? openTodos.value : completedTodos.value));
const activeCount = computed(() => openTodos.value.length);
const widget = computed(() => snapshot.value.settings.widgets.todo);
const size = computed(() => widget.value.size);
const dragRegion = computed(() => (widget.value.locked ? undefined : "deep"));
const page = ref(0);
const pageDirection = ref(1);
const pageSource = computed(() => size.value === "large" ? visibleTodos.value : openTodos.value);
const pages = computed(() => paginateTodos(pageSource.value, size.value, editingId.value));
const pageItems = computed(() => pages.value[Math.min(page.value, pages.value.length - 1)] ?? []);
watch([size, filter], () => { page.value = 0; editingId.value = null; });
watch(pages, (next) => {
  if (editingId.value !== null) { const index = next.findIndex(items => items.some(item => item.id === editingId.value)); if (index >= 0) page.value = index; }
  page.value = Math.min(page.value, next.length - 1);
});
function movePage(direction: number) { pageDirection.value = direction; page.value = Math.max(0, Math.min(pages.value.length - 1, page.value + direction)); }
// Browser previews use the same fixed footprint as native windows.
const previewSize = computed(() => isNativeApp() ? undefined : { width: `${size.value === "small" ? 170 : 364}px`, height: `${size.value === "large" ? 384 : 170}px` });
const countCaption = computed(() => (activeCount.value === 0 ? "全部完成" : "项未完成"));

function formatDueDate(value: string | null): string {
  if (!value) return "";
  const date = new Date(`${value}T12:00:00`);
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const tomorrow = new Date(today);
  tomorrow.setDate(tomorrow.getDate() + 1);
  if (date.toDateString() === today.toDateString()) return "今天";
  if (date.toDateString() === tomorrow.toDateString()) return "明天";
  return new Intl.DateTimeFormat("zh-CN", { month: "numeric", day: "numeric" }).format(date);
}

function isOverdue(todo: TodoItem): boolean {
  if (!todo.dueDate || todo.completed) return false;
  const today = new Date();
  const todayKey = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
  return todo.dueDate < todayKey;
}

async function addTodo(): Promise<void> {
  if (!draft.value.trim()) return;
  try {
    await createTodo(draft.value, dueDate.value || null);
    page.value = 0;
    draft.value = "";
    dueDate.value = "";
    composerOpen.value = false;
  } catch (error) {
    showNotice(typeof error === "string" ? error : "待办没有保存，请重试。");
  }
}

async function openComposer(): Promise<void> {
  menu.value = null;
  composerOpen.value = true;
  await nextTick();
  composerInput.value?.focus();
}

function closeComposer(): void {
  composerOpen.value = false;
  draft.value = "";
  dueDate.value = "";
}

function onComposerOpenChange(open: boolean): void {
  if (open) composerOpen.value = true;
  else closeComposer();
}

async function toggle(todo: TodoItem): Promise<void> {
  try {
    await setTodoCompleted(todo.id, !todo.completed);
  } catch {
    showNotice("没有保存，请重试。");
  }
}

function beginEdit(todo: TodoItem): void {
  editingId.value = todo.id;
  editTitle.value = todo.title;
  editDueDate.value = todo.dueDate ?? "";
}

async function commitEdit(todo: TodoItem): Promise<void> {
  if (savingEditId.value === todo.id) return;
  if (!editTitle.value.trim()) {
    showNotice("待办内容不能为空。");
    return;
  }
  savingEditId.value = todo.id;
  try {
    await updateTodo(todo.id, editTitle.value, editDueDate.value || null);
    editingId.value = null;
  } catch (error) {
    showNotice(typeof error === "string" ? error : "待办没有保存，请重试。");
  } finally {
    savingEditId.value = null;
  }
}

async function remove(todo: TodoItem): Promise<void> {
  try {
    await deleteTodo(todo.id);
  } catch {
    showNotice("没有删除，请重试。");
  }
}

async function openContextMenu(event: MouseEvent): Promise<void> {
  const target = event.target;
  if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || (target instanceof HTMLElement && target.isContentEditable)) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  if (isNativeApp()) {
    try {
      // The native popup clamps itself to the monitor, not to this small window.
      await showWidgetContextMenu("todo", event.clientX, event.clientY);
      return;
    } catch {
      // Fall through to the in-window menu.
    }
  }
  menu.value = {
    x: Math.max(4, Math.min(event.clientX, window.innerWidth - 192)),
    y: Math.max(4, Math.min(event.clientY, window.innerHeight - 190)),
  };
}

function dismissMenu(): void {
  menu.value = null;
}

function showNotice(message: string): void {
  notice.value = message;
  if (noticeTimer) clearTimeout(noticeTimer);
  noticeTimer = setTimeout(() => (notice.value = ""), 2400);
}

async function toggleLayer(): Promise<void> {
  try {
    await setWidgetLayer("todo", !widget.value.alwaysOnTop);
  } catch {
    showNotice("层级设置暂时没有保存。");
  }
  menu.value = null;
}

async function chooseSize(next: WidgetSize): Promise<void> {
  menu.value = null;
  try {
    await setWidgetSize("todo", next);
  } catch {
    showNotice("尺寸暂时没有保存。");
  }
}

async function closeWidget(): Promise<void> {
  try {
    await setWidgetEnabled("todo", false);
  } catch (error) {
    showNotice(typeof error === "string" ? error : "没有关闭组件。");
  }
  menu.value = null;
}

let unlistenCompose: (() => void) | undefined;

useWindowBounds("todo");
onMounted(async () => {
  window.addEventListener("pointerdown", dismissMenu);
  if (isNativeApp()) {
    // "新建待办" in the native context-menu window asks this widget to open its composer.
    unlistenCompose = await listen("vela://todo-compose", async () => {
      await getCurrentWindow().setFocus().catch(() => undefined);
      await openComposer();
    });
  }
});
onUnmounted(() => {
  window.removeEventListener("pointerdown", dismissMenu);
  unlistenCompose?.();
  if (noticeTimer) clearTimeout(noticeTimer);
});
</script>

<template>
  <main
    class="widget-window todo-widget"
    :class="[`size-${size}`, { 'widget-draggable': !widget.locked }]"
    :style="previewSize"
    @contextmenu="openContextMenu"
  >
    <!-- Small: fixed heading and vertically paged unfinished tasks -->
    <template v-if="size === 'small'">
      <div class="todo-glance-head" :data-tauri-drag-region="dragRegion">
        <span class="todo-glyph"><AppIcon name="tick" :size="14" /></span>
        <strong class="todo-big-count">{{ activeCount }}</strong>
      </div>
      <span class="todo-glance-title" :data-tauri-drag-region="dragRegion">待办</span>
      <ul v-if="openTodos.length" class="todo-glance-list todo-page-list" :key="`${size}-${page}`" :data-direction="pageDirection > 0 ? 'down' : 'up'" aria-label="待办列表">
        <li v-for="todo in pageItems" :key="todo.id">
          <button class="todo-check" aria-label="标记完成" :aria-pressed="false" @click="toggle(todo)"></button>
          <span class="todo-glance-text" :data-tooltip="todo.title">{{ todo.title }}</span>
        </li>
      </ul>
      <p v-else class="todo-glance-empty">没有待办事项</p>
      <WidgetPageControls :page="page" :count="pages.length" @move="movePage" />
    </template>

    <!-- Medium: fixed summary beside vertically paged unfinished tasks -->
    <template v-else-if="size === 'medium'">
      <div class="todo-medium">
        <div class="todo-summary" :data-tauri-drag-region="dragRegion">
          <span class="todo-glyph"><AppIcon name="tick" :size="14" /></span>
          <div class="todo-summary-count">
            <strong class="todo-big-count">{{ activeCount }}</strong>
            <span>{{ countCaption }}</span>
          </div>
          <button class="widget-chip todo-add-chip" aria-label="新建待办" @click="openComposer"><AppIcon name="plus" :size="13" />新建</button>
        </div>
        <div class="todo-glance-panel">
          <ul v-if="openTodos.length" class="todo-glance-list todo-page-list" :key="`${size}-${page}`" :data-direction="pageDirection > 0 ? 'down' : 'up'" aria-label="待办列表">
            <li v-for="todo in pageItems" :key="todo.id">
              <button class="todo-check" aria-label="标记完成" :aria-pressed="false" @click="toggle(todo)"></button>
              <span class="todo-glance-text" :data-tooltip="todo.title">{{ todo.title }}</span>
              <span v-if="todo.dueDate" class="todo-due-chip" :class="{ overdue: isOverdue(todo) }">{{ formatDueDate(todo.dueDate) }}</span>
            </li>
          </ul>
          <WidgetPageControls v-if="openTodos.length" :page="page" :count="pages.length" @move="movePage" />
          <div v-if="!openTodos.length" class="todo-glance-empty">
            <span class="todo-empty-mark"><AppIcon name="tick" :size="16" /></span>
            <span>没有待办事项</span>
          </div>
        </div>
      </div>
    </template>

    <!-- Large: the full list with editing -->
    <template v-else>
      <header class="todo-large-header" :class="{ 'widget-header-draggable': !widget.locked }" :data-tauri-drag-region="dragRegion">
        <span class="todo-glyph"><AppIcon name="tick" :size="14" /></span>
        <div class="todo-large-heading">
          <h1>待办</h1>
          <span>{{ activeCount ? `${activeCount} 项未完成` : "全部完成" }}</span>
        </div>
        <button class="widget-icon-button todo-add-button" aria-label="新建待办" data-tooltip="新建待办" @click="openComposer"><AppIcon name="plus" :size="16" /></button>
      </header>

      <div class="todo-segmented" role="tablist">
        <button :class="{ active: filter === 'open' }" role="tab" :aria-selected="filter === 'open'" @click="filter = 'open'">未完成<span>{{ openTodos.length }}</span></button>
        <button :class="{ active: filter === 'done' }" role="tab" :aria-selected="filter === 'done'" @click="filter = 'done'">已完成<span>{{ completedTodos.length }}</span></button>
      </div>

      <section class="todo-list todo-page-list" :key="`large-${page}`" :data-direction="pageDirection > 0 ? 'down' : 'up'" aria-label="待办列表">
        <div v-if="visibleTodos.length === 0" class="todo-empty">
          <span class="todo-empty-mark"><AppIcon name="tick" :size="18" /></span>
          <span>{{ filter === 'open' ? '没有待办事项' : '暂无已完成事项' }}</span>
          <button v-if="filter === 'open'" class="widget-chip" @click="openComposer"><AppIcon name="plus" :size="13" />新建待办</button>
        </div>

        <article v-for="todo in pageItems" :key="todo.id" class="todo-item" :class="{ completed: todo.completed }">
          <button class="todo-check" :aria-label="todo.completed ? '取消完成' : '标记完成'" :aria-pressed="todo.completed" @click="toggle(todo)"><AppIcon v-if="todo.completed" name="tick" :size="11" /></button>
          <div class="todo-item-content">
            <template v-if="editingId === todo.id">
              <form class="todo-edit-form" @submit.prevent="commitEdit(todo)" @pointerdown.stop>
                <input v-model="editTitle" class="todo-edit-title" aria-label="编辑待办内容" maxlength="160" @keydown.esc="editingId = null" />
                <div class="todo-edit-row">
                  <input v-model="editDueDate" class="todo-edit-date" type="date" aria-label="截止日期" />
                  <button class="todo-save-edit" type="submit" :disabled="savingEditId === todo.id" @click.stop>{{ savingEditId === todo.id ? '保存中…' : '保存' }}</button>
                </div>
              </form>
            </template>
            <template v-else>
              <button class="todo-title-button" :data-tooltip="todo.title" @dblclick="beginEdit(todo)">{{ todo.title }}</button>
              <span v-if="todo.dueDate" class="todo-due" :class="{ overdue: isOverdue(todo) }"><AppIcon name="clock" :size="11" />{{ isOverdue(todo) ? '已逾期 · ' : '' }}{{ formatDueDate(todo.dueDate) }}</span>
            </template>
          </div>
          <button class="todo-more" :aria-label="`编辑 ${todo.title}`" data-tooltip="编辑" @click="beginEdit(todo)"><AppIcon name="more" :size="15" /></button>
          <button class="todo-delete" :aria-label="`删除 ${todo.title}`" data-tooltip="删除" @click="remove(todo)"><AppIcon name="close" :size="13" /></button>
        </article>
      </section>
      <WidgetPageControls :page="page" :count="pages.length" :editing="editingId !== null" @move="movePage" />
    </template>

    <DialogRoot :open="composerOpen" @update:open="onComposerOpenChange">
      <DialogPortal>
        <DialogOverlay class="todo-compose-overlay" />
        <DialogContent class="todo-compose-panel" :class="[`size-${size}`, { compact: size !== 'large' }]">
          <header class="todo-compose-header">
            <DialogTitle class="todo-compose-title">新建待办</DialogTitle>
            <DialogClose as-child>
              <button class="todo-compose-close" type="button" aria-label="关闭新建待办" @click="closeComposer"><AppIcon name="close" :size="15" /></button>
            </DialogClose>
          </header>
          <form class="todo-compose-form" @submit.prevent="addTodo">
            <label class="todo-compose-label" for="todo-create-title">待办内容</label>
            <input id="todo-create-title" ref="composerInput" v-model="draft" class="todo-create-input" placeholder="添加一项待办" maxlength="160" />
            <label v-if="size !== 'small'" class="todo-compose-label" for="todo-create-date">截止日期</label>
            <footer class="todo-compose-actions">
              <input v-if="size !== 'small'" id="todo-create-date" v-model="dueDate" class="todo-create-date" type="date" />
              <button class="todo-compose-cancel" type="button" @click="closeComposer">取消</button>
              <button class="todo-compose-submit" type="submit" :disabled="!draft.trim()">添加</button>
            </footer>
          </form>
        </DialogContent>
      </DialogPortal>
    </DialogRoot>

    <transition name="notice"><div v-if="notice" class="widget-notice">{{ notice }}</div></transition>

    <div v-if="menu" class="widget-context-menu" :style="{ left: `${menu.x}px`, top: `${menu.y}px` }" @pointerdown.stop>
      <button class="context-primary" @click="openComposer"><AppIcon name="plus" :size="16" />新建待办</button>
      <div class="context-divider"></div>
      <button @click="openManager(); menu = null"><AppIcon name="sliders" :size="16" />Vela 偏好设置</button>
      <button @click="toggleLayer"><AppIcon name="arrow-up-right" :size="16" />{{ widget.alwaysOnTop ? '取消置顶' : '始终置顶' }}</button>
      <div class="context-divider"></div>
      <WidgetSizeMenuRow :size="size" @choose="chooseSize" />
      <div class="context-divider"></div>
      <button class="context-danger" @click="closeWidget"><AppIcon name="close" :size="16" />关闭待办组件</button>
    </div>
  </main>
</template>
