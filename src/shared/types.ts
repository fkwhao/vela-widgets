import bundledHolidays from "../../data/holidays/china.json";
export type WidgetKind = "calendar" | "todo" | "clock" | "note" | "countdown" | "habit" | "media";
export type ThemeMode = "light" | "dark" | "system";
export type WidgetSize = "small" | "medium" | "large";
export type MediaTheme = "default" | "vinyl" | "atmosphere" | "cream" | "cassette" | "minimal";
export interface MediaSettings { theme: MediaTheme }

export const widgetSizeOptions: { value: WidgetSize; label: string }[] = [
  { value: "small", label: "小" },
  { value: "medium", label: "中" },
  { value: "large", label: "大" },
];

export interface WidgetSettings {
  enabled: boolean;
  alwaysOnTop: boolean;
  locked: boolean;
  x: number | null;
  y: number | null;
  width: number;
  height: number;
  size: WidgetSize;
}

export type ClockMode = "clock" | "alarm" | "stopwatch" | "timer";
export interface Alarm { id: number; label: string; time: string; date: string | null; weekdays: number[]; enabled: boolean; nextAt: number | null; snoozeAt: number | null }
export interface AlarmInput { id: number | null; label: string; time: string; date: string | null; weekdays: number[] }
export interface ClockAlert { id: number; alarmId: number | null; label: string; firedAt: number; missed: boolean }
export interface ClockTools { mode: ClockMode; alarms: Alarm[]; timer: { durationSeconds: number; remainingMs: number; deadline: number | null }; stopwatch: { elapsedMs: number; startedAt: number | null; laps: number[] }; alerts: ClockAlert[] }
export interface ClockAction { action: string; id?: number; seconds?: number; mode?: ClockMode; alarm?: AlarmInput }
export const clockModes: { value: ClockMode; label: string; icon: string }[] = [{ value: "clock", label: "时钟", icon: "clock" }, { value: "alarm", label: "闹钟", icon: "alarm" }, { value: "stopwatch", label: "秒表", icon: "stopwatch" }, { value: "timer", label: "计时器", icon: "hourglass" }];
export const defaultClockTools: ClockTools = { mode: "clock", alarms: [], timer: { durationSeconds: 300, remainingMs: 300000, deadline: null }, stopwatch: { elapsedMs: 0, startedAt: null, laps: [] }, alerts: [] };
export interface ClockCity { name: string; timeZone: string }
export type CalendarStyle = "month" | "agenda" | "list";
export interface CalendarEvent { id: number; title: string; date: string; endDate: string; allDay: boolean; startTime: string; endTime: string; location: string; color: string }
export interface CalendarSettings { showHolidays: boolean; showWorkdays: boolean; autoUpdate: boolean; style: CalendarStyle; events: CalendarEvent[] }
export const calendarStyles: { value: CalendarStyle; label: string }[] = [{ value: "month", label: "日期 / 月历" }, { value: "agenda", label: "日程" }, { value: "list", label: "列表" }];
export interface HolidayDay { date: string; name: string; type: "holiday" | "workday" }
export interface HolidayYear { year: number; source: string; days: HolidayDay[] }
export interface HolidayData { schemaVersion: number; revision: number; updatedAt: string; years: HolidayYear[] }
export interface HolidayCache { data: HolidayData; lastAttemptAt: number | null; lastCheckedAt: number | null; lastUpdatedAt: number | null; lastError: string | null }
export type ClockTheme = "default" | "classic" | "digital" | "world";
export const clockThemes: { value: ClockTheme; label: string }[] = [{ value: "default", label: "默认主题" }, { value: "digital", label: "数字时钟" }, { value: "classic", label: "经典表盘" }, { value: "world", label: "世界时钟" }];
export interface ClockSettings { theme: ClockTheme; hour12: boolean; showSeconds: boolean; cities: ClockCity[] }
export interface NoteItem { id: number; text: string; color: string; createdAt: number; deleteAfterHours: number | null; retentionOverride: boolean }
export interface NoteSettings { text: string; color: string; notes: NoteItem[]; activeId: number | null; defaultDeleteAfterHours: number | null }
export interface CountdownItem { id: number; title: string; date: string; yearly: boolean; countUp: boolean; createdDate: string }

export type HabitStyle = "card" | "list" | "report";
export interface HabitSettings { style: HabitStyle; selectedIds: number[] | null }
export interface HabitItem { id: number; title: string; encouragement: string; icon: string; color: string; startDate: string; weekdays: number[]; dailyTarget: number; goalDays: number | null; archived: boolean; sortOrder: number; trackMood: boolean; trackRating: boolean; trackResult: boolean; reminderTime: string | null; lastRemindedDate: string | null }
export interface HabitRecord { habitId: number; date: string; count: number; target: number; updatedAt: string; mood: string | null; rating: number | null; result: number | null }
export interface HabitRecordInput { habitId: number; date: string; delta: number; captureMetadata?: boolean; mood?: string | null; rating?: number | null; result?: number | null }

