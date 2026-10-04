<script setup lang="ts">
import { onMounted, ref } from 'vue';
import VelaDatePicker from '../../../shared/ui/VelaDatePicker.vue';
import { saveCalendarEvent, deleteCalendarEvent } from '../../../app/store';
import { dateKey } from '../calendarEvents';
import type { CalendarEvent } from '../../../shared/types';
const props=defineProps<{ event?: CalendarEvent; date?: string; manager?: boolean }>();
const emit=defineEmits<{ close: [] }>();
const draft=ref<CalendarEvent>(props.event ? {...props.event} : {id:0,title:'',date:props.date ?? dateKey(new Date()),endDate:props.date ?? dateKey(new Date()),allDay:false,startTime:'09:00',endTime:'10:00',location:'',color:'#3b67b8'});
const busy=ref(false), error=ref(''), deleting=ref(false);
const titleInput=ref<HTMLInputElement|null>(null);
onMounted(()=>titleInput.value?.focus());
async function run(remove=false) { if(busy.value)return; busy.value=true;error.value='';try { if(remove) await deleteCalendarEvent(draft.value.id); else await saveCalendarEvent(draft.value); emit('close'); } catch(e) { error.value=e instanceof Error?e.message:typeof e==='string'?e:'没有保存成功，请重试。'; } finally { busy.value=false; } }
function startDate(value:string) { if(draft.value.endDate===draft.value.date || draft.value.endDate<value)draft.value.endDate=value; draft.value.date=value; }
</script>
<template>
  <form class="cal-event-editor" :class="{ 'in-manager': manager }" aria-label="日程编辑" @submit.prevent="run()" @keydown.esc.stop="emit('close')">
    <header><strong>{{ draft.id ? '编辑日程' : '新建日程' }}</strong><button type="button" class="wg-button" :disabled="busy" @click="emit('close')">取消</button></header>
    <label>名称<input ref="titleInput" v-model="draft.title" class="wg-textbox" aria-label="日程名称" maxlength="80" placeholder="例如：项目讨论" required /></label>
    <div class="cal-event-dates"><label>开始日期<VelaDatePicker :model-value="draft.date" :variant="manager ? 'manager' : 'widget'" label="日程开始日期" min="2000-01-01" @update:model-value="startDate" /></label><label>结束日期<VelaDatePicker v-model="draft.endDate" :variant="manager ? 'manager' : 'widget'" label="日程结束日期" :min="draft.date" /></label></div>
    <label class="cal-event-all-day"><input v-model="draft.allDay" type="checkbox" />全天</label>
    <div v-if="!draft.allDay" class="cal-event-dates"><label>开始时间<input v-model="draft.startTime" class="wg-textbox" type="time" aria-label="日程开始时间" required /></label><label>结束时间<input v-model="draft.endTime" class="wg-textbox" type="time" aria-label="日程结束时间" required /></label></div>
    <label>地点<input v-model="draft.location" class="wg-textbox" aria-label="日程地点" maxlength="120" placeholder="可选" /></label>
    <label class="cal-event-color">日程颜色<input v-model="draft.color" type="color" aria-label="日程颜色" /></label>
    <p v-if="error" class="clock-tool-error" role="alert">{{ error }}</p>
    <footer v-if="deleting"><span>删除这条日程？</span><button type="button" class="wg-button" :disabled="busy" @click="deleting=false">取消</button><button type="button" class="wg-button" :disabled="busy" @click="run(true)">确认删除</button></footer>
    <footer v-else><button v-if="draft.id" type="button" class="wg-button" :disabled="busy" @click="deleting=true">删除</button><button class="wg-button accent" :disabled="busy">{{ busy ? '保存中' : '保存日程' }}</button></footer>
  </form>
</template>
