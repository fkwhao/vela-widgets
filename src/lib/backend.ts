import { emitTo, listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { defaultSnapshot, type AppSnapshot, type ThemeMode, type TodoItem, type WidgetKind, type WidgetSize, widgetKinds, type ClockSettings, type CountdownItem } from "../types";

const previewKey = "vela.preview.snapshot.v1";

export function isNativeApp(): boolean {
  return typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);
}

function readPreview(): AppSnapshot {
  try {
    const stored = localStorage.getItem(previewKey);
    if (stored) {
      const parsed = JSON.parse(stored) as Partial<AppSnapshot>;
      const defaults = structuredClone(defaultSnapshot);
      return {
        ...defaults,
        ...parsed,
        settings: {
          ...defaults.settings,
          ...parsed.settings,
          widgetTransparency: parsed.settings?.widgetTransparency ?? defaults.settings.widgetTransparency,
          widgetCornerRadius: parsed.settings?.widgetCornerRadius ?? defaults.settings.widgetCornerRadius,
          clock: { ...defaults.settings.clock, ...parsed.settings?.clock },
          note: { ...defaults.settings.note, ...parsed.settings?.note },
          // Merge per widget so fields added later (such as size) keep their defaults.
          widgets: Object.fromEntries(widgetKinds.map((kind) => [kind, { ...defaults.settings.widgets[kind], ...parsed.settings?.widgets?.[kind] }])) as AppSnapshot["settings"]["widgets"],
        },
        todos: Array.isArray(parsed.todos) ? parsed.todos : defaults.todos,
        countdowns: Array.isArray(parsed.countdowns) ? parsed.countdowns : [],
      };
    }
  } catch {
    // An unreadable preview snapshot is replaced with the safe first-run state.
  }
  return structuredClone(defaultSnapshot);
}

function writePreview(snapshot: AppSnapshot): AppSnapshot {
  localStorage.setItem(previewKey, JSON.stringify(snapshot));
  return snapshot;
}

function updatePreview(update: (snapshot: AppSnapshot) => void): AppSnapshot {
  const snapshot = readPreview();
  update(snapshot);
  return writePreview(snapshot);
}

export async function getSnapshot(): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke<AppSnapshot>("get_snapshot");
  return readPreview();
}

export async function setWidgetEnabled(kind: WidgetKind, enabled: boolean): Promise<AppSnapshot> {
  if (isNativeApp()) {
    if (kind === "note" && !enabled) await flushNativeNote();
    return invoke<AppSnapshot>("set_widget_enabled", { kind, enabled });
  }
  return updatePreview((snapshot) => {
    snapshot.settings.widgets[kind].enabled = enabled;
  });
}

export async function setWidgetLayer(kind: WidgetKind, alwaysOnTop: boolean): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke<AppSnapshot>("set_widget_layer", { kind, alwaysOnTop });
  return updatePreview((snapshot) => {
    snapshot.settings.widgets[kind].alwaysOnTop = alwaysOnTop;
  });
}

export async function setWidgetLocked(kind: WidgetKind, locked: boolean): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke<AppSnapshot>("set_widget_locked", { kind, locked });
  return updatePreview((snapshot) => {
    snapshot.settings.widgets[kind].locked = locked;
  });
}

export async function setWidgetSize(kind: WidgetKind, size: WidgetSize): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke<AppSnapshot>("set_widget_size", { kind, size });
  return updatePreview((snapshot) => {
    snapshot.settings.widgets[kind].size = size;
  });
}

export async function setWeekStartsMonday(monday: boolean): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke<AppSnapshot>("set_week_starts_monday", { monday });
  return updatePreview((snapshot) => {
    snapshot.settings.weekStartsMonday = monday;
  });
}

export async function setTheme(theme: ThemeMode): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke<AppSnapshot>("set_theme", { theme });
  return updatePreview((snapshot) => {
    snapshot.settings.theme = theme;
  });
}

export async function setAccentColor(color: string): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke<AppSnapshot>("set_accent_color", { color });
  return updatePreview((snapshot) => {
    snapshot.settings.accentColor = color;
  });
}

export async function setWidgetAppearance(
  widgetTransparency: number,
  widgetCornerRadius: number,
): Promise<AppSnapshot> {
  if (isNativeApp()) {
    return invoke<AppSnapshot>("set_widget_appearance", { widgetTransparency, widgetCornerRadius });
  }
  return updatePreview((snapshot) => {
    snapshot.settings.widgetTransparency = widgetTransparency;
    snapshot.settings.widgetCornerRadius = widgetCornerRadius;
  });
}

