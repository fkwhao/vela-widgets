import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import ts from "typescript";
const source=readFileSync(new URL("../src/features/clock/clockTools.ts",import.meta.url),"utf8");
const output=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext}}).outputText;
const {applyClockAction,tickClockTools,nextAlarm,stopwatchElapsed,timerRemaining,durationLabel}=await import(`data:text/javascript;base64,${Buffer.from(output).toString("base64")}`);
const initial=()=>({mode:"clock",alarms:[],timer:{durationSeconds:300,remainingMs:300000,deadline:null},stopwatch:{elapsedMs:0,startedAt:null,laps:[]},alerts:[]});
test("running stopwatch and timer retain elapsed time across mode switches and reloads",()=>{
 let t=initial();applyClockAction(t,{action:"timer-set",seconds:10},1000);applyClockAction(t,{action:"timer-toggle"},1000);applyClockAction(t,{action:"stopwatch-toggle"},1000);applyClockAction(t,{action:"mode",mode:"clock"},2000);
 t=JSON.parse(JSON.stringify(t));assert.equal(stopwatchElapsed(t,4500),3500);assert.equal(timerRemaining(t,4500),6500);
 applyClockAction(t,{action:"stopwatch-lap"},4500);assert.deepEqual(t.stopwatch.laps,[3500]);applyClockAction(t,{action:"stopwatch-toggle"},5000);assert.equal(stopwatchElapsed(t,10000),4000);
 applyClockAction(t,{action:"timer-toggle"},5000);assert.equal(timerRemaining(t,10000),6000);applyClockAction(t,{action:"timer-toggle"},10000);assert.equal(t.timer.deadline,16000);
});
test("overdue timers fire once, retain a missed reminder and can be snoozed",()=>{
 const t=initial();t.timer.deadline=1000;assert.ok(tickClockTools(t,100000));assert.ok(t.alerts[0].missed);assert.equal(t.alerts.length,1);assert.equal(tickClockTools(t,100001),false);
 applyClockAction(t,{action:"snooze",id:t.alerts[0].id},100001);assert.equal(t.alerts.length,0);assert.equal(t.timer.deadline,400001);assert.ok(tickClockTools(t,400001));applyClockAction(t,{action:"dismiss"},400002);assert.equal(t.alerts.length,0);
});
test("one-off and weekly alarms fire once; weekly snooze preserves the next regular occurrence",()=>{
 const now=new Date(2026,9,5,7).getTime(),t=initial();applyClockAction(t,{action:"alarm-save",alarm:{id:null,label:"起床",time:"08:00",date:null,weekdays:[0,1,2,3,4]}},now);
 const due=t.alarms[0].nextAt;assert.ok(tickClockTools(t,due));const next=t.alarms[0].nextAt;assert.ok(next>due);applyClockAction(t,{action:"snooze",id:t.alerts[0].id},due);assert.equal(t.alarms[0].nextAt,next);assert.ok(tickClockTools(t,due+300000));assert.equal(tickClockTools(t,due+300001),false);
 applyClockAction(t,{action:"alarm-save",alarm:{id:null,label:"一次",time:"09:00",date:null,weekdays:[]}},now);const single=t.alarms[1];tickClockTools(t,single.nextAt);assert.equal(single.enabled,false);assert.equal(single.nextAt,null);
});
test("alarm and duration validation rejects impossible input and preserves limits",()=>{
 assert.throws(()=>nextAlarm("25:00",null,[],Date.now()));assert.throws(()=>nextAlarm("08:00","2026-02-30",[],new Date(2026,0,1).getTime()));assert.throws(()=>nextAlarm("08:00","2000-01-01",[],Date.now()));assert.throws(()=>nextAlarm("08:00","2027-01-01",[0],Date.now()));
 const t=initial();for(const seconds of [0,-1,86401,1.5]) assert.throws(()=>applyClockAction(t,{action:"timer-set",seconds},1));
 assert.equal(durationLabel(1001),"00:02");assert.equal(durationLabel(1001,true),"00:01.00");assert.equal(durationLabel(86400000),"24:00:00");
});
