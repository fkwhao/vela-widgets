import { emitTo, listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { applyClockAction, tickClockTools } from "./clockTools";
import { normalizeNotes, nextNoteId } from "./notes";
import { validateCalendarEvent } from "./calendarEvents";
import { dateKey, tickHabitReminders, validateHabit, validateRecord } from "./habits";
import type { HabitItem, HabitRecordInput, HabitSettings } from "../types";
import { fetchHolidayUpdate, holidayUpdateDue, mergeHolidayData, normalizeHolidayCache } from "./holidays";
import { defaultSnapshot, type AppSnapshot, type ThemeMode, type TodoItem, type WidgetKind, type WidgetSize, widgetKinds, type ClockSettings, type CountdownItem } from "../types";

const previewKey = "vela.preview.snapshot.v1";

export async function saveHabit(item: HabitItem): Promise<AppSnapshot> {
  const error = validateHabit(item); if (error) throw error;
  if (isNativeApp()) return invoke("save_habit", { item });
  return updatePreview(s => {
    const copy = { ...item, title: item.title.trim(), weekdays: [...item.weekdays], lastRemindedDate: item.id ? s.habits.find(h => h.id === item.id)?.lastRemindedDate ?? null : null };
    if (item.id) { const index = s.habits.findIndex(h => h.id === item.id); if (index < 0) throw "找不到这个习惯。"; s.habits[index] = copy; }
    else { const id = Math.max(0, ...s.habits.map(h => h.id)) + 1; s.habits.push({ ...copy, id, sortOrder: Math.max(-1, ...s.habits.map(h => h.sortOrder)) + 1 }); }
  });
}
export async function deleteHabit(id: number): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke("delete_habit", { id });
  return updatePreview(s => { s.habits = s.habits.filter(h => h.id !== id); s.habitRecords = s.habitRecords.filter(r => r.habitId !== id); if (s.settings.habit.selectedIds) s.settings.habit.selectedIds = s.settings.habit.selectedIds.filter(i => i !== id); });
}
export async function reorderHabits(ids: number[]): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke("reorder_habits", { ids });
  return updatePreview(s => { if (new Set(ids).size !== s.habits.length || ids.length !== s.habits.length || ids.some(id => !s.habits.some(h => h.id === id))) throw "习惯列表已变化，请重试。"; ids.forEach((id, i) => { s.habits.find(h => h.id === id)!.sortOrder = i; }); });
}
export async function setHabitSettings(settings: HabitSettings): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke("set_habit_settings", { settings });
  return updatePreview(s => { if (!["card", "list", "report"].includes(settings.style) || (settings.selectedIds !== null && settings.selectedIds.some(id => !s.habits.some(h => h.id === id)))) throw "无效的组件设置。"; s.settings.habit = { style: settings.style, selectedIds: settings.selectedIds === null ? null : [...settings.selectedIds] }; });
}
export async function adjustHabitRecord(input: HabitRecordInput): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke("adjust_habit_record", { input });
  return updatePreview(s => {
    const habit = s.habits.find(h => h.id === input.habitId);
    let record = s.habitRecords.find(r => r.habitId === input.habitId && r.date === input.date);
    const error = validateRecord(habit, input, dateKey(), !!record); if (error) throw error;
    if (!record && input.delta <= 0) return;
    if (!record) { record = { habitId: input.habitId, date: input.date, count: 0, target: habit!.dailyTarget, updatedAt: "", mood: null, rating: null, result: null }; s.habitRecords.push(record); }
    record.count = Math.max(0, Math.min(record.target, record.count + input.delta));
    if (input.delta === 0 || input.captureMetadata) {
      record.mood = input.mood ?? null;
      record.rating = input.rating ?? null;
      record.result = input.result ?? null;
    }
    record.updatedAt = new Date().toISOString();
    if (!record.count) s.habitRecords = s.habitRecords.filter(r => r !== record);
  });
}

export function isNativeApp(): boolean {
  return typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);
}

