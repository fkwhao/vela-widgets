<script setup lang="ts">
import { computed, ref } from 'vue';
import VelaSelect from '../../../shared/ui/VelaSelect.vue';
import CalendarEventEditor from './CalendarEventEditor.vue';
import { snapshot, setCalendarSettings } from '../../../app/store';
import { calendarStyles, type CalendarEvent, type CalendarStyle } from '../../../shared/types';
const editing=ref<CalendarEvent|null|undefined>(undefined), error=ref(''), busy=ref(false);
const events=computed(()=>[...snapshot.value.settings.calendar.events].sort((a,b)=>a.date.localeCompare(b.date)||a.startTime.localeCompare(b.startTime)));
async function style(value:string) {busy.value=true;error.value='';try{await setCalendarSettings({...snapshot.value.settings.calendar,style:value as CalendarStyle});}catch(e){error.value=typeof e==='string'?e:'样式没有保存成功。';}finally{busy.value=false;}}
</script>
<template>
  <div class="settings-group calendar-style-settings"><div class="settings-card"><div class="card-text"><strong>日历样式</strong><span>日期／月历、按时间展示日程，或查看近期列表</span></div><VelaSelect :model-value="snapshot.settings.calendar.style" :options="calendarStyles" label="日历样式" :disabled="busy" @update:model-value="style" /></div></div>
  <div class="settings-section-heading"><h2 class="section-title">我的日程</h2><button class="win-button" @click="editing=null">新建日程</button></div>
  <p v-if="error" class="info-bar" role="alert">{{ error }}</p>
  <div v-if="editing !== undefined" class="settings-card stacked"><CalendarEventEditor manager :key="editing?.id ?? 0" :event="editing ?? undefined" @close="editing=undefined" /></div>
  <div class="settings-group"><div v-for="event in events" :key="event.id" class="settings-card"><span class="cal-pref-dot" :style="{background:event.color}"></span><div class="card-text"><strong>{{ event.title }}</strong><span>{{ event.date }}{{ event.endDate !== event.date ? ` 至 ${event.endDate}` : '' }} · {{ event.allDay ? '全天' : `${event.startTime}–${event.endTime}` }}{{ event.location ? ` · ${event.location}` : '' }}</span></div><button class="win-button" @click="editing=event">编辑</button></div><div v-if="!events.length" class="settings-card"><div class="card-text"><strong>还没有日程</strong><span>添加会议、出行或生活安排；日程与列表样式会同步显示</span></div></div></div>
</template>
