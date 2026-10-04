import type { HabitItem, HabitRecord, HabitSettings, ClockTools } from "../types";

export const habitColors = ["#444fb0", "#ff743b", "#858ee8", "#398d78", "#b45485", "#b38330", "#448cb2", "#7660ac"];
export const habitIcons = [
  { label: "成长与成就", icons: ["💎", "🏅", "🏆", "🚀", "⭐", "🎯", "💡", "🌱", "👑", "🔥", "🧠", "💻"] },
  { label: "健康与生活", icons: ["💧", "🌙", "🛌", "⏰", "🥛", "🍎", "🥦", "☕", "💊", "🦷", "🧹", "☀️"] },
  { label: "学习与运动", icons: ["📖", "✍️", "🎨", "🎹", "🎧", "🗣️", "🏃", "🚴", "🏊", "🧘", "🏋️", "⚽"] },
];
export const encouragements = ["每天一点进步，坚持会给你答案。", "把今天做好，就是向目标靠近。", "慢慢来，持续比完美更重要。", "为自己留一点成长的时间。"];
export function dateKey(date = new Date()): string { return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`; }
export function shiftDate(key: string, days: number): string { const d = new Date(`${key}T12:00:00`); d.setDate(d.getDate() + days); return dateKey(d); }
export function validDate(key: string): boolean { return /^\d{4}-\d{2}-\d{2}$/.test(key) && dateKey(new Date(`${key}T12:00:00`)) === key; }
export function scheduled(habit: HabitItem, key: string): boolean { return key >= habit.startDate && habit.weekdays.includes(new Date(`${key}T12:00:00`).getDay()); }
export function recordFor(records: HabitRecord[], id: number, key: string): HabitRecord | undefined { return records.find(r => r.habitId === id && r.date === key); }
export function isComplete(record?: HabitRecord): boolean { return !!record && record.count >= record.target; }
export function selectedHabits(habits: HabitItem[], settings: HabitSettings): HabitItem[] { return habits.filter(h => !h.archived && (settings.selectedIds === null || settings.selectedIds.includes(h.id))).sort((a, b) => a.sortOrder - b.sortOrder || a.id - b.id); }
export function todayHabits(habits: HabitItem[], settings: HabitSettings, key: string): HabitItem[] { return selectedHabits(habits, settings).filter(h => scheduled(h, key)); }
export function nextIncomplete(habits: HabitItem[], records: HabitRecord[], key: string): HabitItem | undefined { return habits.find(h => !isComplete(recordFor(records, h.id, key))); }
export function weekDates(key: string, monday: boolean): string[] { const day = new Date(`${key}T12:00:00`).getDay(); const start = shiftDate(key, -(monday ? (day + 6) % 7 : day)); return Array.from({ length: 7 }, (_, i) => shiftDate(start, i)); }
export function monthDates(key: string): string[] { const d = new Date(`${key}T12:00:00`); const count = new Date(d.getFullYear(), d.getMonth() + 1, 0).getDate(); return Array.from({ length: count }, (_, i) => `${key.slice(0, 7)}-${String(i + 1).padStart(2, "0")}`); }
export function annualWeeklyCheckins(records: HabitRecord[], year: number, monday: boolean, today = dateKey()) {
  const first = weekDates(`${year}-01-01`, monday)[0]!;
  const rows: { start: string; index: number; count: number }[] = [];
  for (let start = first, index = 1; start <= `${year}-12-31` && start <= today; start = shiftDate(start, 7), index++) {
    const end = shiftDate(start, 6);
    rows.push({ start, index, count: records.filter(r => r.date >= start && r.date <= end && r.date <= today && r.date.startsWith(String(year))).reduce((sum, r) => sum + r.count, 0) });
  }
  return rows;
}
export function habitStats(habit: HabitItem, records: HabitRecord[], today: string) {
  const rows = records.filter(r => r.habitId === habit.id && r.date <= today);
  const done = rows.filter(isComplete).sort((a, b) => a.date.localeCompare(b.date));
  const completed = new Set(done.map(r => r.date));
  // Rest days do not break a scheduled streak. Historical records retain their target.
  let current = 0, longest = 0, run = 0;
  if (habit.startDate <= today) {
    for (let day = habit.startDate; day <= today; day = shiftDate(day, 1)) {
      if (completed.has(day)) { run++; longest = Math.max(longest, run); }
      else if (scheduled(habit, day) && day !== today) run = 0;
    }
    current = run;
  }
  return { total: done.length, month: done.filter(r => r.date.startsWith(today.slice(0, 7))).length, current, longest, checkins: rows.reduce((sum, r) => sum + r.count, 0) };
}
export function validateHabit(item: HabitItem): string | null {
  if (!item.title.trim() || item.title.trim().length > 80) return "习惯名称需为 1–80 个字符。";
  if (item.encouragement.length > 160 || !item.icon.trim() || item.icon.length > 16 || !/^#[0-9a-f]{6}$/i.test(item.color)) return "请检查鼓励语、图标和颜色。";
  if (!validDate(item.startDate) || item.startDate < "1900-01-01" || item.startDate > "2100-12-31") return "请选择 1900–2100 年之间的有效开始日期。";
  if (!item.weekdays.length || item.weekdays.some(d => !Number.isInteger(d) || d < 0 || d > 6)) return "至少选择一个执行日。";
  if (!Number.isInteger(item.dailyTarget) || item.dailyTarget < 1 || item.dailyTarget > 99) return "每日目标需为 1–99 次。";
  if (item.goalDays !== null && (!Number.isInteger(item.goalDays) || item.goalDays < 1 || item.goalDays > 10000)) return "坚持目标需为 1–10000 天。";
  if (item.reminderTime && !/^([01]\d|2[0-3]):[0-5]\d$/.test(item.reminderTime)) return "请输入有效提醒时间。";
  return null;
}
export function parseHabitResult(value: string | number): number | null {
  if (typeof value === "string" && !value.trim()) return null;
  const result = Number(value);
  if (!Number.isFinite(result) || Math.abs(result) > 1e9) throw new Error("请输入有效成绩。");
  return result;
}
export function validateRecord(habit: HabitItem | undefined, input: { date: string; delta: number; mood?: string | null; rating?: number | null; result?: number | null }, today = dateKey(), existing = false): string | null {
  if (!habit) return "找不到这个习惯。";
  if (!validDate(input.date) || input.date > today || (!existing && !scheduled(habit, input.date))) return "只能记录开始日期之后的执行日，不能提前打卡。";
  if (![0, -1, 1].includes(input.delta)) return "无效的打卡操作。";
  if (input.mood != null && !["开心", "平静", "疲惫", "低落"].includes(input.mood)) return "请选择有效的心情。";
  if (input.rating != null && (!Number.isInteger(input.rating) || input.rating < 1 || input.rating > 5)) return "评分需为 1–5。";
  if (input.result != null && (!Number.isFinite(input.result) || Math.abs(input.result) > 1e9)) return "请输入有效成绩。";
  return null;
}
export function cardForeground(color: string): string {
  const c = [1, 3, 5].map(i => parseInt(color.slice(i, i + 2), 16) / 255).map(v => v <= .04045 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4);
  const l = .2126 * c[0] + .7152 * c[1] + .0722 * c[2]; return 1.05 / (l + .05) >= (l + .05) / .05857 ? "#fff" : "#171717";
}
export function tickHabitReminders(habits: HabitItem[], records: HabitRecord[], tools: ClockTools, now = new Date()): boolean {
  const day = dateKey(now), time = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;
  let changed = false;
  for (const h of habits) {
    if (!h.reminderTime || h.archived || h.reminderTime !== time || h.lastRemindedDate === day || !scheduled(h, day) || isComplete(recordFor(records, h.id, day))) continue;
    h.lastRemindedDate = day;
    tools.alerts.push({ id: Math.max(now.getTime(), ...tools.alerts.map(a => a.id + 1)), alarmId: null, label: `习惯提醒 · ${h.title}`, firedAt: now.getTime(), missed: false }); changed = true;
  }
  if (tools.alerts.length > 32) tools.alerts.splice(0, tools.alerts.length - 32);
  return changed;
}
