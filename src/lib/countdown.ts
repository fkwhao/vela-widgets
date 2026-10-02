import type { CountdownItem } from "../types";
export function localDateKey(date: Date): string { return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`; }
function dayStamp(date: Date) { return Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()); }
function occurrence(value: string, year: number) {
  const [, month, day] = value.split("-").map(Number);
  // A Feb 29 anniversary falls on Feb 28 in non-leap years.
  return new Date(year, month - 1, Math.min(day, new Date(year, month, 0).getDate()));
}
export function countdownInfo(item: CountdownItem, now: Date) {
  let target = new Date(`${item.date}T12:00:00`);
  const today = dayStamp(now);
  if (item.yearly) {
    target = occurrence(item.date, now.getFullYear());
    if (!item.countUp && dayStamp(target) < today) target = occurrence(item.date, now.getFullYear() + 1);
    if (item.countUp && dayStamp(target) > today) target = occurrence(item.date, now.getFullYear() - 1);
  }
  const signed = Math.round((dayStamp(target) - today) / 86400000);
  const days = Math.abs(signed);
  const caption = signed === 0 ? "就是今天" : signed > 0 ? "还有" : "已经过去";
  let start = new Date(`${item.createdDate}T12:00:00`);
  if (item.yearly) start = occurrence(item.date, target.getFullYear() + (item.countUp ? 1 : -1));
  const total = Math.abs(dayStamp(target) - dayStamp(start));
  const progress = total === 0 ? (signed <= 0 ? 100 : 0) : Math.max(0, Math.min(100, (item.countUp ? Math.abs(dayStamp(target) - today) / total : 1 - Math.abs(dayStamp(target) - today) / total) * 100));
  return { item, target, signed, days, caption, progress: item.countUp && !item.yearly ? null : progress, dateLabel: `${target.getFullYear()}.${String(target.getMonth()+1).padStart(2, "0")}.${String(target.getDate()).padStart(2, "0")}` };
}
export function sortedCountdowns(items: CountdownItem[], now: Date) { return items.map((item) => countdownInfo(item, now)).sort((a,b) => a.days - b.days || a.item.id - b.item.id); }
