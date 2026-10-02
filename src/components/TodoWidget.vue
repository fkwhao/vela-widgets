<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import AppIcon from "./AppIcon.vue";
import { openManager } from "../lib/backend";
import { createTodo, deleteTodo, setTodoCompleted, setWidgetEnabled, setWidgetLayer, snapshot, updateTodo } from "../lib/store";
import { useWindowBounds } from "../lib/useWindowBounds";
import type { TodoItem } from "../types";

const filter = ref<"open" | "done">("open");
const draft = ref("");
const dueDate = ref("");
const showDateInput = ref(false);
const editingId = ref<number | null>(null);
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
    showDateInput.value = false;
  } catch (error) {
    showNotice(typeof error === "string" ? error : "待办没有保存，请重试。");
  }
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
  if (!editTitle.value.trim()) {
    showNotice("待办内容不能为空。");
    return;
  }
  try {
    await updateTodo(todo.id, editTitle.value, editDueDate.value || null);
    editingId.value = null;
  } catch {
    showNotice("待办没有保存，请重试。");
  }
}

async function remove(todo: TodoItem): Promise<void> {
  try {
    await deleteTodo(todo.id);
  } catch {
    showNotice("没有删除，请重试。");
  }
}

function openContextMenu(event: MouseEvent): void {
  const target = event.target;
  if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || (target instanceof HTMLElement && target.isContentEditable)) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  menu.value = {
    x: Math.max(6, Math.min(event.clientX, window.innerWidth - 211)),
    y: Math.max(6, Math.min(event.clientY, window.innerHeight - 158)),
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

useWindowBounds("todo");
onMounted(() => window.addEventListener("pointerdown", dismissMenu));
onUnmounted(() => {
  window.removeEventListener("pointerdown", dismissMenu);
  if (noticeTimer) clearTimeout(noticeTimer);
});
</script>

<template>
  <main class="widget-window todo-widget" @contextmenu="openContextMenu">
    <header class="widget-header todo-header" :class="{ 'widget-header-draggable': !snapshot.settings.widgets.todo.locked }" :data-tauri-drag-region="snapshot.settings.widgets.todo.locked ? undefined : 'deep'">
      <div class="widget-brandline">
        <span class="widget-brand-dot todo-brand-dot"></span>
        <span>待办</span>
        <span class="todo-count">{{ activeCount }} 项未完成</span>
      </div>
      <span class="widget-header-caption">DESKTOP</span>
    </header>

    <div class="todo-tabs" role="tablist">
      <button :class="{ active: filter === 'open' }" role="tab" :aria-selected="filter === 'open'" @click="filter = 'open'">未完成 <span>{{ openTodos.length }}</span></button>
      <button :class="{ active: filter === 'done' }" role="tab" :aria-selected="filter === 'done'" @click="filter = 'done'">已完成 <span>{{ completedTodos.length }}</span></button>
    </div>

    <section class="todo-list" aria-label="待办列表">
      <div v-if="visibleTodos.length === 0" class="todo-empty">
        <span class="empty-checkmark"><AppIcon name="check" :size="19" /></span>
        <strong>{{ filter === 'open' ? '今天从容一点' : '还没有完成的待办' }}</strong>
        <p>{{ filter === 'open' ? '写下第一件要做的事。' : '完成的事项会留在这里。' }}</p>
      </div>

      <article v-for="todo in visibleTodos" :key="todo.id" class="todo-item" :class="{ completed: todo.completed }">
        <button class="todo-check" :aria-label="todo.completed ? '取消完成' : '标记完成'" :aria-pressed="todo.completed" @click="toggle(todo)"><AppIcon v-if="todo.completed" name="check" :size="14" /></button>
        <div class="todo-item-content">
          <template v-if="editingId === todo.id">
            <input v-model="editTitle" class="todo-edit-title" aria-label="编辑待办内容" @keydown.enter.prevent="commitEdit(todo)" @keydown.esc="editingId = null" />
            <input v-model="editDueDate" class="todo-edit-date" type="date" aria-label="截止日期" />
            <button class="todo-save-edit" @click="commitEdit(todo)">保存</button>
          </template>
          <template v-else>
            <button class="todo-title-button" :title="todo.title" @dblclick="beginEdit(todo)">{{ todo.title }}</button>
            <span v-if="todo.dueDate" class="todo-due" :class="{ overdue: isOverdue(todo) }">{{ isOverdue(todo) ? '已逾期 · ' : '' }}{{ formatDueDate(todo.dueDate) }}</span>
            <span v-else class="todo-due empty-due">双击编辑 · 添加日期</span>
          </template>
        </div>
        <button class="todo-more" :aria-label="`编辑 ${todo.title}`" title="编辑" @click="beginEdit(todo)"><AppIcon name="more" :size="16" /></button>
        <button class="todo-delete" :aria-label="`删除 ${todo.title}`" title="删除" @click="remove(todo)"><AppIcon name="close" :size="14" /></button>
      </article>
    </section>

    <form class="todo-composer" @submit.prevent="addTodo">
      <div class="composer-main">
        <span class="composer-plus"><AppIcon name="plus" :size="16" /></span>
        <input v-model="draft" placeholder="添加一项待办…" aria-label="添加一项待办" @keydown.esc="draft = ''" />
        <button v-if="draft.trim()" class="composer-submit" type="submit">添加</button>
      </div>
      <div class="composer-extras">
        <button class="date-chip" type="button" :class="{ selected: showDateInput || dueDate }" @click="showDateInput = !showDateInput"><AppIcon name="calendar" :size="13" />{{ dueDate ? formatDueDate(dueDate) : '截止日期' }}</button>
        <span class="composer-hint">Enter 添加</span>
        <input v-if="showDateInput" v-model="dueDate" class="composer-date-input" type="date" aria-label="选择截止日期" />
      </div>
    </form>

    <transition name="notice"><div v-if="notice" class="widget-notice">{{ notice }}</div></transition>

    <div v-if="menu" class="widget-context-menu" :style="{ left: `${menu.x}px`, top: `${menu.y}px` }" @pointerdown.stop>
      <div class="context-menu-label">VELA WIDGET</div>
      <button @click="openManager(); menu = null"><AppIcon name="sliders" :size="16" />Vela 偏好设置</button>
      <button @click="toggleLayer"><AppIcon name="arrow-up-right" :size="16" />{{ snapshot.settings.widgets.todo.alwaysOnTop ? '取消置顶' : '始终置顶' }}</button>
      <div class="context-divider"></div>
      <button class="context-danger" @click="closeWidget"><AppIcon name="close" :size="16" />关闭待办组件</button>
    </div>
  </main>
</template>
