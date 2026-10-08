<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import WidgetPageControls from "../../../shared/widgets/WidgetPageControls.vue";
import { paginateTodos } from "../todoPages";
import AppIcon from "../../../shared/ui/AppIcon.vue";
import WidgetSizeMenuRow from "../../../shared/widgets/WidgetSizeMenuRow.vue";
import { useContextMenu } from "../../../shared/composables/useContextMenu";
import VelaDatePicker from "../../../shared/ui/VelaDatePicker.vue";
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "reka-ui";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isNativeApp, openManager, showWidgetContextMenu } from "../../../infrastructure/backend";
import {
  createTodo,
  deleteTodo,
  setTodoCompleted,
  setWidgetEnabled,
  setWidgetLayer,
  setWidgetSize,
  snapshot,
  updateTodo,
} from "../../../app/store";
import { useDesktopCanvas } from '../../../features/desktop/context';
import { useWindowBounds } from "../../../shared/composables/useWindowBounds";
import type { TodoItem, WidgetSize } from "../../../shared/types";

const filter = ref<"open" | "done">("open");
const draft = ref("");
const dueDate = ref("");
const composerOpen = ref(false);
const composerInput = ref<HTMLInputElement | null>(null);
const editingId = ref<number | null>(null);
const savingEditId = ref<number | null>(null);
const editTitle = ref("");
const editDueDate = ref("");
const { menu, menuElement, placeMenu } = useContextMenu();
const notice = ref("");
const justAdded = ref(false);
const adding = ref(false);
let noticeTimer: ReturnType<typeof setTimeout> | undefined;
let addedTimer: ReturnType<typeof setTimeout> | undefined;
// The ⋯ menu returns focus to its trigger on close, except when "编辑" moves it into the field.
let keepEditFocus = false;

const openTodos = computed(() => snapshot.value.todos.filter((todo) => !todo.completed));
const completedTodos = computed(() => snapshot.value.todos.filter((todo) => todo.completed));
const visibleTodos = computed(() => (filter.value === "open" ? openTodos.value : completedTodos.value));
const activeCount = computed(() => openTodos.value.length);
const widget = computed(() => snapshot.value.settings.widgets.todo);
const size = computed(() => widget.value.size);
const canvas = useDesktopCanvas();
const dragRegion = computed(() => (canvas || widget.value.locked ? undefined : "deep"));
const page = ref(0);
const pageDirection = ref(1);
const pageSource = computed(() => size.value === "large" ? visibleTodos.value : openTodos.value);
const pages = computed(() => paginateTodos(pageSource.value, size.value, editingId.value, composerOpen.value ? (size.value === "large" ? 68 : 48) : 0));
const pageItems = computed(() => pages.value[Math.min(page.value, pages.value.length - 1)] ?? []);
watch([size, filter], () => { page.value = 0; editingId.value = null; });
watch(pages, (next) => {
  if (editingId.value !== null) { const index = next.findIndex(items => items.some(item => item.id === editingId.value)); if (index >= 0) page.value = index; }
  page.value = Math.min(page.value, next.length - 1);
});
function movePage(direction: number) { pageDirection.value = direction; page.value = Math.max(0, Math.min(pages.value.length - 1, page.value + direction)); }
// Browser previews use the same fixed footprint as native windows.
const previewSize = computed(() => isNativeApp() || canvas ? undefined : { width: `${size.value === "small" ? 170 : 364}px`, height: `${size.value === "large" ? 384 : 170}px` });
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

// Like Microsoft To Do, the composer stays open after adding so several tasks can be typed in a row.
async function addTodo(): Promise<void> {
  if (!draft.value.trim() || adding.value) return;
  adding.value = true;
  try {
    await createTodo(draft.value, dueDate.value || null);
    page.value = 0;
    draft.value = "";
    justAdded.value = true;
    if (addedTimer) clearTimeout(addedTimer);
    addedTimer = setTimeout(() => (justAdded.value = false), 1600);
    await nextTick();
    composerInput.value?.focus();
  } catch (error) {
    showNotice(typeof error === "string" ? error : "待办没有保存，请重试。");
  } finally { adding.value = false; }
}

async function openComposer(): Promise<void> {
  menu.value = null;
  filter.value = "open";
  composerOpen.value = true;
  await nextTick();
  composerInput.value?.focus();
}

