import bundledHolidays from "../data/holidays/china.json";
export type WidgetKind = "calendar" | "todo" | "clock" | "note" | "countdown";
export type ThemeMode = "light" | "dark" | "system";
export type WidgetSize = "small" | "medium" | "large";

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

export interface ClockCity { name: string; timeZone: string }
export interface CalendarSettings { showHolidays: boolean; showWorkdays: boolean; autoUpdate: boolean }
export interface HolidayDay { date: string; name: string; type: "holiday" | "workday" }
export interface HolidayYear { year: number; source: string; days: HolidayDay[] }
export interface HolidayData { schemaVersion: number; revision: number; updatedAt: string; years: HolidayYear[] }
export interface HolidayCache { data: HolidayData; lastAttemptAt: number | null; lastCheckedAt: number | null; lastUpdatedAt: number | null; lastError: string | null }
export interface ClockSettings { hour12: boolean; showSeconds: boolean; cities: ClockCity[] }
export interface NoteItem { id: number; text: string; color: string; createdAt: number; deleteAfterHours: number | null }
export interface NoteSettings { text: string; color: string; notes: NoteItem[]; activeId: number | null }
export interface CountdownItem { id: number; title: string; date: string; yearly: boolean; countUp: boolean; createdDate: string }

export const widgetRegistry: Record<WidgetKind, { label: string; icon: string; description: string; defaultSize: WidgetSize; sizes: WidgetSize[] }> = {
  calendar: { label: "日历", icon: "calendar", description: "在桌面上查看日期与月历", defaultSize: "large", sizes: ["small", "medium", "large"] },
  todo: { label: "待办", icon: "check", description: "记录要做的事，完成后随手勾选", defaultSize: "medium", sizes: ["small", "medium", "large"] },
  clock: { label: "时钟", icon: "clock", description: "此刻的时间，以及远方城市的昼夜", defaultSize: "small", sizes: ["small", "medium", "large"] },
  note: { label: "便签", icon: "note", description: "随手写下想法，自动保存在本机", defaultSize: "medium", sizes: ["small", "medium", "large"] },
  countdown: { label: "倒数日", icon: "hourglass", description: "记住值得期待与纪念的日子", defaultSize: "small", sizes: ["small", "medium", "large"] },
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
  note: NoteSettings;
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
  holidays: HolidayCache;
}

export const defaultSnapshot: AppSnapshot = {
  settings: {
    theme: "light",
    accentColor: "#3b67b8",
    widgetTransparency: 12,
    widgetCornerRadius: 19,
    weekStartsMonday: true,
    calendar: { showHolidays: true, showWorkdays: true, autoUpdate: false },
    clock: { hour12: false, showSeconds: false, cities: [] },
    note: { text: "", color: "#3b67b8", notes: [{ id: 1, text: "", color: "#3b67b8", createdAt: 0, deleteAfterHours: null }], activeId: 1 },
    widgets: {
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
  holidays: { data: bundledHolidays as HolidayData, lastAttemptAt: null, lastCheckedAt: null, lastUpdatedAt: null, lastError: null },
};
