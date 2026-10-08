<script setup lang="ts">
import { computed, defineAsyncComponent, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import WidgetFrame from '../../../shared/widgets/WidgetFrame.vue';
import AppIcon from '../../../shared/ui/AppIcon.vue';
const CalendarEventEditor = defineAsyncComponent(() => import('./CalendarEventEditor.vue'));
import { snapshot, setCalendarSettings } from '../../../app/store';
import { calendarStyles, type CalendarEvent, type CalendarStyle } from '../../../shared/types';
import { addDays, dateKey, dayEntries, upcomingEntries, timelineEvents, type CalendarEntry } from '../calendarEvents';
const now=ref(new Date()), offset=ref(0), page=ref(0), editing=ref<CalendarEvent|null|undefined>(undefined), error=ref('');
let timer:ReturnType<typeof setInterval>|undefined;
onMounted(()=>{timer=setInterval(()=>now.value=new Date(),30_000);void resetTimelines();});onUnmounted(()=>{if(timer)clearInterval(timer);});
const calendar=computed(()=>snapshot.value.settings.calendar), size=computed(()=>snapshot.value.settings.widgets.calendar.size);
const today=computed(()=>dateKey(now.value)), base=computed(()=>addDays(today.value,offset.value));
const holidays=computed(()=>snapshot.value.holidays.data.years.flatMap(y=>y.days));
const currentTime=computed(()=>`${String(now.value.getHours()).padStart(2,'0')}:${String(now.value.getMinutes()).padStart(2,'0')}`);
const entries=computed(()=>upcomingEntries(calendar.value.events,holidays.value,calendar.value,today.value,currentTime.value));
const pageSize=computed(()=>size.value==='large'?5:2);
const pages=computed(()=>Math.max(1,Math.ceil(entries.value.length/pageSize.value)));
const visible=computed(()=>entries.value.slice(page.value*pageSize.value,(page.value+1)*pageSize.value));
watch(pages,count=>page.value=Math.min(page.value,count-1));
const days=computed(()=>[base.value,addDays(base.value,1)]);
function rows(day:string) {return dayEntries(calendar.value.events,holidays.value,calendar.value,day).filter(e=>day!==today.value || !e.event || e.event.allDay || e.event.endDate>day || e.event.endTime>currentTime.value);}
function entryHint(entry:CalendarEntry) {return `${entry.title} · ${entry.detail}${entry.event?.location ? ' · '+entry.event.location : ''}`;}
function dateCaption(day:string) {const d=new Date(`${day}T12:00:00`);return `${d.getMonth()+1}月${d.getDate()}日 ${new Intl.DateTimeFormat('zh-CN',{weekday:'long'}).format(d)}`;}
function dayLabel(day:string) {return day===today.value?'今天':day===addDays(today.value,1)?'明天':dateCaption(day);}
function lunar(day:string) {try{const parts=new Intl.DateTimeFormat('zh-CN-u-ca-chinese',{month:'long',day:'numeric'}).formatToParts(new Date(`${day}T12:00:00`));const d=Number(parts.find(p=>p.type==='day')?.value),digits=['一','二','三','四','五','六','七','八','九'];const label=d<10?`初${digits[d-1]}`:d===10?'初十':d<20?`十${digits[d-11]}`:d===20?'二十':d<30?`廿${digits[d-21]}`:'三十';return `${parts.find(p=>p.type==='month')?.value ?? ''}${label}`;}catch{return '';}}
const hours=Array.from({length:24},(_,i)=>i);
const timelineViews=new Map<string,HTMLElement>();
function bindTimeline(el:unknown,day:string) {if(el instanceof HTMLElement)timelineViews.set(day,el);else timelineViews.delete(day);}
async function resetTimelines() {await nextTick();for(const [day,el] of timelineViews)el.scrollTop=(day===today.value?Math.min(Math.max(0,now.value.getHours()-1),18):9)*31;}
watch([base,size,()=>calendar.value.style],()=>void resetTimelines(),{flush:'post'});
function returnToday() {offset.value=0;void resetTimelines();}
function blocks(day:string) {return timelineEvents(calendar.value.events,day,0,1440);}
function blockStyle(item:ReturnType<typeof timelineEvents>[number]) {return {top:`${item.from/60*31}px`,height:`${(item.to-item.from)/60*31}px`,left:`calc(28px + (100% - 28px) * ${item.lane/item.lanes})`,width:`calc((100% - 28px) / ${item.lanes} - 2px)`,'--event-color':item.event.color};}
function nowPosition(day:string) {return day===today.value ? (now.value.getHours()+now.value.getMinutes()/60)*31 : -1;}
async function chooseStyle(style:CalendarStyle) {error.value='';try{await setCalendarSettings({...calendar.value,style});}catch(e){error.value=typeof e==='string'?e:'样式没有保存成功。';}}
</script>
<template>
  <WidgetFrame kind="calendar" header-only class="cal-schedule" :class="`cal-style-${calendar.style}`">
    <template #default="{drag}">
      <header class="cal-schedule-toolbar" :data-tauri-drag-region="drag"><span>{{ calendar.style==='agenda'?'日程':'近期列表' }}</span><div><button class="widget-icon-button" aria-label="新建日程" @click="editing=null"><AppIcon name="plus" :size="14" /></button><button class="widget-icon-button" aria-label="切换为日期月历" @click="chooseStyle('month')"><AppIcon name="calendar" :size="14" /></button><button class="widget-icon-button" :aria-label="calendar.style==='agenda'?'切换为列表':'切换为日程'" @click="chooseStyle(calendar.style==='agenda'?'list':'agenda')"><AppIcon name="grid" :size="14" /></button></div></header>
      <template v-if="calendar.style==='list'">
        <div class="cal-list-entries"><article v-for="(entry,index) in visible" :key="entry.key" class="cal-list-entry"><header v-if="index===0 || visible[index-1].date!==entry.date" :class="{today:entry.date===today}">{{ entry.date===today ? dateCaption(entry.date) : dayLabel(entry.date) }}<span>{{ lunar(entry.date) }}</span></header><button v-if="entry.event" class="cal-event-pill" :class="{holiday:!entry.event}" :data-tooltip="entryHint(entry)" :style="{'--event-color':entry.color}" :aria-label="`编辑日程 ${entry.title}`" @click="editing=entry.event"><strong>{{ entry.title }}</strong><span>{{ entry.detail }}</span></button><div v-else class="cal-event-pill" :class="{holiday:!entry.event}" :data-tooltip="entryHint(entry)" :style="{'--event-color':entry.color}"><strong>★ {{ entry.title }}</strong><span>{{ entry.detail }}</span></div></article><p v-if="!entries.length" class="cal-schedule-empty">近期没有日程<br /><span>点击 ＋ 添加安排</span></p></div>
        <div class="immersive-footer-zone cal-footer-zone"><footer class="cal-schedule-pager immersive-footer"><button class="widget-icon-button" aria-label="上一页日程" :disabled="page===0" @click="page--"><AppIcon name="chevron-up" :size="13" /></button><span>{{ page+1 }} / {{ pages }}</span><button class="widget-icon-button" aria-label="下一页日程" :disabled="page+1>=pages" @click="page++"><AppIcon name="chevron-down" :size="13" /></button></footer></div>
      </template>
      <template v-else>
        <div class="cal-agenda-days" :class="{single:size==='small'}"><section v-for="day in size==='small'?[base]:days" :key="day" class="cal-agenda-day"><header class="cal-agenda-date" :data-tauri-drag-region="drag"><strong>{{ new Date(`${day}T12:00:00`).getDate() }}</strong><div><span>{{ dayLabel(day) }}</span><small>{{ lunar(day) }}</small></div></header>
          <template v-if="size==='large'"><div class="cal-all-day"><div v-for="entry in rows(day).filter(e=>!e.event || e.event.allDay)" :key="entry.key" class="cal-event-pill" :class="{holiday:!entry.event}" :data-tooltip="entryHint(entry)" :style="{'--event-color':entry.color}"><button v-if="entry.event" :aria-label="`编辑日程 ${entry.title}`" @click="editing=entry.event">{{ entry.title }}</button><strong v-else>★ {{ entry.title }}</strong></div><span v-if="!rows(day).some(e=>!e.event || e.event.allDay)" class="cal-no-all-day">无全天日程</span></div><div :ref="el=>bindTimeline(el,day)" class="cal-timeline-scroll" :aria-label="`${dayLabel(day)}时间轴`" tabindex="0"><div class="cal-timeline"><div v-for="hour in hours" :key="hour" class="cal-hour-line"><span>{{ hour }}时</span></div><button v-for="item in blocks(day)" :key="item.event.id" class="cal-time-block" :style="blockStyle(item)" :aria-label="`编辑日程 ${item.event.title}`" :data-tooltip="`${item.event.title} · ${item.event.startTime}–${item.event.endTime}${item.event.location?' · '+item.event.location:''}`" @click="editing=item.event"><strong>{{ item.event.title }}</strong><small>{{ item.event.startTime }}</small></button><div v-if="nowPosition(day)>=0" class="cal-now-line" :style="{top:`${nowPosition(day)}px`}"></div></div></div></template>
          <template v-else><div class="cal-agenda-compact-entries" :aria-label="`${dayLabel(day)}日程`" tabindex="0"><template v-for="entry in rows(day)" :key="entry.key"><button v-if="entry.event" class="cal-event-pill" :class="{holiday:!entry.event}" :data-tooltip="entryHint(entry)" :style="{'--event-color':entry.color}" :aria-label="`编辑日程 ${entry.title}`" @click="editing=entry.event"><strong>{{ entry.title }}</strong><span>{{ entry.detail }}</span></button><div v-else class="cal-event-pill" :class="{holiday:!entry.event}" :data-tooltip="entryHint(entry)" :style="{'--event-color':entry.color}"><strong>★ {{ entry.title }}</strong></div></template><p v-if="!rows(day).length" class="cal-schedule-empty">没有安排</p></div></template>
        </section></div>
        <div class="immersive-footer-zone cal-footer-zone"><footer class="cal-schedule-pager immersive-footer"><button class="widget-icon-button" aria-label="前一天" @click="offset--"><AppIcon name="chevron-left" :size="13" /></button><button class="widget-icon-button" aria-label="日程回到今天" data-tooltip="回到今天" @click="returnToday"><AppIcon name="return-today" :size="14" /></button><button class="widget-icon-button" aria-label="后一天" @click="offset++"><AppIcon name="chevron-right" :size="13" /></button></footer></div>
      </template>
      <CalendarEventEditor v-if="editing!==undefined" :event="editing ?? undefined" :date="base" @close="editing=undefined" />
      <p v-if="error" class="extra-notice" role="alert">{{ error }}</p>
    </template>
    <template #context-actions="{dismiss}"><button v-for="style in calendarStyles" :key="style.value" role="menuitem" @click="chooseStyle(style.value).then(dismiss)">{{ style.label }}</button><div class="context-divider"></div></template>
  </WidgetFrame>
</template>
