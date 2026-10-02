<script setup lang="ts">
import AppIcon from "./AppIcon.vue";
withDefaults(defineProps<{ page: number; count: number; editing?: boolean; label?: string }>(), { label: "待办" });
defineEmits<{ move: [direction: number] }>();
</script>
<template>
  <nav v-if="count > 1" class="todo-pagination" :aria-label="`${label}分页`">
    <span class="todo-page-number" aria-live="polite">{{ page + 1 }} / {{ count }}</span>
    <div class="todo-page-actions"><button class="widget-icon-button" :aria-label="`上一页${label}`" title="上一页" :disabled="page === 0 || editing" @click="$emit('move', -1)"><AppIcon name="chevron-up" :size="14" /></button><button class="widget-icon-button" :aria-label="`下一页${label}`" title="下一页" :disabled="page >= count - 1 || editing" @click="$emit('move', 1)"><AppIcon name="chevron-down" :size="14" /></button></div>
  </nav>
</template>
