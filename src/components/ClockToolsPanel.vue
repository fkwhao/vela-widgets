<script setup lang="ts">
import { computed, ref, watch, onMounted, onUnmounted } from "vue";
import AppIcon from "./AppIcon.vue";
import { snapshot, clockAction } from "../lib/store";
import { alarmRepeat, durationLabel, stopwatchElapsed, timerRemaining } from "../lib/clockTools";
import type { Alarm, ClockAction, ClockMode } from "../types";
const props=withDefaults(defineProps<{ size?: string; manager?: boolean; mode?: ClockMode }>(),{size:"large",manager:false});
const tools=computed(()=>snapshot.value.settings.clockTools);
const mode=computed(()=>props.mode ?? tools.value.mode);
const now=ref(Date.now()); let frame: ReturnType<typeof setTimeout> | undefined;
function update() { if(frame) clearTimeout(frame); now.value=Date.now(); if(!document.hidden) frame=setTimeout(update,mode.value==="stopwatch" && tools.value.stopwatch.startedAt!==null?50:250); }
onMounted(()=>{update();document.addEventListener("visibilitychange",update);});
onUnmounted(()=>{if(frame) clearTimeout(frame);document.removeEventListener("visibilitychange",update);});
const elapsed=computed(()=>stopwatchElapsed(tools.value,now.value));
const remaining=computed(()=>timerRemaining(tools.value,now.value));
const busy=ref(false),error=ref("");
async function act(input: ClockAction) { if(busy.value)return;busy.value=true;error.value="";try { await clockAction(input); }catch(e){error.value=typeof e==="string"?e:e instanceof Error?e.message:"操作没有完成。";}finally{busy.value=false;} }
const timerHours=ref(0),timerMinutes=ref(5),timerSeconds=ref(0),timerEditor=ref(false);
function configureTimer(){const seconds=tools.value.timer.durationSeconds;timerHours.value=Math.floor(seconds/3600);timerMinutes.value=Math.floor(seconds/60)%60;timerSeconds.value=seconds%60;timerEditor.value=true;}
async function saveTimer(){ await act({action:"timer-set",seconds:timerHours.value*3600+timerMinutes.value*60+timerSeconds.value});if(!error.value)timerEditor.value=false; }
const editor=ref(false),editingId=ref<number|null>(null),label=ref(""),time=ref("08:00"),date=ref(""),weekdays=ref<number[]>([]),deleting=ref<number|null>(null);
const days=["一","二","三","四","五","六","日"];
function edit(alarm?:Alarm){editingId.value=alarm?.id??null;label.value=alarm?.label??"";time.value=alarm?.time??"08:00";date.value=alarm?.date??"";weekdays.value=[...(alarm?.weekdays??[])];editor.value=true;error.value="";}
function toggleDay(day:number){date.value="";weekdays.value=weekdays.value.includes(day)?weekdays.value.filter(d=>d!==day):[...weekdays.value,day].sort();}
async function saveAlarm(){await act({action:"alarm-save",alarm:{id:editingId.value,label:label.value,time:time.value,date:date.value||null,weekdays:weekdays.value}});if(!error.value)editor.value=false;}
watch(mode,()=>{editor.value=false;timerEditor.value=false;error.value="";deleting.value=null;});
const nextAlarm=computed(()=>tools.value.alarms.filter(a=>a.enabled||a.snoozeAt!==null).sort((a,b)=>(a.snoozeAt??a.nextAt??Infinity)-(b.snoozeAt??b.nextAt??Infinity))[0]);
const alarms=computed(()=>props.size==="small"&&!props.manager?(nextAlarm.value?[nextAlarm.value]:tools.value.alarms.slice(0,1)):tools.value.alarms);
const laps=computed(()=>tools.value.stopwatch.laps.map((total,index)=>({index:index+1,total,split:total-(tools.value.stopwatch.laps[index-1]??0)})).reverse());
const nextLabel=(alarm:Alarm)=>alarm.snoozeAt!==null?"稍后提醒中":alarm.nextAt===null?"已关闭":new Date(alarm.nextAt).toLocaleString("zh-CN",{month:"numeric",day:"numeric",hour:"2-digit",minute:"2-digit"});
</script>
<template>
  <section class="clock-tools-panel" :class="[`tools-${size}`,`tool-mode-${mode}`,{'clock-management':manager}]" :aria-busy="busy">
    <template v-if="mode === 'stopwatch'">
      <div class="clock-counter"><span class="extra-eyebrow">{{ tools.stopwatch.startedAt !== null ? '正在计时' : elapsed ? '已暂停' : '准备开始' }}</span><strong>{{ durationLabel(elapsed,true) }}</strong></div>
      <div class="clock-tool-actions"><button class="clock-tool-button primary" :disabled="busy" @click="act({action:'stopwatch-toggle'})">{{ tools.stopwatch.startedAt !== null ? '暂停' : elapsed ? '继续' : '开始' }}</button><button class="clock-tool-button" :disabled="busy || !elapsed" @click="act({action:'stopwatch-reset'})">重置</button><button class="clock-tool-button" :disabled="busy || tools.stopwatch.startedAt === null || laps.length >= 100" @click="act({action:'stopwatch-lap'})">计次</button></div>
      <div v-if="size === 'large' || manager" class="clock-laps" aria-label="分段记录"><div v-for="lap in laps" :key="lap.index"><span>第 {{ lap.index }} 次</span><span>{{ durationLabel(lap.split,true) }}</span><strong>{{ durationLabel(lap.total,true) }}</strong></div><p v-if="!laps.length" class="extra-empty-caption">开始后点击计次，记录每一段时间</p></div>
      <span v-else-if="laps.length" class="extra-muted">第 {{ laps[0].index }} 次 · {{ durationLabel(laps[0].split,true) }}</span>
    </template>
    <template v-else-if="mode === 'timer'">
      <div class="clock-counter timer-counter" :style="{'--timer-progress': `${Math.min(100,remaining/(tools.timer.durationSeconds*1000)*100)}%`}"><span class="extra-eyebrow">{{ tools.timer.deadline !== null ? '剩余时间' : remaining === 0 ? '计时结束' : remaining < tools.timer.durationSeconds*1000 ? '已暂停' : '准备开始' }}</span><strong>{{ durationLabel(remaining) }}</strong></div>
      <div class="clock-tool-actions"><button class="clock-tool-button primary" :disabled="busy" @click="act({action:'timer-toggle'})">{{ tools.timer.deadline !== null ? '暂停' : remaining < tools.timer.durationSeconds*1000 && remaining > 0 ? '继续' : '开始' }}</button><button class="clock-tool-button" :disabled="busy" @click="act({action:'timer-reset'})">重置</button><button class="clock-tool-button" :disabled="busy || tools.timer.deadline !== null" aria-label="设置计时器时长" @click="configureTimer">时长</button></div>
      <div v-if="size !== 'small' || manager" class="clock-timer-presets" aria-label="快捷计时"><button v-for="minutes in [1,5,10,25]" :key="minutes" class="clock-tool-button" :disabled="busy || tools.timer.deadline !== null" @click="act({action:'timer-set',seconds:minutes*60})">{{ minutes }} 分钟</button></div>
      <form v-if="timerEditor" class="clock-tool-editor clock-timer-editor" @submit.prevent="saveTimer"><strong>设置时长</strong><div class="clock-duration-fields"><label>时<input v-model.number="timerHours" type="number" min="0" max="24" required aria-label="计时器小时" /></label><label>分<input v-model.number="timerMinutes" type="number" min="0" max="59" required aria-label="计时器分钟" /></label><label>秒<input v-model.number="timerSeconds" type="number" min="0" max="59" required aria-label="计时器秒" /></label></div><div class="clock-tool-actions"><button class="clock-tool-button primary" :disabled="busy">保存</button><button type="button" class="clock-tool-button" @click="timerEditor=false;error=''">取消</button></div><p class="extra-muted">1 秒至 24 小时</p><p v-if="error" class="clock-tool-error" role="alert">{{ error }}</p></form>
    </template>
    <template v-else-if="mode === 'alarm'">
      <div class="clock-alarm-heading"><span class="extra-eyebrow">{{ nextAlarm ? '下一次提醒' : '闹钟' }}</span><button class="widget-icon-button" aria-label="新建闹钟" :disabled="busy" @click="edit()"><AppIcon name="plus" :size="15" /></button></div>
      <div class="clock-alarm-list"><div v-for="alarm in alarms" :key="alarm.id" class="clock-alarm-row"><button class="clock-alarm-edit" :aria-label="`编辑闹钟 ${alarm.label}`" @click="edit(alarm)"><strong>{{ alarm.time }}</strong><span>{{ alarm.label }} · {{ size==='small'?nextLabel(alarm):alarmRepeat(alarm) }}</span><small v-if="size === 'large' || manager">{{ nextLabel(alarm) }}</small></button><button class="clock-alarm-toggle" :class="{on:alarm.enabled}" role="switch" :aria-label="`启用闹钟 ${alarm.label}`" :aria-checked="alarm.enabled" :disabled="busy" @click="act({action:'alarm-toggle',id:alarm.id})"><i></i></button><button v-if="size !== 'small' || manager" class="widget-icon-button" :aria-label="`删除闹钟 ${alarm.label}`" @click="deleting=alarm.id"><AppIcon name="trash" :size="13" /></button></div><p v-if="!alarms.length" class="extra-empty-caption">点击 ＋ 添加闹钟</p></div>
      <form v-if="editor" class="clock-tool-editor" @submit.prevent="saveAlarm"><strong>{{ editingId === null ? '新建闹钟' : '编辑闹钟' }}</strong><label>名称<input v-model="label" maxlength="40" placeholder="例如：起床" aria-label="闹钟名称" /></label><label>时间<input v-model="time" type="time" required aria-label="闹钟时间" /></label><label>指定日期<input v-model="date" type="date" min="0001-01-01" max="9999-12-31" aria-label="闹钟日期" @change="weekdays=[]" /></label><div><span class="extra-muted">每周重复（不选则仅一次）</span><div class="clock-weekdays"><button v-for="(day,index) in days" :key="index" type="button" :class="{selected:weekdays.includes(index)}" :aria-pressed="weekdays.includes(index)" :aria-label="`每周${day}`" @click="toggleDay(index)">{{ day }}</button></div></div><div class="clock-tool-actions"><button class="clock-tool-button primary" :disabled="busy">保存</button><button type="button" class="clock-tool-button" @click="editor=false;error=''">取消</button><button v-if="editingId !== null" type="button" class="clock-tool-button" @click="deleting=editingId">删除</button></div><p v-if="error" class="clock-tool-error" role="alert">{{ error }}</p></form>
      <div v-if="deleting !== null" class="clock-delete-confirm"><p>删除这个闹钟？</p><button class="clock-tool-button primary" :disabled="busy" @click="act({action:'alarm-delete',id:deleting!}).then(()=>{if(!error){deleting=null;editor=false;}})">确认删除</button><button class="clock-tool-button" @click="deleting=null">取消</button></div>
    </template>
    <p v-if="error && !editor && !timerEditor" class="clock-tool-error" role="alert">{{ error }}</p>
  </section>
</template>