function readPreview(): AppSnapshot {
  try {
    const stored = localStorage.getItem(previewKey);
    if (stored) {
      const parsed = JSON.parse(stored) as Partial<AppSnapshot>;
      const defaults = structuredClone(defaultSnapshot);
      const next: AppSnapshot = {
        ...defaults,
        ...parsed,
        settings: {
          ...defaults.settings,
          ...parsed.settings,
          widgetTransparency: parsed.settings?.widgetTransparency ?? defaults.settings.widgetTransparency,
          widgetCornerRadius: parsed.settings?.widgetCornerRadius ?? defaults.settings.widgetCornerRadius,
          clock: { ...defaults.settings.clock, ...parsed.settings?.clock },
          clockTools: { ...defaults.settings.clockTools, ...parsed.settings?.clockTools },
          calendar: { ...defaults.settings.calendar, ...parsed.settings?.calendar },
          note: normalizeNotes(parsed.settings?.note ?? defaults.settings.note),
          habit: { ...defaults.settings.habit, ...parsed.settings?.habit },
          // Merge per widget so fields added later (such as size) keep their defaults.
          widgets: Object.fromEntries(widgetKinds.map((kind) => [kind, { ...defaults.settings.widgets[kind], ...parsed.settings?.widgets?.[kind] }])) as AppSnapshot["settings"]["widgets"],
        },
        todos: Array.isArray(parsed.todos) ? parsed.todos : defaults.todos,
        countdowns: Array.isArray(parsed.countdowns) ? parsed.countdowns : [],
        habits: Array.isArray(parsed.habits) ? parsed.habits.map(h => ({ ...h, reminderTime: h.reminderTime ?? null, lastRemindedDate: h.lastRemindedDate ?? null })) : [],
        habitRecords: Array.isArray(parsed.habitRecords) ? parsed.habitRecords : [],
        holidays: normalizeHolidayCache(parsed.holidays, defaults.holidays.data),
      };
      if (JSON.stringify(next) !== stored) writePreview(next);
      return next;
    }
  } catch {
    // An unreadable preview snapshot is replaced with the safe first-run state.
  }
  const initial = structuredClone(defaultSnapshot);
  initial.settings.note = normalizeNotes(initial.settings.note);
  return writePreview(initial);
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

export async function openManager(page?: string): Promise<void> {
  if (isNativeApp()) {
    await invoke("show_manager");
    if (typeof page === "string") await emitTo("manager", "vela://navigate", page);
    return;
  }
  window.location.search = `?view=manager${typeof page === "string" ? `&page=${encodeURIComponent(page)}` : ''}`;
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
  if (!["default", "classic", "digital", "world"].includes(clock.theme) || clock.cities.length > 4) throw new Error("时钟主题或城市配置无效。");
  return updatePreview((s) => { s.settings.clock = clock; });
}
export async function saveNote(id: number, text: string): Promise<AppSnapshot> {
  if (Array.from(text).length > 20000) throw new Error("便签最多保存 20000 字。");
  if (isNativeApp()) return invoke("save_note", { id, text });
  return updatePreview((s) => { const note = s.settings.note.notes.find(n => n.id === id); if (!note) throw new Error("这个便签已被删除。"); note.text = text; s.settings.note = normalizeNotes(s.settings.note); });
}

export async function setCalendarSettings(calendar: import("../types").CalendarSettings): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke("set_calendar_settings", { calendar });
  return updatePreview(s => { s.settings.calendar = { ...calendar, events: s.settings.calendar.events }; });
}

export async function saveCalendarEvent(event: import("../types").CalendarEvent): Promise<AppSnapshot> {
  validateCalendarEvent(event);
  if (isNativeApp()) return invoke("save_calendar_event", { event });
  return updatePreview(s => {
    const events = s.settings.calendar.events;
    if (event.id === 0) { if (events.length >= 5000) throw new Error("最多保存 5000 条日程。"); events.push({ ...event, title: event.title.trim(), id: Math.max(0, ...events.map(e => e.id)) + 1 }); }
    else { const index = events.findIndex(e => e.id === event.id); if (index < 0) throw new Error("这条日程已不存在。"); events[index] = { ...event, title: event.title.trim() }; }
  });
}
export async function deleteCalendarEvent(id: number): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke("delete_calendar_event", { id });
  return updatePreview(s => { s.settings.calendar.events = s.settings.calendar.events.filter(e => e.id !== id); });
}

