import { defineAsyncComponent, type Component } from 'vue';
import type { WidgetKind } from '../types';

// Share lazy loaders between the canvas and standalone preview routes.
export const widgetComponents: Record<WidgetKind, Component> = {
  calendar: defineAsyncComponent(() => import('../../features/calendar/components/CalendarWidget.vue')),
  todo: defineAsyncComponent(() => import('../../features/todos/components/TodoWidget.vue')),
  clock: defineAsyncComponent(() => import('../../features/clock/components/ClockWidget.vue')),
  note: defineAsyncComponent(() => import('../../features/notes/components/NoteWidget.vue')),
  countdown: defineAsyncComponent(() => import('../../features/countdown/components/CountdownWidget.vue')),
  habit: defineAsyncComponent(() => import('../../features/habits/components/HabitWidget.vue')),
  media: defineAsyncComponent(() => import('../../features/media/components/MediaWidget.vue')),
};
