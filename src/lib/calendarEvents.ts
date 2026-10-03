import type { CalendarEvent, CalendarSettings, HolidayDay } from "../types";

export function dateKey(date: Date): string { return `${date.getFullYear()}-${String(date.getMonth()+1).padStart(2,'0')}-${String(date.getDate()).padStart(2,'0')}`; }
export function addDays(date: string, delta: number): string { const d = new Date(`${date}T12:00:00`); d.setDate(d.getDate()+delta); return dateKey(d); }
function validDate(date: string): boolean { return /^\d{4}-\d{2}-\d{2}$/.test(date) && Number(date.slice(0,4)) >= 2000 && dateKey(new Date(`${date}T12:00:00`)) === date; }
export function validateCalendarEvent(e: CalendarEvent): void {
  if (!e.title.trim() || [...e.title].length > 80 || [...e.location].length > 120) throw new Error('日程名称请填写 1–80 个字，地点最多 120 个字。');
  if (!validDate(e.date) || !validDate(e.endDate) || e.endDate < e.date || (!e.allDay && (!/^([01]\d|2[0-3]):[0-5]\d$/.test(e.startTime) || !/^([01]\d|2[0-3]):[0-5]\d$/.test(e.endTime) || (e.date === e.endDate && e.endTime <= e.startTime)))) throw new Error('请选择有效日期，结束时间须晚于开始时间。');
  if (!/^#[\da-f]{6}$/i.test(e.color)) throw new Error('日程颜色无效。');
}
export function eventsForDay(events: CalendarEvent[], day: string): CalendarEvent[] {
  return events.filter(e => e.date <= day && e.endDate >= day).sort((a,b) => Number(b.allDay)-Number(a.allDay) || (a.date === day ? a.startTime : '00:00').localeCompare(b.date === day ? b.startTime : '00:00') || a.id-b.id);
}
export function minutes(time: string): number { const [h,m] = time.split(':').map(Number); return h*60+m; }
export function eventMinutes(event: CalendarEvent, day: string): [number, number] { return [event.date < day ? 0 : minutes(event.startTime), event.endDate > day ? 1440 : minutes(event.endTime)]; }
export interface CalendarEntry { key: string; date: string; title: string; detail: string; color: string; event?: CalendarEvent }
export function dayEntries(events: CalendarEvent[], holidays: HolidayDay[], settings: CalendarSettings, day: string): CalendarEntry[] {
  const holiday = holidays.find(h => h.date === day && (h.type === 'holiday' ? settings.showHolidays : settings.showWorkdays));
  let detail='全天';
  if (holiday) {
    const matching = new Set(holidays.filter(h=>h.name===holiday.name && h.type===holiday.type).map(h=>h.date));
    let start=day, end=day;
    while(matching.has(addDays(start,-1)))start=addDays(start,-1);
    while(matching.has(addDays(end,1)))end=addDays(end,1);
    const label=(date:string)=>`${Number(date.slice(5,7))}月${Number(date.slice(8))}日`;
    if(start!==end)detail=`${label(start)}至${label(end)}`;
  }
  return [...(holiday ? [{ key: `holiday-${day}`, date: day, title: `${holiday.name}（${holiday.type === 'holiday' ? '休' : '班'}）`, detail, color: holiday.type === 'holiday' ? '#e34f54' : '#ca8b30' }] : []), ...eventsForDay(events,day).map(event => ({ key: `${event.id}-${day}`, date: day, title: event.title, detail: event.allDay ? '全天' : `${event.date < day ? '00:00' : event.startTime}–${event.endDate > day ? '24:00' : event.endTime}`, color: event.color, event }))];
}
export function upcomingEntries(events: CalendarEvent[], holidays: HolidayDay[], settings: CalendarSettings, today: string, nowTime: string): CalendarEntry[] {
  const dates = new Set<string>(holidays.filter(h => h.date >= today).map(h => h.date));
  for (const event of events) {
    let day = event.date < today ? today : event.date;
    for (let i=0; day <= event.endDate && i<366; i++, day=addDays(day,1)) dates.add(day);
  }
  return [...dates].sort().flatMap(day => dayEntries(events,holidays,settings,day)).filter(entry => !entry.event || entry.event.allDay || entry.date !== today || entry.event.endDate > today || entry.event.endTime > nowTime);
}
// Overlapping events share columns; disjoint groups regain the full day width.
export function timelineEvents(events: CalendarEvent[], day: string, start: number, end: number) {
  const items = eventsForDay(events,day).filter(e => !e.allDay).map(event => {
    const [from,to] = eventMinutes(event,day);
    return { event, from: Math.max(start,from), to: Math.min(end,to), lane: 0, lanes: 1 };
  }).filter(e => e.to > e.from).sort((a,b) => a.from-b.from || b.to-a.to);
  let group: typeof items = []; let until = -1;
  const finish = () => { const lanes = Math.max(1,...group.map(e => e.lane+1)); group.forEach(e => e.lanes=lanes); group=[]; };
  for (const item of items) {
    if (item.from >= until) { finish(); until=-1; }
    let lane=0; while (group.some(e => e.lane===lane && e.to>item.from)) lane++;
    item.lane=lane; group.push(item); until=Math.max(until,item.to);
  }
  finish(); return items;
}
