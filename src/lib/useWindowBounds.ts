import { getCurrentWindow } from "@tauri-apps/api/window";
import type { WidgetKind } from "../types";
import { isNativeApp } from "./backend";
import { saveWidgetPosition } from "./store";

// Widget sizes are fixed presets, so only the dragged position is persisted.
export function useWindowBounds(kind: WidgetKind): void {
  if (!isNativeApp()) return;

  let timer: ReturnType<typeof setTimeout> | undefined;
  const current = getCurrentWindow();
  const save = async () => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(async () => {
      try {
        const [position, scale] = await Promise.all([current.outerPosition(), current.scaleFactor()]);
        await saveWidgetPosition(kind, { x: position.x / scale, y: position.y / scale });
      } catch {
        // Window state saving is best-effort while a widget is being closed.
      }
    }, 450);
  };

  void current.onMoved(save);
}
