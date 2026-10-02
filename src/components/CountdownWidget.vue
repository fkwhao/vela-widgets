<script setup lang="ts">
import { computed, ref, watch } from "vue";
import WidgetFrame from "./WidgetFrame.vue";
import WidgetPageControls from "./WidgetPageControls.vue";
import { snapshot } from "../lib/store";
import { useWidgetClock } from "../lib/useWidgetClock";
import { sortedCountdowns } from "../lib/countdown";
const now = useWidgetClock(undefined, true);
const events = computed(() => sortedCountdowns(snapshot.value.countdowns, now.value));
const size = computed(() => snapshot.value.settings.widgets.countdown.size);
const page = ref(0);
const direction = ref(1);
const capacity = computed(() => size.value === "large" ? 3 : 1);
const pageCount = computed(() => Math.max(1, Math.ceil(events.value.length / capacity.value)));
const pageEvents = computed(() => events.value.slice(page.value * capacity.value, (page.value + 1) * capacity.value));
const main = computed(() => pageEvents.value[0]);
watch(size, () => { page.value = 0; });
watch(pageCount, count => { page.value = Math.min(page.value, count - 1); });
function movePage(step: number) { direction.value = step; page.value = Math.max(0, Math.min(pageCount.value - 1, page.value + step)); }
</script>
<template>
  <WidgetFrame kind="countdown" header-only v-slot="{ widget, drag }">
    <header v-if="widget.size === 'large'" class="extra-header" :data-tauri-drag-region="drag"><h1>倒数日</h1><span class="extra-caption">{{ events.length }} 个日子</span></header>
    <div v-if="!main" class="extra-empty" :data-tauri-drag-region="drag"><span class="extra-eyebrow">倒数日</span><strong class="countdown-empty-number">—</strong><p>留一个值得期待的日子</p><span class="extra-caption">右键打开偏好设置添加</span></div>
    <div v-else-if="widget.size !== 'large'" :key="`${widget.size}-${page}`" class="countdown-compact todo-page-list" :data-direction="direction < 0 ? 'up' : 'down'" :data-tauri-drag-region="drag">
      <div class="countdown-hero"><span class="extra-eyebrow" :title="main.item.title">{{ main.item.title }}</span><div class="countdown-number" :class="{ 'long-number': main.days > 999 }">{{ main.days }}<small>天</small></div><span class="extra-caption">{{ main.caption }}</span><span class="extra-muted">{{ main.dateLabel }}</span></div>
    </div>
    <div v-else :key="`large-${page}`" class="countdown-list todo-page-list" :data-direction="direction < 0 ? 'up' : 'down'"><article v-for="entry in pageEvents" :key="entry.item.id" class="countdown-row"><div class="countdown-row-title"><strong :title="entry.item.title">{{ entry.item.title }}</strong><span>{{ entry.caption }} <b>{{ entry.days }}</b> 天</span></div><div class="countdown-row-meta"><span>{{ entry.dateLabel }}</span><span v-if="entry.item.yearly">每年重复</span></div><div v-if="entry.progress !== null" class="countdown-progress" role="progressbar" :aria-label="`${entry.item.title}进度`" :aria-valuenow="Math.round(entry.progress)" aria-valuemin="0" aria-valuemax="100"><i :style="{ width: `${entry.progress}%` }"></i></div></article></div>
    <WidgetPageControls :page="page" :count="pageCount" label="倒数日" @move="movePage" />
  </WidgetFrame>
</template>
