import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import ts from 'typescript';
const source=readFileSync(new URL('../src/features/calendar/calendarEvents.ts',import.meta.url),'utf8');
const output=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext}}).outputText;
const {validateCalendarEvent,eventsForDay,timelineEvents,upcomingEntries}=await import(`data:text/javascript;base64,${Buffer.from(output).toString('base64')}`);
const event={id:1,title:'讨论',date:'2026-10-03',endDate:'2026-10-03',allDay:false,startTime:'09:00',endTime:'10:00',location:'',color:'#3b67b8'};
const settings={showHolidays:true,showWorkdays:false};
test('events reject invalid dates and reversed times but allow overnight spans',()=>{
  assert.throws(()=>validateCalendarEvent({...event,date:'2026-02-30'}));
  assert.throws(()=>validateCalendarEvent({...event,endTime:'08:00'}));
  assert.doesNotThrow(()=>validateCalendarEvent({...event,endDate:'2026-10-04',startTime:'23:00',endTime:'01:00'}));
});
test('all-day and multi-day events appear on inclusive dates in time order',()=>{
  const overnight={...event,id:3,date:'2026-10-02',endTime:'01:00'};
  const allDay={...event,id:2,allDay:true};
  assert.deepEqual(eventsForDay([event,allDay,overnight],'2026-10-03').map(e=>e.id),[2,3,1]);
  assert.equal(eventsForDay([overnight],'2026-10-04').length,0);
});
test('upcoming list excludes ended appointments and disabled workdays',()=>{
  const allDay={...event,id:2,allDay:true};
  const entries=upcomingEntries([event,allDay],[{date:event.date,name:'调休',type:'workday'}],settings,event.date,'10:00');
  assert.equal(entries.length,1);assert.equal(entries[0].event.id,2);
});
test('overlapping timeline events share lanes and separate groups recover width',()=>{
  const second={...event,id:2,startTime:'09:30',endTime:'10:30'};
  const third={...event,id:3,startTime:'11:00',endTime:'12:00'};
  const blocks=timelineEvents([event,second,third],event.date,540,720);
  assert.deepEqual(blocks.map(e=>[e.lane,e.lanes]),[[0,2],[1,2],[0,1]]);
  const overnight={...event,date:'2026-10-02',endTime:'01:00'};
  assert.deepEqual(timelineEvents([overnight],event.date,0,360).map(e=>[e.from,e.to]),[[0,60]]);
});
