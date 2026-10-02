export type WidgetKind = "calendar" | "todo";
export type ThemeMode = "light" | "dark" | "system";

export interface WidgetSettings {
  enabled: boolean;
  alwaysOnTop: boolean;
  locked: boolean;
  x: number | null;
  y: number | null;
  width: number;
  height: number;
}

export interface Settings {
  theme: ThemeMode;
  accentColor: string;
  widgetTransparency: number;
  widgetCornerRadius: number;
  weekStartsMonday: boolean;
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
}

export const defaultSnapshot: AppSnapshot = {
  settings: {
    theme: "light",
    accentColor: "#3b67b8",
    widgetTransparency: 12,
    widgetCornerRadius: 19,
    weekStartsMonday: true,
    widgets: {
      calendar: {
        enabled: true,
        alwaysOnTop: false,
        locked: false,
        x: null,
        y: null,
        width: 332,
        height: 450,
      },
      todo: {
        enabled: false,
        alwaysOnTop: false,
        locked: false,
        x: null,
        y: null,
        width: 350,
        height: 430,
      },
    },
  },
  todos: [],
};
