import { getCurrentWindow } from "@tauri-apps/api/window";
import type { WidgetKind } from "../types";
import { isNativeApp } from "./backend";
import { saveWidgetBounds } from "./store";

export function useWindowBounds(kind: WidgetKind): void {
  if (!isNativeApp()) return;

  let timer: ReturnType<typeof setTimeout> | undefined;
  const current = getCurrentWindow();
  const save = async () => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(async () => {
      try {
        const [position, size, scale] = await Promise.all([
          current.outerPosition(),
          current.innerSize(),
          current.scaleFactor(),
        ]);
        await saveWidgetBounds(kind, {
          x: position.x / scale,
          y: position.y / scale,
          width: size.width / scale,
          height: size.height / scale,
        });
      } catch {
        // Window state saving is best-effort while a widget is being closed.
      }
    }, 450);
  };

  void current.onMoved(save);
  void current.onResized(save);
}