export const widgetRegistry: Record<WidgetKind, { label: string; icon: string; description: string; defaultSize: WidgetSize; sizes: WidgetSize[] }> = {
  habit: { label: "习惯打卡", icon: "check", description: "坚持每日习惯，记录进度与成长", defaultSize: "small", sizes: ["small", "medium", "large"] },
  calendar: { label: "日历", icon: "calendar", description: "查看日期、月历与近期日程", defaultSize: "large", sizes: ["small", "medium", "large"] },
  todo: { label: "待办", icon: "check", description: "记录要做的事，完成后随手勾选", defaultSize: "medium", sizes: ["small", "medium", "large"] },
  clock: { label: "时钟", icon: "clock", description: "查看时间，管理闹钟、秒表和计时器", defaultSize: "small", sizes: ["small", "medium", "large"] },
  note: { label: "便签", icon: "note", description: "随手写下想法，自动保存在本机", defaultSize: "medium", sizes: ["small", "medium", "large"] },
  countdown: { label: "倒数日", icon: "hourglass", description: "记住值得期待与纪念的日子", defaultSize: "small", sizes: ["small", "medium", "large"] },
  media: { label: "正在播放", icon: "music", description: "查看当前媒体，随手暂停或切换曲目", defaultSize: "medium", sizes: ["small", "medium", "large"] },
};
export const widgetKinds = Object.keys(widgetRegistry) as WidgetKind[];
export function isWidgetKind(value: unknown): value is WidgetKind { return typeof value === "string" && Object.prototype.hasOwnProperty.call(widgetRegistry, value); }

export interface Settings {
  theme: ThemeMode;
  accentColor: string;
  widgetTransparency: number;
  widgetCornerRadius: number;
  weekStartsMonday: boolean;
  calendar: CalendarSettings;
  clock: ClockSettings;
  clockTools: ClockTools;
  note: NoteSettings;
  habit: HabitSettings;
  media: MediaSettings;
  widgets: Record<WidgetKind, WidgetSettings>;
}

export interface TodoItem {
  id: number;
  title: string;
  dueDate: string | null;
  completed: boolean;
  createdAt: string;
}

export interface AppSnapshot {
  settings: Settings;
  todos: TodoItem[];
  countdowns: CountdownItem[];
  habits: HabitItem[];
  habitRecords: HabitRecord[];
  holidays: HolidayCache;
}

export const defaultSnapshot: AppSnapshot = {
  settings: {
    theme: "light",
    accentColor: "#3b67b8",
    widgetTransparency: 12,
    widgetCornerRadius: 19,
    weekStartsMonday: true,
    calendar: { showHolidays: true, showWorkdays: true, autoUpdate: false, style: "month", events: [] },
    clock: { theme: "default", hour12: false, showSeconds: false, cities: [] },
    clockTools: structuredClone(defaultClockTools),
    note: { text: "", color: "#3b67b8", notes: [{ id: 1, text: "", color: "#3b67b8", createdAt: 0, deleteAfterHours: null, retentionOverride: false }], activeId: 1, defaultDeleteAfterHours: null },
    habit: { style: "card", selectedIds: null },
    media: { theme: "default" },
    widgets: {
      media: { enabled: false, alwaysOnTop: false, locked: false, x: null, y: null, width: 364, height: 170, size: "medium" },
      habit: { enabled: false, alwaysOnTop: false, locked: false, x: null, y: null, width: 170, height: 170, size: "small" },
      clock: { enabled: false, alwaysOnTop: false, locked: false, x: null, y: null, width: 170, height: 170, size: "small" },
      note: { enabled: false, alwaysOnTop: false, locked: false, x: null, y: null, width: 364, height: 170, size: "medium" },
      countdown: { enabled: false, alwaysOnTop: false, locked: false, x: null, y: null, width: 170, height: 170, size: "small" },
      calendar: {
        enabled: true,
        alwaysOnTop: false,
        locked: false,
        x: null,
        y: null,
        width: 364,
        height: 384,
        size: "large",
      },
      todo: {
        enabled: false,
        alwaysOnTop: false,
        locked: false,
        x: null,
        y: null,
        width: 364,
        height: 170,
        size: "medium",
      },
    },
  },
  todos: [],
  countdowns: [],
  habits: [],
  habitRecords: [],
  holidays: { data: bundledHolidays as HolidayData, lastAttemptAt: null, lastCheckedAt: null, lastUpdatedAt: null, lastError: null },
};
