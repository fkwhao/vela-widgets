import type { CalendarSettings, HolidayCache, HolidayData, HolidayDay } from "../types";

export const holidayUpdateBase = "https://raw.githubusercontent.com/fkwhao/vela-widgets/main/data/holidays";
export const holidayCheckInterval = 86_400_000;

function validDate(date: unknown): date is string {
  if (typeof date !== "string" || !/^\d{4}-\d{2}-\d{2}$/.test(date)) return false;
  const parsed = new Date(`${date}T12:00:00Z`);
  return Number.isFinite(parsed.getTime()) && parsed.toISOString().slice(0, 10) === date && Number(date.slice(0, 4)) >= 2000;
}
export function validateHolidayData(value: unknown): HolidayData {
  const data = value as HolidayData;
  if (!data || data.schemaVersion !== 1 || !Number.isSafeInteger(data.revision) || data.revision <= 0 || !validDate(data.updatedAt) || !Array.isArray(data.years) || !data.years.length || data.years.length > 100) throw new Error("节假日数据格式不受支持，已保留原有数据。");
  const years = new Set<number>();
  for (const year of data.years) {
    let source: URL;
    try { source = new URL(year.source); } catch { throw new Error("节假日官方来源无效。"); }
    if (!Number.isInteger(year.year) || year.year < 2000 || year.year > 2199 || years.has(year.year) || source.protocol !== "https:" || !(source.hostname === "gov.cn" || source.hostname.endsWith(".gov.cn")) || !Array.isArray(year.days) || !year.days.length || year.days.length > 366) throw new Error("节假日年份或官方来源无效，已保留原有数据。");
    years.add(year.year);
    const dates = new Set<string>();
    for (const day of year.days) {
      if (!validDate(day.date) || Number(day.date.slice(0, 4)) !== year.year || dates.has(day.date) || !["holiday", "workday"].includes(day.type) || typeof day.name !== "string" || !day.name.trim() || Array.from(day.name).length > 24 || /[\u0000-\u001f\u007f-\u009f]/.test(day.name)) throw new Error("节假日日期重复或无效，已保留原有数据。");
      dates.add(day.date);
    }
  }
  return data;
}
export function mergeHolidayData(current: HolidayData, incoming: HolidayData): HolidayData {
  return { ...incoming, years: [...incoming.years, ...current.years.filter(old => !incoming.years.some(year => year.year === old.year))].sort((a,b) => a.year - b.year) };
}
export function normalizeHolidayCache(value: HolidayCache | undefined, bundled: HolidayData): HolidayCache {
  const initial: HolidayCache = { data: bundled, lastAttemptAt: null, lastCheckedAt: null, lastUpdatedAt: null, lastError: null };
  try {
    if (!value) return initial;
    const data = validateHolidayData(value.data);
    return { ...initial, ...value, data: data.revision < bundled.revision ? mergeHolidayData(data, bundled) : data };
  } catch { return initial; }
}
export function visibleHoliday(day: HolidayDay | undefined, settings: CalendarSettings): HolidayDay | undefined {
  return day && (day.type === "holiday" ? settings.showHolidays : settings.showWorkdays) ? day : undefined;
}
// Official adjustments take priority over the usual Saturday/Sunday rest days.
export function isCalendarRestDay(date: Date, day?: HolidayDay): boolean {
  if (day) return day.type === "holiday";
  return date.getDay() === 0 || date.getDay() === 6;
}
export function holidayLabel(day: HolidayDay): string { return `${day.name} · ${day.type === "holiday" ? "休息" : "调休上班"}`; }
export function holidayUpdateDue(lastAttemptAt: number | null, now = Date.now()): boolean {
  return lastAttemptAt === null || now < lastAttemptAt || now - lastAttemptAt >= holidayCheckInterval;
}
async function fetchJson(path: string, fetcher: typeof fetch): Promise<unknown> {
  const response = await fetcher(`${holidayUpdateBase}/${path}`, { cache: "no-cache", credentials: "omit", referrerPolicy: "no-referrer", signal: AbortSignal.timeout(15000) });
  if (response.status === 404) throw new Error("节假日更新源尚未发布，本地数据仍可使用。");
  if (!response.ok) throw new Error("节假日更新源暂时不可用，本地数据仍可使用。");
  const reader = response.body?.getReader();
  if (!reader) throw new Error("下载没有完成，已保留原有数据。");
  const chunks: Uint8Array[] = []; let length = 0;
  try {
    while (true) { const {value, done} = await reader.read(); if (done) break; length += value.length; if (length > 1_048_576) throw new Error("节假日数据过大，已保留原有数据。"); chunks.push(value); }
  } finally { await reader.cancel(); }
  const bytes = new Uint8Array(length); let offset = 0;
  for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
  try { return JSON.parse(new TextDecoder().decode(bytes)); } catch { throw new Error("节假日数据格式无效，已保留原有数据。"); }
}
export async function fetchHolidayUpdate(current: HolidayData, fetcher: typeof fetch = fetch): Promise<HolidayData | null> {
  const manifest = await fetchJson("manifest.json", fetcher) as { schemaVersion: number; revision: number };
  if (!manifest || manifest.schemaVersion !== 1 || !Number.isSafeInteger(manifest.revision) || manifest.revision <= 0) throw new Error("节假日更新版本无效，已保留原有数据。");
  if (manifest.revision < current.revision) throw new Error("更新源版本较旧，已保留原有数据。");
  if (manifest.revision === current.revision) return null;
  const next = validateHolidayData(await fetchJson("china.json", fetcher));
  if (next.revision !== manifest.revision) throw new Error("更新文件版本尚未同步，请稍后重试。");
  return next;
}
