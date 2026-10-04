<script setup lang="ts">
import { computed, nextTick, ref, watch, onMounted, onUnmounted } from "vue";
import AppIcon from "../../../shared/ui/AppIcon.vue";
import VelaTimeInput from "../../../shared/ui/VelaTimeInput.vue";
import VelaDatePicker from "../../../shared/ui/VelaDatePicker.vue";
import { snapshot, clockAction } from "../../../app/store";
import { localDateKey } from "../../countdown/countdown";
import { alarmRepeat, durationLabel, stopwatchElapsed, timerRemaining } from "../clockTools";
import type { Alarm, ClockAction, ClockMode } from "../../../shared/types";
const props=withDefaults(defineProps<{ size?: string; manager?: boolean; mode?: ClockMode; drag?: string }>(),{size:"large",manager:false});
const tools=computed(()=>snapshot.value.settings.clockTools);
const mode=computed(()=>props.mode ?? tools.value.mode);
const root=ref<HTMLElement|null>(null);
const now=ref(Date.now()); let frame: ReturnType<typeof setTimeout> | undefined;
function update() { if(frame) clearTimeout(frame); now.value=Date.now(); if(!document.hidden) frame=setTimeout(update,mode.value==="stopwatch" && tools.value.stopwatch.startedAt!==null?50:250); }
onMounted(()=>{update();document.addEventListener("visibilitychange",update);});
onUnmounted(()=>{if(frame) clearTimeout(frame);document.removeEventListener("visibilitychange",update);});
const elapsed=computed(()=>stopwatchElapsed(tools.value,now.value));
const remaining=computed(()=>timerRemaining(tools.value,now.value));
const busy=ref(false),error=ref("");
async function act(input: ClockAction) { if(busy.value)return;busy.value=true;error.value="";try { await clockAction(input); }catch(e){error.value=typeof e==="string"?e:e instanceof Error?e.message:"操作没有完成。";}finally{busy.value=false;} }
// Editors open with the first time segment focused, as Windows Clock does.
async function focusEditor(){ await nextTick(); root.value?.querySelector<HTMLElement>(".clock-tool-editor [role='spinbutton']")?.focus(); }
const timerTotal=ref(tools.value.timer.durationSeconds),timerEditor=ref(false);
const timerReady=computed(()=>tools.value.timer.deadline===null && (remaining.value===0 || remaining.value===tools.value.timer.durationSeconds*1000));
watch(()=>tools.value.timer.durationSeconds,value=>{timerTotal.value=value;});
async function toggleDesktopTimer(){
  if(busy.value)return;
  if(timerReady.value){await act({action:"timer-set",seconds:timerTotal.value});if(error.value)return;}
  await act({action:"timer-toggle"});
}
function configureTimer(){timerTotal.value=tools.value.timer.durationSeconds;timerEditor.value=true;error.value="";void focusEditor();}
async function saveTimer(){ if(timerTotal.value<1){error.value="时长至少 1 秒。";return;} await act({action:"timer-set",seconds:timerTotal.value});if(!error.value)timerEditor.value=false; }
const editor=ref(false),editingId=ref<number|null>(null),label=ref(""),time=ref("08:00"),date=ref(""),weekdays=ref<number[]>([]),deleting=ref<number|null>(null);
const days=["一","二","三","四","五","六","日"];
// The time picker works in seconds of the day; alarms are stored as "HH:MM".
const alarmSeconds=computed({ get:()=>{const [h,m]=time.value.split(":").map(Number);return (h||0)*3600+(m||0)*60;}, set:(value:number)=>{time.value=`${String(Math.floor(value/3600)%24).padStart(2,"0")}:${String(Math.floor(value/60)%60).padStart(2,"0")}`;} });
const repeatSummary=computed(()=>alarmRepeat({date:date.value||null,weekdays:weekdays.value}));
function edit(alarm?:Alarm){editingId.value=alarm?.id??null;label.value=alarm?.label??"";time.value=alarm?.time??"08:00";date.value=alarm?.date??"";weekdays.value=[...(alarm?.weekdays??[])];editor.value=true;error.value="";void focusEditor();}
function toggleDay(day:number){date.value="";weekdays.value=weekdays.value.includes(day)?weekdays.value.filter(d=>d!==day):[...weekdays.value,day].sort();}
function chooseDate(value:string){date.value=value;if(value)weekdays.value=[];}
async function saveAlarm(){await act({action:"alarm-save",alarm:{id:editingId.value,label:label.value,time:time.value,date:date.value||null,weekdays:weekdays.value}});if(!error.value)editor.value=false;}
function closeEditors(){editor.value=false;timerEditor.value=false;error.value="";}
watch(mode,()=>{closeEditors();deleting.value=null;});
const nextAlarm=computed(()=>tools.value.alarms.filter(a=>a.enabled||a.snoozeAt!==null).sort((a,b)=>(a.snoozeAt??a.nextAt??Infinity)-(b.snoozeAt??b.nextAt??Infinity))[0]);
const alarms=computed(()=>props.size==="small"&&!props.manager?(nextAlarm.value?[nextAlarm.value]:tools.value.alarms.slice(0,1)):tools.value.alarms);
const laps=computed(()=>tools.value.stopwatch.laps.map((total,index)=>({index:index+1,total,split:total-(tools.value.stopwatch.laps[index-1]??0)})).reverse());
const nextLabel=(alarm:Alarm)=>alarm.snoozeAt!==null?"稍后提醒中":alarm.nextAt===null?"已关闭":new Date(alarm.nextAt).toLocaleString("zh-CN",{month:"numeric",day:"numeric",hour:"2-digit",minute:"2-digit"});
const compact=computed(()=>props.size!=="large"&&!props.manager);
</script>
<template>
  <section ref="root" class="clock-tools-panel" :class="[`tools-${size}`,`tool-mode-${mode}`,{'clock-management':manager, 'has-laps': mode === 'stopwatch' && laps.length > 0}]" :aria-busy="busy" :tabindex="!manager && mode==='stopwatch' ? 0 : undefined">
    <div v-if="!manager && drag" class="clock-tool-drag-handle" :data-tauri-drag-region="drag" aria-hidden="true"></div>
    <template v-if="mode === 'stopwatch'">
      <template v-if="manager">
      <div class="clock-counter"><span class="extra-eyebrow">{{ tools.stopwatch.startedAt !== null ? '正在计时' : elapsed ? '已暂停' : '准备开始' }}</span><strong>{{ durationLabel(elapsed,true) }}</strong></div>
      <div class="clock-tool-actions"><button class="wg-button accent" :disabled="busy" @click="act({action:'stopwatch-toggle'})">{{ tools.stopwatch.startedAt !== null ? '暂停' : elapsed ? '继续' : '开始' }}</button><button class="wg-button" :disabled="busy || !elapsed" @click="act({action:'stopwatch-reset'})">重置</button><button class="wg-button" :disabled="busy || tools.stopwatch.startedAt === null || laps.length >= 100" @click="act({action:'stopwatch-lap'})">计次</button></div>
      </template>
      <template v-else>
        <div class="clock-counter simple-counter" :data-tauri-drag-region="drag" :class="{'long-duration':elapsed>=3600000}"><strong>{{ durationLabel(elapsed,true) }}</strong></div>
        <div class="clock-round-actions" :data-tauri-drag-region="drag ? '' : undefined">
          <button class="clock-round-button secondary" :disabled="busy || (tools.stopwatch.startedAt !== null ? laps.length >= 100 : !elapsed)" @click="act({action:tools.stopwatch.startedAt !== null ? 'stopwatch-lap' : 'stopwatch-reset'})">{{ tools.stopwatch.startedAt !== null ? '分段' : '重置' }}</button>
          <button class="clock-round-button" :class="tools.stopwatch.startedAt !== null ? 'pause' : 'start'" :disabled="busy" @click="act({action:'stopwatch-toggle'})">{{ tools.stopwatch.startedAt !== null ? '暂停' : elapsed ? '继续' : '启动' }}</button>
        </div>
      </template>
      <div v-if="laps.length" class="clock-laps" :class="{ compact: size !== 'large' }" aria-label="分段记录" :data-tauri-drag-region="manager ? undefined : drag"><div v-for="lap in laps" :key="lap.index"><span>第 {{ lap.index }} 次</span><span>{{ durationLabel(lap.split,true) }}</span><strong>{{ durationLabel(lap.total,true) }}</strong></div></div>
      <p v-else-if="manager" class="extra-empty-caption" style="margin-top: 12px;">开始后点击计次，记录每一段时间</p>
    </template>
    <template v-else-if="mode === 'timer'">
      <template v-if="manager">
      <div class="clock-counter timer-counter" :style="{'--timer-progress': `${Math.min(100,remaining/(tools.timer.durationSeconds*1000)*100)}%`}"><span class="extra-eyebrow">{{ tools.timer.deadline !== null ? '剩余时间' : remaining === 0 ? '计时结束' : remaining < tools.timer.durationSeconds*1000 ? '已暂停' : '准备开始' }}</span><strong>{{ durationLabel(remaining) }}</strong></div>
      <div class="clock-tool-actions"><button class="wg-button accent" :disabled="busy" @click="act({action:'timer-toggle'})">{{ tools.timer.deadline !== null ? '暂停' : remaining < tools.timer.durationSeconds*1000 && remaining > 0 ? '继续' : '开始' }}</button><button class="wg-button" :disabled="busy" @click="act({action:'timer-reset'})">重置</button><button class="wg-button" :disabled="busy || tools.timer.deadline !== null" aria-label="设置计时器时长" @click="configureTimer">时长</button></div>
      <div v-if="size !== 'small' || manager" class="clock-timer-presets" aria-label="快捷计时"><button v-for="minutes in [1,5,10,25]" :key="minutes" class="wg-button" :disabled="busy || tools.timer.deadline !== null" @click="act({action:'timer-set',seconds:minutes*60})">{{ minutes }} 分钟</button></div>
      </template>
      <template v-else>
        <div v-if="timerReady" class="clock-inline-timer" :data-tauri-drag-region="drag ? '' : undefined"><VelaTimeInput v-model="timerTotal" seconds duration label="计时器时长" :compact="size!=='large'" /></div>
        <div v-else class="clock-counter simple-counter" :data-tauri-drag-region="drag" :class="{'long-duration':remaining>=3600000}"><strong>{{ durationLabel(remaining) }}</strong></div>
        <div class="clock-round-actions" :data-tauri-drag-region="drag ? '' : undefined">
          <button class="clock-round-button secondary" aria-label="取消计时" :disabled="busy || timerReady" @click="act({action:'timer-reset'})"><AppIcon name="close" :size="size==='small'?18:24" /></button>
          <button class="clock-round-button" :class="tools.timer.deadline !== null ? 'pause' : 'start'" :disabled="busy || (timerReady && timerTotal<1)" :aria-label="tools.timer.deadline !== null ? '暂停计时' : '启动计时'" @click="toggleDesktopTimer"><AppIcon :name="tools.timer.deadline !== null ? 'pause' : 'play'" :size="size==='small'?18:24" /></button>
        </div>
      </template>
      <form v-if="timerEditor" class="clock-tool-editor clock-timer-editor" @submit.prevent="saveTimer" @keydown.esc.stop="closeEditors"><strong>设置时长</strong><VelaTimeInput v-model="timerTotal" seconds duration label="计时器时长" :compact="compact" /><p class="extra-muted">1 秒至 24 小时 · 滚轮或方向键调整</p><div class="clock-tool-actions"><button type="button" class="wg-button" @click="closeEditors">取消</button><button class="wg-button accent" :disabled="busy">保存</button></div><p v-if="error" class="clock-tool-error" role="alert">{{ error }}</p></form>
    </template>
    <template v-else-if="mode === 'alarm'">
      <div class="clock-alarm-heading" :data-tauri-drag-region="drag ? '' : undefined"><span class="extra-eyebrow" :data-tauri-drag-region="drag">{{ nextAlarm ? '下一次提醒' : '闹钟' }}</span><button class="widget-icon-button" aria-label="新建闹钟" data-tooltip="新建闹钟" :disabled="busy" @click="edit()"><AppIcon name="plus" :size="15" /></button></div>
      <div class="clock-alarm-list"><div v-for="alarm in alarms" :key="alarm.id" class="clock-alarm-row"><button class="clock-alarm-edit" :aria-label="`编辑闹钟 ${alarm.label}`" @click="edit(alarm)"><strong>{{ alarm.time }}</strong><span>{{ alarm.label }} · {{ size==='small'?nextLabel(alarm):alarmRepeat(alarm) }}</span><small v-if="size === 'large' || manager">{{ nextLabel(alarm) }}</small></button><button class="wg-switch" :class="{on:alarm.enabled}" role="switch" :aria-label="`启用闹钟 ${alarm.label}`" :aria-checked="alarm.enabled" :disabled="busy" @click="act({action:'alarm-toggle',id:alarm.id})"><span></span></button><button v-if="size !== 'small' || manager" class="widget-icon-button" :aria-label="`删除闹钟 ${alarm.label}`" data-tooltip="删除" @click="deleting=alarm.id"><AppIcon name="trash" :size="13" /></button></div><p v-if="!alarms.length" class="extra-empty-caption" :data-tauri-drag-region="drag">点击 ＋ 添加闹钟</p></div>
      <form v-if="editor" class="clock-tool-editor" @submit.prevent="saveAlarm" @keydown.esc.stop="closeEditors">
        <strong>{{ editingId === null ? '新建闹钟' : '编辑闹钟' }}</strong>
        <VelaTimeInput v-model="alarmSeconds" label="闹钟时间" :compact="compact" />
        <div class="clock-editor-side">
          <label class="clock-editor-field"><span v-if="size !== 'small' || manager">名称</span><input v-model="label" class="wg-textbox" maxlength="40" placeholder="闹钟名称，例如：起床" aria-label="闹钟名称" spellcheck="false" autocomplete="off" /></label>
          <div class="clock-editor-field"><span class="clock-editor-hint"><span>重复</span><span>{{ repeatSummary }}</span></span><div class="clock-weekdays" role="group" aria-label="每周重复"><button v-for="(day,index) in days" :key="index" type="button" :class="{selected:weekdays.includes(index)}" :aria-pressed="weekdays.includes(index)" :aria-label="`每周${day}`" @click="toggleDay(index)">{{ day }}</button></div></div>
          <div v-if="size !== 'small' || manager" class="clock-editor-field"><span>仅一次的日期</span><VelaDatePicker :model-value="date" label="闹钟日期" placeholder="下一次到点时" clearable :min="localDateKey(new Date())" :variant="manager ? 'manager' : 'widget'" @update:model-value="chooseDate" /></div>
        </div>
        <div class="clock-tool-actions"><button v-if="editingId !== null" type="button" class="wg-button danger" @click="deleting=editingId">删除</button><button type="button" class="wg-button" @click="closeEditors">取消</button><button class="wg-button accent" :disabled="busy">保存</button></div>
        <p v-if="error" class="clock-tool-error" role="alert">{{ error }}</p>
      </form>
      <div v-if="deleting !== null" class="clock-delete-confirm" role="alertdialog" aria-label="删除闹钟" @keydown.esc.stop="deleting=null"><p>删除这个闹钟？</p><div class="clock-tool-actions"><button class="wg-button" @click="deleting=null">取消</button><button class="wg-button accent" :disabled="busy" @click="act({action:'alarm-delete',id:deleting!}).then(()=>{if(!error){deleting=null;editor=false;}})">删除</button></div></div>
    </template>
    <p v-if="error && !editor && !timerEditor" class="clock-tool-error" role="alert">{{ error }}</p>
  </section>
</template>
