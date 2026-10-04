import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import ts from 'typescript';
const source = readFileSync(new URL('../src/features/habits/habits.ts', import.meta.url), 'utf8');
const output = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText;
const { annualWeeklyCheckins, dateKey, shiftDate, validDate, selectedHabits, todayHabits, nextIncomplete, habitStats, isComplete, weekDates, monthDates, parseHabitResult, validateHabit, validateRecord, cardForeground, tickHabitReminders } = await import(`data:text/javascript;base64,${Buffer.from(output).toString('base64')}`);
const habit = (overrides = {}) => ({ id: 1, title: '喝水', encouragement: '', icon: '💧', color: '#444fb0', startDate: '2026-10-01', weekdays: [0,1,2,3,4,5,6], dailyTarget: 3, goalDays: null, archived: false, sortOrder: 0, trackMood: false, trackRating: false, trackResult: false, ...overrides });
const record = (date, overrides = {}) => ({ habitId: 1, date, count: 3, target: 3, updatedAt: '', mood: null, rating: null, result: null, ...overrides });

test('numeric result inputs preserve zero and decimals, and empty inputs clear the result', () => {
  for (const value of ['', '   ']) assert.equal(parseHabitResult(value), null);
  for (const value of [0, '0']) assert.equal(parseHabitResult(value), 0);
  for (const value of [12.5, '12.5', ' 12.5 ']) assert.equal(parseHabitResult(value), 12.5);
  assert.equal(parseHabitResult(-2.5), -2.5);
  for (const value of [NaN, Infinity, 'invalid', 1e9 + 1]) assert.throws(() => parseHabitResult(value), /请输入有效成绩/);
});
test('small card advances only at the recorded target, and returns after undo or midnight', () => {
  const list = [habit(), habit({ id: 2 })];
  assert.equal(nextIncomplete(list, [record('2026-10-04', { count: 2 })], '2026-10-04').id, 1);
  assert.equal(nextIncomplete(list, [record('2026-10-04')], '2026-10-04').id, 2);
  const done = [record('2026-10-04'), record('2026-10-04', { habitId: 2 })];
  assert.equal(nextIncomplete(list, done, '2026-10-04'), undefined);
  assert.equal(nextIncomplete(list, done, '2026-10-05').id, 1);
  assert.equal(nextIncomplete(list, [record('2026-10-04', { count: 2 }), done[1]], '2026-10-04').id, 1);
});
test('widget selection distinguishes all from none, skips paused, future and rest habits', () => {
  const list = [habit({ id: 1, sortOrder: 3 }), habit({ id: 2, sortOrder: 1 }), habit({ id: 3, archived: true }), habit({ id: 4, startDate: '2026-10-06' }), habit({ id: 5, weekdays: [1] })];
  const settings = { style: 'card', selectedIds: null };
  assert.deepEqual(todayHabits(list, settings, '2026-10-04').map(h => h.id), [2,1]);
  assert.deepEqual(selectedHabits(list, { ...settings, selectedIds: [] }), []);
  assert.deepEqual(todayHabits(list, { ...settings, selectedIds: [1,3,4] }, '2026-10-04').map(h => h.id), [1]);
});
test('statistics count completed days rather than taps, use stored targets and break at missed scheduled days', () => {
  const rows = [record('2026-10-01'), record('2026-10-02', { count: 1 }), record('2026-10-03', { target: 2, count: 2 }), record('2026-10-04', { count: 1 }), record('2026-10-05')];
  assert.equal(isComplete(rows[2]), true);
  assert.deepEqual(habitStats(habit({ dailyTarget: 5 }), rows, '2026-10-04'), { total: 2, month: 2, current: 1, longest: 1, checkins: 7 });
});
test('scheduled streaks bridge weekends, month and year boundaries', () => {
  const h = habit({ startDate: '2025-12-31', weekdays: [1,2,3,4,5] });
  const rows = ['2025-12-31', '2026-01-01', '2026-01-02', '2026-01-05'].map(d => record(d));
  assert.equal(habitStats(h, rows, '2026-01-05').current, 4);
  assert.equal(habitStats(h, rows, '2026-01-06').current, 4);
  assert.equal(habitStats(h, rows, '2026-01-07').current, 0);
  assert.equal(habitStats(h, rows, '2026-01-07').longest, 4);
  assert.equal(habitStats(h, rows, '2026-01-07').month, 3);
});
test('calendar computations are local-day based and support Sunday/Monday and leap years', () => {
  assert.deepEqual(weekDates('2026-10-04', true), ['2026-09-28','2026-09-29','2026-09-30','2026-10-01','2026-10-02','2026-10-03','2026-10-04']);
  assert.equal(weekDates('2026-10-04', false)[0], '2026-10-04');
  assert.equal(monthDates('2024-02-01').length, 29);
  assert.equal(monthDates('2025-02-01').length, 28);
  assert.equal(validDate('2026-02-30'), false);
  const old = process.env.TZ; process.env.TZ = 'America/New_York';
  try { assert.equal(shiftDate('2026-03-08', 1), '2026-03-09'); assert.equal(dateKey(new Date(2026, 9, 4, 23, 59)), '2026-10-04'); }
  finally { if (old === undefined) delete process.env.TZ; else process.env.TZ = old; }
});
test('invalid targets, plans, dates, ratings and future checkins are rejected', () => {
  assert.equal(validateHabit(habit()), null);
  for (const patch of [{ title: '' }, { weekdays: [] }, { weekdays: [7] }, { dailyTarget: 0 }, { dailyTarget: 1.5 }, { startDate: '2026-02-30' }, { goalDays: 0 }, { color: '#bad' }]) assert.ok(validateHabit(habit(patch)));
  const input = { date: '2026-10-04', delta: 1 };
  assert.equal(validateRecord(habit(), input, '2026-10-04'), null);
  assert.ok(validateRecord(habit(), { ...input, date: '2026-10-05' }, '2026-10-04'));
  assert.ok(validateRecord(habit(), { ...input, rating: 6 }, '2026-10-04'));
  assert.ok(validateRecord(habit(), { ...input, result: Infinity }, '2026-10-04'));
  assert.equal(validateRecord(habit({ weekdays: [1], startDate: '2026-10-06' }), input, '2026-10-04', true), null);
  assert.equal(cardForeground('#444fb0'), '#fff'); assert.equal(cardForeground('#ffffff'), '#171717');
});
test('opt-in reminders fire once in the scheduled minute, skip completed and paused habits, and persist deduplication', () => {
  const h = habit({ reminderTime: '20:00', lastRemindedDate: null });
  const tools = { alerts: [] };
  assert.equal(tickHabitReminders([h], [], tools, new Date(2026,9,4,19,59)), false);
  assert.equal(tickHabitReminders([h], [], tools, new Date(2026,9,4,20,0)), true);
  assert.equal(tickHabitReminders([h], [], tools, new Date(2026,9,4,20,0,30)), false);
  assert.equal(tools.alerts.length, 1);
  assert.equal(tickHabitReminders([JSON.parse(JSON.stringify(h))], [], tools, new Date(2026,9,4,20,0)), false);
  assert.equal(tickHabitReminders([h], [record('2026-10-05')], tools, new Date(2026,9,5,20,0)), false);
  assert.equal(tickHabitReminders([{ ...h, archived: true }], [], tools, new Date(2026,9,5,20,0)), false);
  assert.equal(tickHabitReminders([h], [], tools, new Date(2026,9,5,20,0)), true);
  assert.ok(validateHabit(habit({ reminderTime: '25:00' })));
});


test('annual weekly chart sums taps, includes partial days and keeps year boundaries separate', () => {
  const rows = [record('2025-12-31'), record('2026-01-01', { count: 2 }), record('2026-01-04', { count: 1 }), record('2026-01-05'), record('2026-01-06')];
  const monday = annualWeeklyCheckins(rows, 2026, true, '2026-01-05');
  assert.deepEqual(monday, [{ start: '2025-12-29', index: 1, count: 3 }, { start: '2026-01-05', index: 2, count: 3 }]);
  assert.deepEqual(annualWeeklyCheckins(rows, 2026, false, '2026-01-05').map(w => w.count), [2, 4]);
  assert.equal(annualWeeklyCheckins([], 2024, true, '2026-10-04').length, 53);
});
