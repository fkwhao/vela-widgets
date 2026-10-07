<script setup lang="ts">
withDefaults(defineProps<{ name: string; size?: number }>(), { size: 18 });
</script>

<template>
  <svg
    aria-hidden="true"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.7"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <template v-if="name === 'music'"><path d="M9 18V5l11-2v13M9 8l11-2" /><ellipse cx="6" cy="18" rx="3" ry="2.5" /><ellipse cx="17" cy="16" rx="3" ry="2.5" /></template>
    <template v-else-if="name === 'previous'"><path d="m19 5-11 7 11 7Z" fill="currentColor" stroke="none" /><path d="M5 5v14" stroke-width="2.5" /></template>
    <template v-else-if="name === 'next'"><path d="m5 5 11 7-11 7Z" fill="currentColor" stroke="none" /><path d="M19 5v14" stroke-width="2.5" /></template>
    <template v-else-if="name === 'play'"><path d="m8 4 12 8-12 8Z" fill="currentColor" stroke="none" /></template>
    <template v-else-if="name === 'pause'"><path d="M8 5v14M16 5v14" stroke-width="4" /></template>
    <template v-else-if="name === 'volume'"><path d="m11 5-6 4H2v6h3l6 4ZM15 8a6 6 0 0 1 0 8M18 5a10 10 0 0 1 0 14" /></template>
    <template v-else-if="name === 'volume-off'"><path d="m11 5-6 4H2v6h3l6 4ZM16 9l6 6M22 9l-6 6" /></template>
    <template v-else-if="name === 'return-today'"><path d="M4 8a8 8 0 1 1-1 8M4 3v5h5" /><circle cx="12" cy="12" r="2" /></template>
    <template v-else-if="name === 'arrow-up'"><path d="M12 21V3M3 12l9-9 9 9" /></template>
    <template v-else-if="name === 'arrow-down'"><path d="M12 3v18M3 12l9 9 9-9" /></template>
    <template v-else-if="name === 'alarm'"><circle cx="12" cy="13" r="7.5" /><path d="M12 9v4l3 2M4 3 1.5 6M20 3l2.5 3M6 20l-1 2M18 20l1 2" /></template>
    <template v-else-if="name === 'stopwatch'"><circle cx="12" cy="14" r="7.5" /><path d="M9 2h6M12 2v4M18 7l2-2M12 10v4l3-2" /></template>
    <template v-else-if="name === 'trash'"><path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 10v7M14 10v7" /></template>
    <template v-else-if="name === 'note'">
      <path d="M5 3.5h14a1.5 1.5 0 0 1 1.5 1.5v10L15 20.5H5A1.5 1.5 0 0 1 3.5 19V5A1.5 1.5 0 0 1 5 3.5Z" /><path d="M15 20.5V15h5.5M7.5 8h9M7.5 12h6" />
    </template>
    <template v-else-if="name === 'hourglass'">
      <path d="M6 3.5h12M6 20.5h12M7 3.5v4l5 4.5-5 4v4.5M17 3.5v4L12 12l5 4v4.5M9 18h6" />
    </template>
    <template v-else-if="name === 'grid'">
      <rect x="3.5" y="3.5" width="7" height="7" rx="2" />
      <rect x="13.5" y="3.5" width="7" height="7" rx="2" />
      <rect x="3.5" y="13.5" width="7" height="7" rx="2" />
      <rect x="13.5" y="13.5" width="7" height="7" rx="2" />
    </template>
    <template v-else-if="name === 'calendar'">
      <rect x="3.5" y="5" width="17" height="15.5" rx="3" />
      <path d="M7.5 3.5v3M16.5 3.5v3M3.5 9.5h17M8 13h.01M12 13h.01M16 13h.01M8 16.5h.01M12 16.5h.01" />
    </template>
    <template v-else-if="name === 'check'">
      <rect x="4" y="4" width="16" height="16" rx="4" />
      <path d="m8.5 12.5 2.3 2.3 4.8-5.2" />
    </template>
    <template v-else-if="name === 'tick'">
      <path d="m5.5 12.5 4.2 4.2 8.8-9.4" stroke-width="2.4" />
    </template>
    <template v-else-if="name === 'sun'">
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2.5v2M12 19.5v2M4.7 4.7l1.4 1.4m11.8 11.8 1.4 1.4M2.5 12h2m15 0h2M4.7 19.3l1.4-1.4M17.9 6.1l1.4-1.4" />
    </template>
    <template v-else-if="name === 'sliders'">
      <path d="M4 6h9m4 0h3M4 12h3m4 0h9M4 18h9m4 0h3" />
      <circle cx="15" cy="6" r="2" />
      <circle cx="9" cy="12" r="2" />
      <circle cx="15" cy="18" r="2" />
    </template>
    <template v-else-if="name === 'power'">
      <path d="M12 3v9" />
      <path d="M7.1 5.7a8 8 0 1 0 9.8 0" />
    </template>
    <template v-else-if="name === 'database'">
      <ellipse cx="12" cy="5" rx="8" ry="3" />
      <path d="M4 5v7c0 1.7 3.6 3 8 3s8-1.3 8-3V5M4 12v7c0 1.7 3.6 3 8 3s8-1.3 8-3v-7" />
    </template>
    <template v-else-if="name === 'info'">
      <circle cx="12" cy="12" r="9" />
      <path d="M12 11v5m0-8h.01" />
    </template>
    <template v-else-if="name === 'search'">
      <circle cx="10.8" cy="10.8" r="6.5" />
      <path d="m16 16 4.5 4.5" />
    </template>
    <template v-else-if="name === 'arrow-up-right'">
      <path d="M7 17 17 7M8 7h9v9" />
    </template>
    <template v-else-if="name === 'chevron-left'">
      <path d="m14.5 18-6-6 6-6" />
    </template>
    <template v-else-if="name === 'chevron-right'">
      <path d="m9.5 18 6-6-6-6" />
    </template>
    <template v-else-if="name === 'chevron-up'">
      <path d="m6.5 15 5.5-5.5 5.5 5.5" />
    </template>
    <template v-else-if="name === 'chevron-down'">
      <path d="m6.5 9 5.5 5.5L17.5 9" />
    </template>
    <template v-else-if="name === 'plus'">
      <path d="M12 5v14M5 12h14" />
    </template>
    <template v-else-if="name === 'close'">
      <path d="m6 6 12 12M18 6 6 18" />
    </template>
    <template v-else-if="name === 'more'">
      <circle cx="5" cy="12" r="1" fill="currentColor" />
      <circle cx="12" cy="12" r="1" fill="currentColor" />
      <circle cx="19" cy="12" r="1" fill="currentColor" />
    </template>
    <template v-else-if="name === 'edit'">
      <path d="m15 4 5 5M4 20l5-1 11-11a2.2 2.2 0 0 0-5-5L4 15v5Z" />
    </template>
    <template v-else-if="name === 'moon'">
      <path d="M20 14a8.5 8.5 0 0 1-10-10A8.5 8.5 0 1 0 20 14Z" />
    </template>
    <template v-else-if="name === 'clock'">
      <circle cx="12" cy="12" r="9" />
      <path d="M12 7v5l3.2 2" />
    </template>
    <template v-else-if="name === 'lock'">
      <rect x="4.5" y="10" width="15" height="11" rx="2.5" />
      <path d="M8 10V7a4 4 0 1 1 8 0v3m-4 4v3" />
    </template>
    <template v-else-if="name === 'spark'">
      <path d="m12 3 1.7 6.3L20 11l-6.3 1.7L12 19l-1.7-6.3L4 11l6.3-1.7L12 3Z" />
      <path d="m19 16 .7 2.3L22 19l-2.3.7L19 22l-.7-2.3L16 19l2.3-.7L19 16Z" />
    </template>
  </svg>
</template>