export async function saveWidgetPosition(
  kind: WidgetKind,
  position: { x: number; y: number },
): Promise<void> {
  if (isNativeApp()) {
    await invoke("save_widget_position", { kind, ...position });
    return;
  }
  updatePreview((snapshot) => {
    Object.assign(snapshot.settings.widgets[kind], position);
  });
}

export async function createTodo(title: string, dueDate: string | null): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke<AppSnapshot>("create_todo", { title, dueDate });
  return updatePreview((snapshot) => {
    const todo: TodoItem = {
      id: Date.now(),
      title: title.trim(),
      dueDate,
      completed: false,
      createdAt: new Date().toISOString(),
    };
    snapshot.todos.unshift(todo);
  });
}

export async function updateTodo(
  id: number,
  title: string,
  dueDate: string | null,
): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke<AppSnapshot>("update_todo", { id, title, dueDate });
  return updatePreview((snapshot) => {
    const todo = snapshot.todos.find((item) => item.id === id);
    if (todo) Object.assign(todo, { title: title.trim(), dueDate });
  });
}

export async function setTodoCompleted(id: number, completed: boolean): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke<AppSnapshot>("set_todo_completed", { id, completed });
  return updatePreview((snapshot) => {
    const todo = snapshot.todos.find((item) => item.id === id);
    if (todo) todo.completed = completed;
  });
}

export async function deleteTodo(id: number): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke<AppSnapshot>("delete_todo", { id });
  return updatePreview((snapshot) => {
    snapshot.todos = snapshot.todos.filter((todo) => todo.id !== id);
  });
}

export async function openManager(): Promise<void> {
  if (isNativeApp()) {
    await invoke("show_manager");
    return;
  }
  window.location.search = "?view=manager";
}

export async function showWidgetContextMenu(
  kind: WidgetKind,
  x: number,
  y: number,
): Promise<void> {
  if (isNativeApp()) await invoke("show_context_menu", { kind, x, y });
}

export async function exitVela(): Promise<void> {
  if (isNativeApp()) { if ((await getSnapshot()).settings.widgets.note.enabled) await flushNativeNote(); await invoke("exit_vela"); }
  else window.close();
}

export async function setClockSettings(clock: ClockSettings): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke("set_clock_settings", { clock });
  return updatePreview((s) => { s.settings.clock = clock; });
}
export async function saveNote(text: string): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke("save_note", { text });
  return updatePreview((s) => { s.settings.note.text = text; });
}
export async function setNoteColor(color: string): Promise<AppSnapshot> {
  if (!/^#[0-9a-f]{6}$/i.test(color)) throw new Error("请选择有效的颜色。");
  color = color.toLowerCase();
  if (isNativeApp()) return invoke("set_note_color", { color });
  return updatePreview((s) => { s.settings.note.color = color; });
}
export async function saveCountdown(item: Omit<CountdownItem, "id" | "createdDate"> & { id: number | null; createdDate: string }): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke("save_countdown", { item });
  return updatePreview((s) => {
    if (item.id !== null) { const current = s.countdowns.find((c) => c.id === item.id); if (current) Object.assign(current, item); }
    else s.countdowns.push({ ...item, id: Date.now() });
  });
}
export async function deleteCountdown(id: number): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke("delete_countdown", { id });
  return updatePreview((s) => { s.countdowns = s.countdowns.filter((c) => c.id !== id); });
}

async function flushNativeNote(): Promise<void> {
  const requestId = crypto.randomUUID();
  let unlisten: (() => void) | undefined;
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    await new Promise<void>((resolve, reject) => {
      timer = setTimeout(() => reject(new Error("便签尚未保存，请稍后再试。")), 5000);
      void listen<{ requestId: string; ok: boolean }>("vela://note-flushed", ({ payload }) => {
        if (payload.requestId === requestId) payload.ok ? resolve() : reject(new Error("请先保存便签再关闭。"));
      }).then((stop) => { unlisten = stop; return emitTo("note", "vela://note-flush", { requestId }); }).catch(reject);
    });
  } finally { if (timer) clearTimeout(timer); unlisten?.(); }
}
