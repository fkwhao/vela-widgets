import type { TodoItem, WidgetSize } from "../types";
export function paginateTodos(items: TodoItem[], size: WidgetSize, editingId: number | null = null, reservedHeight = 0): TodoItem[][] {
  if (!items.length) return [[]];
  // Compact rows have fixed heights; large rows budget for dates and the inline editor.
  const budget = Math.max(24, (size === "small" ? 72 : size === "medium" ? 114 : 245) - reservedHeight);
  const pages: TodoItem[][] = [];
  let page: TodoItem[] = [], height = 0;
  for (const item of items) {
    const row = size === "small" ? 24 : size === "medium" ? 28 : item.id === editingId ? 96 : item.dueDate ? 61 : 45;
    if (page.length && height + row > budget) { pages.push(page); page = []; height = 0; }
    page.push(item); height += row;
  }
  pages.push(page);
  return pages;
}
