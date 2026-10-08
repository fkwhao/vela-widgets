import { onUnmounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { WidgetKind } from "../types";
import { isNativeApp } from "../../infrastructure/backend";
import { saveWidgetPosition } from "../../app/store";
import { useDesktopCanvas } from '../../features/desktop/context';

// Widget sizes are fixed presets, so only the dragged position is persisted.
export function useWindowBounds(kind: WidgetKind): void {
  if (useDesktopCanvas() || !isNativeApp()) return;

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

  let disposed = false;
  let unlisten: (() => void) | undefined;
  void current.onMoved(save).then((stop) => { if (disposed) stop(); else unlisten = stop; }).catch(() => undefined);
  onUnmounted(() => { disposed = true; unlisten?.(); if (timer) clearTimeout(timer); });
}
