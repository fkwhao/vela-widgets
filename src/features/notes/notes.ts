import type { NoteItem, NoteSettings } from "../../shared/types";
export function noteRetentionHours(note: Pick<NoteItem, "retentionOverride" | "deleteAfterHours">, defaultHours: number | null): number | null {
  return note.retentionOverride || note.deleteAfterHours != null ? note.deleteAfterHours : defaultHours;
}
export function normalizeNotes(value: Partial<NoteSettings>, now = Date.now()): NoteSettings {
  const color = value.color ?? "#3b67b8";
  const defaultDeleteAfterHours = value.defaultDeleteAfterHours ?? null;
  const initial = value.activeId == null ? [{ id: 1, text: value.text ?? "", color, createdAt: now, deleteAfterHours: null, retentionOverride: false }] : (value.notes ?? []);
  const notes = initial.map(n => ({ ...n, createdAt: n.createdAt || now, deleteAfterHours: n.deleteAfterHours ?? null, retentionOverride: n.retentionOverride === true || n.deleteAfterHours != null })).filter(n => {
    const hours = noteRetentionHours(n, defaultDeleteAfterHours);
    return hours === null || now < n.createdAt + hours * 3600000;
  });
  const active = notes.find(n => n.id === value.activeId) ?? notes[0];
  return { notes, defaultDeleteAfterHours, activeId: active?.id ?? 0, text: active?.text ?? "", color: active?.color ?? color };
}
export function nextNoteId(notes: NoteItem[]) { return Math.max(Date.now(), ...notes.map(n => n.id + 1)); }