function closeComposer(): void {
  composerOpen.value = false;
  draft.value = "";
  dueDate.value = "";
  justAdded.value = false;
}

async function toggle(todo: TodoItem): Promise<void> {
  try {
    await setTodoCompleted(todo.id, !todo.completed);
  } catch {
    showNotice("没有保存，请重试。");
  }
}

// Inline rename, as in Explorer or To Do: one click opens the field with the text selected;
// Enter or clicking elsewhere saves, Esc restores the original.
async function beginEdit(todo: TodoItem): Promise<void> {
  if (editingId.value === todo.id) return;
  const current = editingTodo();
  if (current) await commitEdit(current, true);
  editingId.value = todo.id;
  editTitle.value = todo.title;
  editDueDate.value = todo.dueDate ?? "";
  await nextTick();
  const field = document.querySelector<HTMLInputElement>(".todo-edit-title");
  field?.focus();
  field?.select();
}

function editingTodo(): TodoItem | undefined {
  return snapshot.value.todos.find((todo) => todo.id === editingId.value);
}

function cancelEdit(): void {
  editingId.value = null;
}

async function commitEdit(todo: TodoItem, quiet = false): Promise<void> {
  if (savingEditId.value === todo.id) return;
  const title = editTitle.value.trim();
  if (!title) {
    // Clearing the text and clicking away keeps the old title rather than leaving an error behind.
    if (quiet) cancelEdit();
    else showNotice("待办内容不能为空。");
    return;
  }
  if (title === todo.title && (editDueDate.value || null) === todo.dueDate) {
    if (editingId.value === todo.id) editingId.value = null;
    return;
  }
  savingEditId.value = todo.id;
  try {
    await updateTodo(todo.id, title, editDueDate.value || null);
    if (editingId.value === todo.id) editingId.value = null;
  } catch (error) {
    showNotice(typeof error === "string" ? error : "待办没有保存，请重试。");
  } finally {
    savingEditId.value = null;
  }
}

function commitIfOutside(event: PointerEvent): void {
  if (editingId.value === null) return;
  const target = event.target as Element | null;
  if (target?.closest(".todo-edit-form, [data-vela-picker], .wg-menu")) return;
  const todo = editingTodo();
  if (todo) void commitEdit(todo, true);
}

function commitOnWindowBlur(): void {
  // Clicking the desktop or another app also ends the rename.
  if (document.querySelector("[data-vela-picker]")) return;
  const todo = editingTodo();
  if (todo) void commitEdit(todo, true);
}

function chooseEdit(todo: TodoItem): void {
  keepEditFocus = true;
  void beginEdit(todo);
}

function onMenuCloseFocus(event: Event): void {
  if (keepEditFocus) event.preventDefault();
  keepEditFocus = false;
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
  if (isNativeApp() && !canvas) {
    try {
      // The native popup clamps itself to the monitor, not to this small window.
      await showWidgetContextMenu("todo", event.clientX, event.clientY);
      return;
    } catch {
      // Fall through to the in-window menu.
    }
  }
  await placeMenu(event);
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
  window.addEventListener("pointerdown", commitIfOutside, true);
  window.addEventListener("blur", commitOnWindowBlur);
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
  window.removeEventListener("pointerdown", commitIfOutside, true);
  window.removeEventListener("blur", commitOnWindowBlur);
  unlistenCompose?.();
  if (noticeTimer) clearTimeout(noticeTimer);
  if (addedTimer) clearTimeout(addedTimer);
});
</script>

