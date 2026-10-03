import { ref } from "vue";
import * as backend from "./backend";
import { defaultSnapshot, type AppSnapshot, type ThemeMode, type WidgetKind, type WidgetSize } from "../types";

export const snapshot = ref<AppSnapshot>(structuredClone(defaultSnapshot));
export const ready = ref(false);
export const storeError = ref("");

export function applySnapshot(next: AppSnapshot): void {
  snapshot.value = next;
  storeError.value = "";
}

export async function refreshSnapshot(): Promise<void> {
  try {
    applySnapshot(await backend.getSnapshot());
  } catch {
    storeError.value = "Vela 暂时无法读取本地数据。";
  } finally {
    ready.value = true;
  }
}

export async function setWidgetEnabled(kind: WidgetKind, enabled: boolean): Promise<void> {
  applySnapshot(await backend.setWidgetEnabled(kind, enabled));
}

export async function setWidgetLayer(kind: WidgetKind, alwaysOnTop: boolean): Promise<void> {
  applySnapshot(await backend.setWidgetLayer(kind, alwaysOnTop));
}

export async function setWidgetLocked(kind: WidgetKind, locked: boolean): Promise<void> {
  applySnapshot(await backend.setWidgetLocked(kind, locked));
}

export async function setWidgetSize(kind: WidgetKind, size: WidgetSize): Promise<void> {
  applySnapshot(await backend.setWidgetSize(kind, size));
}

export async function setWeekStartsMonday(monday: boolean): Promise<void> {
  applySnapshot(await backend.setWeekStartsMonday(monday));
}

export async function setCalendarSettings(calendar: import("../types").CalendarSettings): Promise<void> { applySnapshot(await backend.setCalendarSettings(calendar)); }
export async function checkHolidayUpdates(): Promise<void> { applySnapshot(await backend.checkHolidayUpdates()); }

export async function setTheme(theme: ThemeMode): Promise<void> {
  applySnapshot(await backend.setTheme(theme));
}

export async function setAccentColor(color: string): Promise<void> {
  applySnapshot(await backend.setAccentColor(color));
}

export async function setWidgetAppearance(
  widgetTransparency: number,
  widgetCornerRadius: number,
): Promise<void> {
  applySnapshot(await backend.setWidgetAppearance(widgetTransparency, widgetCornerRadius));
}

export async function createTodo(title: string, dueDate: string | null): Promise<void> {
  applySnapshot(await backend.createTodo(title, dueDate));
}

export async function updateTodo(id: number, title: string, dueDate: string | null): Promise<void> {
  applySnapshot(await backend.updateTodo(id, title, dueDate));
}

export async function setTodoCompleted(id: number, completed: boolean): Promise<void> {
  applySnapshot(await backend.setTodoCompleted(id, completed));
}

export async function deleteTodo(id: number): Promise<void> {
  applySnapshot(await backend.deleteTodo(id));
}

export async function saveWidgetPosition(kind: WidgetKind, position: { x: number; y: number }): Promise<void> {
  await backend.saveWidgetPosition(kind, position);
}

export async function setClockSettings(clock: import("../types").ClockSettings): Promise<void> { applySnapshot(await backend.setClockSettings(clock)); }
export async function saveNote(id: number, text: string): Promise<void> { applySnapshot(await backend.saveNote(id, text)); }
export async function createNote(): Promise<void> { applySnapshot(await backend.createNote()); }
export async function selectNote(id: number): Promise<void> { applySnapshot(await backend.selectNote(id)); }
export async function deleteNote(id: number): Promise<void> { applySnapshot(await backend.deleteNote(id)); }
export async function restoreNote(item: import("../types").NoteItem): Promise<void> { applySnapshot(await backend.restoreNote(item)); }
export async function setNoteExpiry(id: number, hours: number | null): Promise<void> { applySnapshot(await backend.setNoteExpiry(id, hours)); }
export async function setNoteColor(color: string): Promise<void> { applySnapshot(await backend.setNoteColor(color)); }
export async function saveCountdown(item: Parameters<typeof backend.saveCountdown>[0]): Promise<void> { applySnapshot(await backend.saveCountdown(item)); }
export async function deleteCountdown(id: number): Promise<void> { applySnapshot(await backend.deleteCountdown(id)); }
