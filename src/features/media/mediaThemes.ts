import type { MediaTheme, WidgetSize } from "../../shared/types";

export const mediaThemes: { value: MediaTheme; label: string; description: string }[] = [
  { value: "default", label: "经典", description: "清晰封面 · 跟随系统外观" },
  { value: "atmosphere", label: "封面卡片", description: "竖向封面 · 专辑取色" },
  { value: "vinyl", label: "流彩黑胶", description: "半露唱片 · 专辑取色" },
  { value: "minimal", label: "极简横条", description: "纯黑底色 · 紧凑控制" },
];

export function normalizeMediaTheme(value: unknown): MediaTheme {
  return mediaThemes.find(theme => theme.value === value)?.value ?? "default";
}

export function mediaWidgetDimensions(size: WidgetSize, theme: MediaTheme) {
  if (size === "medium" && theme === "atmosphere") return { width: 224, height: 356 };
  return { width: size === "small" ? 170 : 364, height: size === "large" ? 384 : 170 };
}