<template>
  <main
    class="widget-window todo-widget"
    :class="[`size-${size}`, { 'widget-draggable': !widget.locked, 'is-composing': composerOpen }]"
    :style="previewSize"
    @contextmenu="openContextMenu"
  >
    <!-- Small: fixed heading and vertically paged unfinished tasks -->
    <template v-if="size === 'small'">
      <div class="todo-glance-head" :data-tauri-drag-region="dragRegion">
        <span class="todo-glyph"><AppIcon name="tick" :size="14" /></span>
        <div class="todo-small-actions"><button class="widget-icon-button" aria-label="新建待办" @click="openComposer"><AppIcon name="plus" :size="14" /></button><strong class="todo-big-count">{{ activeCount }}</strong></div>
      </div>
      <span class="todo-glance-title" :data-tauri-drag-region="dragRegion">待办</span>
      <form v-if="composerOpen" class="todo-inline-composer" aria-label="新建待办" :aria-busy="adding" @submit.prevent="addTodo" @keydown.esc.stop.prevent="closeComposer">
        <input ref="composerInput" v-model="draft" class="todo-inline-title" aria-label="新待办名称" placeholder="添加一项待办…" maxlength="160" autocomplete="off" :disabled="adding" />
        <div class="todo-inline-meta"><VelaDatePicker v-model="dueDate" label="截止日期" placeholder="日期" clearable /><span v-if="justAdded" class="todo-added-mark" role="status">已添加</span><button type="button" class="widget-icon-button todo-inline-cancel" aria-label="关闭新增待办" @click="closeComposer"><AppIcon name="close" :size="12" /></button><button type="submit" class="widget-icon-button todo-inline-submit" aria-label="添加待办" :disabled="adding || !draft.trim()"><AppIcon name="plus" :size="14" /></button></div>
      </form>
      <ul v-if="openTodos.length" class="todo-glance-list todo-page-list" :key="`${size}-${page}`" :data-direction="pageDirection > 0 ? 'down' : 'up'" aria-label="待办列表">
        <li v-for="todo in pageItems" :key="todo.id">
          <button class="todo-check" aria-label="标记完成" :aria-pressed="false" @click="toggle(todo)"></button>
          <span class="todo-glance-text" :data-tooltip="todo.title">{{ todo.title }}</span>
        </li>
      </ul>
      <p v-else-if="!composerOpen" class="todo-glance-empty">没有待办事项</p>
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
          <form v-if="composerOpen" class="todo-inline-composer" aria-label="新建待办" :aria-busy="adding" @submit.prevent="addTodo" @keydown.esc.stop.prevent="closeComposer">
        <input ref="composerInput" v-model="draft" class="todo-inline-title" aria-label="新待办名称" placeholder="添加一项待办…" maxlength="160" autocomplete="off" :disabled="adding" />
        <div class="todo-inline-meta"><VelaDatePicker v-model="dueDate" label="截止日期" placeholder="日期" clearable /><span v-if="justAdded" class="todo-added-mark" role="status">已添加</span><button type="button" class="widget-icon-button todo-inline-cancel" aria-label="关闭新增待办" @click="closeComposer"><AppIcon name="close" :size="12" /></button><button type="submit" class="widget-icon-button todo-inline-submit" aria-label="添加待办" :disabled="adding || !draft.trim()"><AppIcon name="plus" :size="14" /></button></div>
      </form>
          <ul v-if="openTodos.length" class="todo-glance-list todo-page-list" :key="`${size}-${page}`" :data-direction="pageDirection > 0 ? 'down' : 'up'" aria-label="待办列表">
            <li v-for="todo in pageItems" :key="todo.id">
              <button class="todo-check" aria-label="标记完成" :aria-pressed="false" @click="toggle(todo)"></button>
              <span class="todo-glance-text" :data-tooltip="todo.title">{{ todo.title }}</span>
              <span v-if="todo.dueDate" class="todo-due-chip" :class="{ overdue: isOverdue(todo) }">{{ formatDueDate(todo.dueDate) }}</span>
            </li>
          </ul>
          <WidgetPageControls v-if="openTodos.length" :page="page" :count="pages.length" @move="movePage" />
          <div v-if="!openTodos.length && !composerOpen" class="todo-glance-empty">
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

      <form v-if="composerOpen" class="todo-inline-composer" aria-label="新建待办" :aria-busy="adding" @submit.prevent="addTodo" @keydown.esc.stop.prevent="closeComposer">
        <input ref="composerInput" v-model="draft" class="todo-inline-title" aria-label="新待办名称" placeholder="添加一项待办…" maxlength="160" autocomplete="off" :disabled="adding" />
        <div class="todo-inline-meta"><VelaDatePicker v-model="dueDate" label="截止日期" placeholder="日期" clearable /><span v-if="justAdded" class="todo-added-mark" role="status">已添加</span><button type="button" class="widget-icon-button todo-inline-cancel" aria-label="关闭新增待办" @click="closeComposer"><AppIcon name="close" :size="12" /></button><button type="submit" class="widget-icon-button todo-inline-submit" aria-label="添加待办" :disabled="adding || !draft.trim()"><AppIcon name="plus" :size="14" /></button></div>
      </form>
      <section class="todo-list todo-page-list" :key="`large-${page}`" :data-direction="pageDirection > 0 ? 'down' : 'up'" aria-label="待办列表">
        <div v-if="visibleTodos.length === 0 && !composerOpen" class="todo-empty">
          <span class="todo-empty-mark"><AppIcon name="tick" :size="18" /></span>
          <span>{{ filter === 'open' ? '没有待办事项' : '暂无已完成事项' }}</span>
          <button v-if="filter === 'open'" class="widget-chip" @click="openComposer"><AppIcon name="plus" :size="13" />新建待办</button>
        </div>

        <article v-for="todo in pageItems" :key="todo.id" class="todo-item" :class="{ completed: todo.completed, editing: editingId === todo.id }">
          <button class="todo-check" :aria-label="todo.completed ? '取消完成' : '标记完成'" :aria-pressed="todo.completed" @click="toggle(todo)"><AppIcon v-if="todo.completed" name="tick" :size="11" /></button>
          <div class="todo-item-content">
            <template v-if="editingId === todo.id">
              <form class="todo-edit-form" :aria-busy="savingEditId === todo.id" @submit.prevent="commitEdit(todo)" @keydown.esc.prevent.stop="cancelEdit">
                <input v-model="editTitle" class="wg-textbox todo-edit-title" aria-label="编辑待办内容，回车保存，Esc 取消" maxlength="160" spellcheck="false" autocomplete="off" enterkeyhint="done" />
                <VelaDatePicker v-model="editDueDate" label="截止日期" placeholder="添加截止日期" clearable />
              </form>
            </template>
            <template v-else>
              <button class="todo-title-button" :aria-label="`${todo.title}，点击重命名`" :data-tooltip="todo.title" @click="beginEdit(todo)">{{ todo.title }}</button>
              <span v-if="todo.dueDate" class="todo-due" :class="{ overdue: isOverdue(todo) }"><AppIcon name="clock" :size="11" />{{ isOverdue(todo) ? '已逾期 · ' : '' }}{{ formatDueDate(todo.dueDate) }}</span>
            </template>
          </div>
          <DropdownMenuRoot :modal="false">
            <DropdownMenuTrigger class="todo-more" :aria-label="`${todo.title}的更多操作`" data-tooltip="更多操作"><AppIcon name="more" :size="15" /></DropdownMenuTrigger>
            <DropdownMenuPortal>
              <DropdownMenuContent class="wg-menu" align="end" :side-offset="2" :collision-padding="6" @close-auto-focus="onMenuCloseFocus">
                <DropdownMenuItem class="wg-menu-item" @select="chooseEdit(todo)"><AppIcon name="edit" :size="14" />编辑</DropdownMenuItem>
                <DropdownMenuItem class="wg-menu-item" @select="toggle(todo)"><AppIcon name="tick" :size="14" />{{ todo.completed ? '标记为未完成' : '标记为已完成' }}</DropdownMenuItem>
                <DropdownMenuSeparator class="wg-menu-separator" />
                <DropdownMenuItem class="wg-menu-item danger" @select="remove(todo)"><AppIcon name="trash" :size="14" />删除</DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenuPortal>
          </DropdownMenuRoot>
        </article>
      </section>
      <WidgetPageControls :page="page" :count="pages.length" :editing="editingId !== null" @move="movePage" />
    </template>


    <transition name="notice"><div v-if="notice" class="widget-notice">{{ notice }}</div></transition>

    <Teleport to="body"><div v-if="menu" ref="menuElement" class="widget-context-menu" :style="{ left: `${menu.x}px`, top: `${menu.y}px` }" @pointerdown.stop>
      <button class="context-primary" @click="openComposer"><AppIcon name="plus" :size="16" />新建待办</button>
      <div class="context-divider"></div>
      <button @click="openManager(); menu = null"><AppIcon name="sliders" :size="16" />Vela 偏好设置</button>
      <button @click="toggleLayer"><AppIcon name="arrow-up-right" :size="16" />{{ widget.alwaysOnTop ? '取消置顶' : '始终置顶' }}</button>
      <div class="context-divider"></div>
      <WidgetSizeMenuRow :size="size" @choose="chooseSize" />
      <div class="context-divider"></div>
      <button class="context-danger" @click="closeWidget"><AppIcon name="close" :size="16" />关闭待办组件</button>
    </div></Teleport>
  </main>
</template>
