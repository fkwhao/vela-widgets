import type { NoteItem, NoteSettings } from "../types";
export function normalizeNotes(value: Partial<NoteSettings>, now = Date.now()): NoteSettings {
  const color = value.color ?? "#3b67b8";
  const initial = value.activeId == null ? [{ id: 1, text: value.text ?? "", color, createdAt: now, deleteAfterHours: null }] : (value.notes ?? []);
  const notes = initial.map(n => ({ ...n, createdAt: n.createdAt || now, deleteAfterHours: n.deleteAfterHours ?? null })).filter(n => n.deleteAfterHours === null || now < n.createdAt + n.deleteAfterHours * 3600000);
  const active = notes.find(n => n.id === value.activeId) ?? notes[0];
  return { notes, activeId: active?.id ?? 0, text: active?.text ?? "", color: active?.color ?? color };
}
export function nextNoteId(notes: NoteItem[]) { return Math.max(Date.now(), ...notes.map(n => n.id + 1)); }
