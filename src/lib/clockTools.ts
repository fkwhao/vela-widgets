import type { ClockTools, ClockAction, Alarm } from "../types";
export function stopwatchElapsed(tools: ClockTools, now = Date.now()) { return tools.stopwatch.elapsedMs + (tools.stopwatch.startedAt === null ? 0 : Math.max(0, now - tools.stopwatch.startedAt)); }
export function timerRemaining(tools: ClockTools, now = Date.now()) { return tools.timer.deadline === null ? tools.timer.remainingMs : Math.max(0, tools.timer.deadline - now); }
export function durationLabel(ms: number, hundredths = false) {
  const seconds = Math.max(0, hundredths ? Math.floor(ms / 1000) : Math.ceil(ms / 1000));
  const hh = Math.floor(seconds / 3600), mm = Math.floor(seconds / 60) % 60, ss = seconds % 60;
  const base = `${hh ? `${hh.toString().padStart(2,"0")}:` : ""}${mm.toString().padStart(2,"0")}:${ss.toString().padStart(2,"0")}`;
  return hundredths ? `${base}.${Math.floor(Math.max(0, ms) % 1000 / 10).toString().padStart(2,"0")}` : base;
}
export function alarmRepeat(alarm: Pick<Alarm,"date"|"weekdays">) { return alarm.weekdays.length ? "每周" + alarm.weekdays.map(day => ["一","二","三","四","五","六","日"][day]).join("、") : alarm.date ?? "仅一次"; }
export function nextAlarm(time: string, date: string | null, weekdays: number[], now: number): number {
  if (!/^([01]\d|2[0-3]):[0-5]\d$/.test(time) || weekdays.some(d => !Number.isInteger(d) || d < 0 || d > 6)) throw new Error("请输入有效时间和重复日期。");
  if (date && weekdays.length) throw new Error("指定日期不能与每周重复同时使用。");
  const [h,m] = time.split(":").map(Number);
  for (let offset=0;offset<9;offset++) {
    const day = new Date(now); day.setDate(day.getDate()+offset);
    if (date) { if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) throw new Error("请输入有效日期。"); const [y,mo,d]=date.split("-").map(Number); day.setFullYear(y,mo-1,d); if(day.getFullYear()!==y||day.getMonth()!==mo-1||day.getDate()!==d) throw new Error("请输入有效日期。"); }
    day.setHours(h,m,0,0);
    if ((!weekdays.length || weekdays.includes((day.getDay()+6)%7)) && day.getTime()>now) return day.getTime();
    if (date) break;
  }
  throw new Error("请选择未来的日期和时间。");
}
export function tickClockTools(tools: ClockTools, now=Date.now()): boolean {
  let changed=false;
  function alert(alarmId: number | null, label: string, due: number) { tools.alerts.push({ id: Math.max(now,...tools.alerts.map(a=>a.id+1)),alarmId,label,firedAt:now,missed:now-due>60000 }); changed=true; }
  if (tools.timer.deadline !== null && now>=tools.timer.deadline) { alert(null,"计时结束",tools.timer.deadline); tools.timer.deadline=null; tools.timer.remainingMs=0; }
  for (const alarm of tools.alarms) {
    const due=alarm.enabled && alarm.nextAt!==null && alarm.nextAt<=now;
    const snooze=alarm.snoozeAt!==null && alarm.snoozeAt<=now;
    if(due||snooze) { alert(alarm.id,alarm.label,(due?alarm.nextAt:alarm.snoozeAt)!); if(snooze) alarm.snoozeAt=null; if(due) { if(alarm.weekdays.length) alarm.nextAt=nextAlarm(alarm.time,null,alarm.weekdays,now); else { alarm.enabled=false;alarm.nextAt=null; } } }
  }
  tools.alerts=tools.alerts.slice(-32); return changed;
}
export function applyClockAction(tools: ClockTools, input: ClockAction, now=Date.now()) {
  const { action,id,seconds,mode }=input;
  tickClockTools(tools,now);
  switch(action) {
    case "mode": if(!mode||!["clock","alarm","stopwatch","timer"].includes(mode)) throw new Error("无效模式。"); tools.mode=mode;break;
    case "timer-set": if(!Number.isInteger(seconds)||seconds!<1||seconds!>86400) throw new Error("时长范围为 1 秒至 24 小时。"); tools.timer={durationSeconds:seconds!,remainingMs:seconds!*1000,deadline:null};break;
    case "timer-toggle": if(tools.timer.deadline!==null) { tools.timer.remainingMs=timerRemaining(tools,now);tools.timer.deadline=null; } else { if(tools.timer.remainingMs<=0) tools.timer.remainingMs=tools.timer.durationSeconds*1000;tools.timer.deadline=now+tools.timer.remainingMs; } break;
    case "timer-reset": tools.timer.deadline=null;tools.timer.remainingMs=tools.timer.durationSeconds*1000;break;
    case "stopwatch-toggle": if(tools.stopwatch.startedAt!==null) { tools.stopwatch.elapsedMs=stopwatchElapsed(tools,now);tools.stopwatch.startedAt=null; } else tools.stopwatch.startedAt=now;break;
    case "stopwatch-reset": tools.stopwatch={elapsedMs:0,startedAt:null,laps:[]};break;
    case "stopwatch-lap": if(tools.stopwatch.startedAt===null) throw new Error("开始秒表后才能计次。"); if(tools.stopwatch.laps.length>=100) throw new Error("最多记录 100 次。"); tools.stopwatch.laps.push(stopwatchElapsed(tools,now));break;
    case "alarm-save": { const a=input.alarm;if(!a) throw new Error("请填写闹钟。");if(a.label.trim().length>40) throw new Error("闹钟名称最多 40 字。"); const index=tools.alarms.findIndex(n=>n.id===a.id);if(a.id!==null&&index<0) throw new Error("这个闹钟已被删除。");if(index<0&&tools.alarms.length>=32) throw new Error("最多保存 32 个闹钟。"); const nextAt=nextAlarm(a.time,a.date,a.weekdays,now);const item={...a,id:a.id??Math.max(now,...tools.alarms.map(n=>n.id+1)),label:a.label.trim()||"闹钟",enabled:true,nextAt,snoozeAt:null,weekdays:[...new Set(a.weekdays)].sort()};if(index>=0) tools.alarms[index]=item;else tools.alarms.push(item);break; }
    case "alarm-toggle": { const a=tools.alarms.find(n=>n.id===id);if(!a) throw new Error("这个闹钟已被删除。");if(!a.enabled) a.nextAt=nextAlarm(a.time,a.date,a.weekdays,now);else { a.nextAt=null;a.snoozeAt=null; }a.enabled=!a.enabled;break; }
    case "alarm-delete": tools.alarms=tools.alarms.filter(n=>n.id!==id);tools.alerts=tools.alerts.filter(n=>n.alarmId!==id);break;
    case "dismiss": tools.alerts=id===undefined?[]:tools.alerts.filter(n=>n.id!==id);break;
    case "snooze": { const alert=tools.alerts.find(n=>n.id===id);if(!alert) throw new Error("这条提醒已结束。");if(alert.alarmId!==null) { const alarm=tools.alarms.find(n=>n.id===alert.alarmId);if(!alarm) throw new Error("这个闹钟已被删除。");alarm.snoozeAt=now+300000; }else tools.timer={durationSeconds:300,remainingMs:300000,deadline:now+300000};tools.alerts=tools.alerts.filter(n=>n.id!==id);break; }
    default: throw new Error("不支持这个操作。");
  }
}
