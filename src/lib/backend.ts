import { invoke } from "@tauri-apps/api/core";
import { defaultSnapshot, type AppSnapshot, type ThemeMode, type TodoItem, type WidgetKind } from "../types";

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
          widgets: { ...defaults.settings.widgets, ...parsed.settings?.widgets },
        },
        todos: Array.isArray(parsed.todos) ? parsed.todos : defaults.todos,
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
  if (isNativeApp()) return invoke<AppSnapshot>("set_widget_enabled", { kind, enabled });
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

export async function saveWidgetBounds(
  kind: WidgetKind,
  bounds: { x: number; y: number; width: number; height: number },
): Promise<void> {
  if (isNativeApp()) {
    await invoke("save_widget_bounds", { kind, ...bounds });
    return;
  }
  updatePreview((snapshot) => {
    Object.assign(snapshot.settings.widgets[kind], bounds);
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

export async function exitVela(): Promise<void> {
  if (isNativeApp()) await invoke("exit_vela");
  else window.close();
}
