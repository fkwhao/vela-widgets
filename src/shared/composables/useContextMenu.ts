import { nextTick, ref } from 'vue';

export function useContextMenu() {
  const menu = ref<{ x: number; y: number } | null>(null);
  const menuElement = ref<HTMLElement | null>(null);
  async function placeMenu(event: MouseEvent) {
    const point = { x: event.clientX, y: event.clientY };
    menu.value = point;
    const current = menu.value;
    await nextTick();
    if (menu.value !== current || !menuElement.value) return;
    const bounds = menuElement.value.getBoundingClientRect();
    menu.value = {
      x: Math.max(4, Math.min(point.x, window.innerWidth - bounds.width - 4)),
      y: Math.max(4, Math.min(point.y, window.innerHeight - bounds.height - 4)),
    };
  }
  return { menu, menuElement, placeMenu };
}
