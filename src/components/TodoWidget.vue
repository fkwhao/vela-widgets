<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import AppIcon from "./AppIcon.vue";
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
import { createTodo, deleteTodo, setTodoCompleted, setWidgetEnabled, setWidgetLayer, snapshot, updateTodo } from "../lib/store";
import { useWindowBounds } from "../lib/useWindowBounds";
import type { TodoItem } from "../types";

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
    const x = Math.max(6, Math.min(event.clientX, window.innerWidth - 194));
    const y = Math.max(6, Math.min(event.clientY, window.innerHeight - 116));
    try {
      await showWidgetContextMenu("todo", x, y);
    } catch {
      menu.value = {
        x: Math.max(6, Math.min(event.clientX, window.innerWidth - 196)),
        y: Math.max(6, Math.min(event.clientY, window.innerHeight - 160)),
      };
    }
    return;
  }
  menu.value = {
    x: Math.max(6, Math.min(event.clientX, window.innerWidth - 196)),
    y: Math.max(6, Math.min(event.clientY, window.innerHeight - 160)),
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
    await setWidgetLayer("todo", !snapshot.value.settings.widgets.todo.alwaysOnTop);
  } catch {
    showNotice("层级设置暂时没有保存。");
  }
  menu.value = null;
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
  <main class="widget-window todo-widget" @contextmenu="openContextMenu">
    <header class="widget-header todo-header" :class="{ 'widget-header-draggable': !snapshot.settings.widgets.todo.locked }" :data-tauri-drag-region="snapshot.settings.widgets.todo.locked ? undefined : 'deep'">
      <div class="widget-brandline">
        <span class="widget-brand-dot todo-brand-dot"></span>
        <span>待办</span>
      <span class="todo-count"><strong>{{ activeCount }}</strong> 项未完成</span>
      </div>
    </header>

    <div class="todo-tabs" role="tablist">
      <button :class="{ active: filter === 'open' }" role="tab" :aria-selected="filter === 'open'" @click="filter = 'open'">未完成 <span>{{ openTodos.length }}</span></button>
      <button :class="{ active: filter === 'done' }" role="tab" :aria-selected="filter === 'done'" @click="filter = 'done'">已完成 <span>{{ completedTodos.length }}</span></button>
    </div>

    <section class="todo-list" aria-label="待办列表">
      <div v-if="visibleTodos.length === 0" class="todo-empty">
        <span>{{ filter === 'open' ? '暂无待办' : '暂无已完成事项' }}</span>
      </div>

      <article v-for="todo in visibleTodos" :key="todo.id" class="todo-item" :class="{ completed: todo.completed }">
        <button class="todo-check" :aria-label="todo.completed ? '取消完成' : '标记完成'" :aria-pressed="todo.completed" @click="toggle(todo)"><AppIcon v-if="todo.completed" name="check" :size="14" /></button>
        <div class="todo-item-content">
          <template v-if="editingId === todo.id">
            <form class="todo-edit-form" @submit.prevent="commitEdit(todo)" @pointerdown.stop>
              <input v-model="editTitle" class="todo-edit-title" aria-label="编辑待办内容" maxlength="160" @keydown.esc="editingId = null" />
              <input v-model="editDueDate" class="todo-edit-date" type="date" aria-label="截止日期" />
              <button class="todo-save-edit" type="submit" :disabled="savingEditId === todo.id" @click.stop>{{ savingEditId === todo.id ? '保存中…' : '保存' }}</button>
            </form>
          </template>
          <template v-else>
            <button class="todo-title-button" :title="todo.title" @dblclick="beginEdit(todo)">{{ todo.title }}</button>
            <span v-if="todo.dueDate" class="todo-due" :class="{ overdue: isOverdue(todo) }">{{ isOverdue(todo) ? '已逾期 · ' : '' }}{{ formatDueDate(todo.dueDate) }}</span>
          </template>
        </div>
        <button class="todo-more" :aria-label="`编辑 ${todo.title}`" title="编辑" @click="beginEdit(todo)"><AppIcon name="more" :size="16" /></button>
        <button class="todo-delete" :aria-label="`删除 ${todo.title}`" title="删除" @click="remove(todo)"><AppIcon name="close" :size="14" /></button>
      </article>
    </section>

    <DialogRoot :open="composerOpen" @update:open="onComposerOpenChange">
      <DialogPortal>
        <DialogOverlay class="todo-compose-overlay" />
        <DialogContent class="todo-compose-panel">
          <header class="todo-compose-header">
            <DialogTitle class="todo-compose-title">新建待办</DialogTitle>
            <DialogClose as-child>
              <button class="todo-compose-close" type="button" aria-label="关闭新建待办" @click="closeComposer"><AppIcon name="close" :size="16" /></button>
            </DialogClose>
          </header>
          <form class="todo-compose-form" @submit.prevent="addTodo">
            <label class="todo-compose-label" for="todo-create-title">待办内容</label>
            <input id="todo-create-title" ref="composerInput" v-model="draft" class="todo-create-input" placeholder="添加一项待办" maxlength="160" />
            <label class="todo-compose-label" for="todo-create-date">截止日期</label>
            <input id="todo-create-date" v-model="dueDate" class="todo-create-date" type="date" />
            <footer class="todo-compose-actions">
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
      <button @click="toggleLayer"><AppIcon name="arrow-up-right" :size="16" />{{ snapshot.settings.widgets.todo.alwaysOnTop ? '取消置顶' : '始终置顶' }}</button>
      <div class="context-divider"></div>
      <button class="context-danger" @click="closeWidget"><AppIcon name="close" :size="16" />关闭待办组件</button>
    </div>
  </main>
</template>