let previewHolidayCheck: Promise<AppSnapshot> | undefined;
export async function checkHolidayUpdates(automatic = false): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke("check_holiday_updates");
  if (previewHolidayCheck) return previewHolidayCheck;
  const initial = readPreview();
  if (automatic && (!initial.settings.calendar.autoUpdate || !holidayUpdateDue(initial.holidays.lastAttemptAt))) return initial;
  updatePreview(s => { s.holidays.lastAttemptAt = Date.now(); });
  previewHolidayCheck = (async () => {
    let next: import("../types").HolidayData | null = null; let error: string | null = null;
    try { next = await fetchHolidayUpdate(initial.holidays.data); }
    catch (e) { error = e instanceof Error && /^(节假日|更新|下载)/.test(e.message) ? e.message : "无法连接节假日更新源，请稍后重试；本地数据仍可使用。"; }
    const result = updatePreview(s => {
      if (error) s.holidays.lastError = error;
      else {
        if (next) { s.holidays.data = mergeHolidayData(s.holidays.data, next); s.holidays.lastUpdatedAt = Date.now(); }
        s.holidays.lastCheckedAt = Date.now(); s.holidays.lastError = null;
      }
    });
    window.dispatchEvent(new CustomEvent("vela:holidays-updated", { detail: result }));
    return result;
  })();
  try { return await previewHolidayCheck; } finally { previewHolidayCheck = undefined; }
}
export async function createNote(): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke("create_note");
  return updatePreview(s => {
    if (s.settings.note.notes.length >= 200) throw new Error("最多保存 200 篇便签。");
    const id = nextNoteId(s.settings.note.notes);
    s.settings.note.notes.push({ id, text: "", color: s.settings.note.color, createdAt: Date.now(), deleteAfterHours: null, retentionOverride: false });
    s.settings.note.activeId = id; s.settings.note = normalizeNotes(s.settings.note);
  });
}
export async function selectNote(id: number): Promise<AppSnapshot> {
  if (isNativeApp()) { if ((await getSnapshot()).settings.widgets.note.enabled) await flushNativeNote(); return invoke("select_note", { id }); }
  return updatePreview(s => { if (!s.settings.note.notes.some(n => n.id === id)) throw new Error("这个便签已被删除。"); s.settings.note.activeId = id; s.settings.note = normalizeNotes(s.settings.note); });
}
export async function deleteNote(id: number): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke("delete_note", { id });
  return updatePreview(s => {
    const index = s.settings.note.notes.findIndex(n => n.id === id);
    if (index < 0) throw new Error("这个便签已被删除。");
    s.settings.note.notes.splice(index, 1);
    if (s.settings.note.activeId === id) s.settings.note.activeId = (s.settings.note.notes[index] ?? s.settings.note.notes.at(-1))?.id ?? 0;
    s.settings.note = normalizeNotes(s.settings.note);
  });
}
export async function restoreNote(item: import("../types").NoteItem): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke("restore_note", { item });
  return updatePreview(s => {
    if (s.settings.note.notes.length >= 200) throw new Error("最多保存 200 篇便签。");
    const id = nextNoteId(s.settings.note.notes);
    s.settings.note.notes.push({ ...item, id, deleteAfterHours: null, retentionOverride: true }); s.settings.note.activeId = id; s.settings.note = normalizeNotes(s.settings.note);
  });
}
export async function setNoteExpiry(id: number, hours: number | null, inherit = false): Promise<AppSnapshot> {
  if (hours !== null && (!Number.isInteger(hours) || hours < 1 || hours > 87600)) throw new Error("请输入 1–87600 小时。");
  if (isNativeApp()) return invoke("set_note_expiry", { id, hours, inherit });
  return updatePreview(s => { const item = s.settings.note.notes.find(n => n.id === id); if (!item) throw new Error("这个便签已被删除。"); item.retentionOverride = !inherit; item.deleteAfterHours = inherit ? null : hours; s.settings.note = normalizeNotes(s.settings.note); });
}
export async function setNoteDefaultExpiry(hours: number | null): Promise<AppSnapshot> {
  if (hours !== null && (!Number.isInteger(hours) || hours < 1 || hours > 87600)) throw new Error("请输入 1–87600 小时。");
  if (isNativeApp()) return invoke("set_note_default_expiry", { hours });
  return updatePreview(s => { s.settings.note.defaultDeleteAfterHours = hours; s.settings.note = normalizeNotes(s.settings.note); });
}
export async function openNoteLink(url: string): Promise<void> {
  if (!/^(https?:\/\/|mailto:)/i.test(url)) throw new Error("不支持打开这个链接。");
  if (isNativeApp()) await invoke("open_note_link", { url }); else window.open(url, "_blank", "noopener,noreferrer");
}
export async function setNoteColor(color: string): Promise<AppSnapshot> {
  if (!/^#[0-9a-f]{6}$/i.test(color)) throw new Error("请选择有效的颜色。");
  color = color.toLowerCase();
  if (isNativeApp()) return invoke("set_note_color", { color });
  return updatePreview((s) => { const note = s.settings.note.notes.find(n => n.id === s.settings.note.activeId); if (note) note.color = color; s.settings.note.color = color; s.settings.note = normalizeNotes(s.settings.note); });
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

export async function clockAction(input: import("../types").ClockAction): Promise<AppSnapshot> {
  if (isNativeApp()) return invoke("clock_action", { id: null, seconds: null, mode: null, alarm: null, ...input });
  const next=updatePreview(s => applyClockAction(s.settings.clockTools,input));
  window.dispatchEvent(new CustomEvent("vela:clock-updated", { detail: next }));
  return next;
}
export function tickPreviewClock(): void {
  const next=readPreview();
  const clockChanged = tickClockTools(next.settings.clockTools);
  const habitChanged = tickHabitReminders(next.habits, next.habitRecords, next.settings.clockTools);
  if(clockChanged || habitChanged) {
    writePreview(next); window.dispatchEvent(new CustomEvent("vela:clock-updated", { detail: next }));
  }
}
