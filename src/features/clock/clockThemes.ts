import type { ClockCity } from '../../shared/types';

export const defaultClockCities: ClockCity[] = [
  { name: '库比提诺', timeZone: 'America/Los_Angeles' },
  { name: '东京', timeZone: 'Asia/Tokyo' },
  { name: '悉尼', timeZone: 'Australia/Sydney' },
  { name: '巴黎', timeZone: 'Europe/Paris' },
];

export function clockZoneTime(now: Date, timeZone?: string) {
  const parts = new Intl.DateTimeFormat('en-GB', { timeZone, hour: 'numeric', minute: 'numeric', second: 'numeric', hourCycle: 'h23' }).formatToParts(now);
  const value = (type: string) => Number(parts.find(p => p.type === type)?.value);
  return { hour: value('hour'), minute: value('minute'), second: value('second') };
}
function offset(now: Date, timeZone?: string) {
  const value = new Intl.DateTimeFormat('en-US', { timeZone, timeZoneName: 'shortOffset' }).formatToParts(now).find(p => p.type === 'timeZoneName')?.value ?? 'GMT';
  const match = /GMT([+-])(\d+)(?::(\d+))?/.exec(value);
  return match ? (match[1] === '-' ? -1 : 1) * (Number(match[2]) * 60 + Number(match[3] ?? 0)) : 0;
}
export function clockCityTime(now: Date, city: ClockCity, localZone?: string) {
  const time = clockZoneTime(now, city.timeZone);
  const delta = (offset(now, city.timeZone) - offset(now, localZone)) / 60;
  const dayNumber = (zone?: string) => {
    const parts = new Intl.DateTimeFormat('en-GB', { timeZone: zone, year: 'numeric', month: 'numeric', day: 'numeric' }).formatToParts(now);
    const value = (type: string) => Number(parts.find(p => p.type === type)?.value);
    return Date.UTC(value('year'), value('month') - 1, value('day')) / 86400000;
  };
  const dayDelta = dayNumber(city.timeZone) - dayNumber(localZone);
  return { ...city, ...time, daylight: time.hour >= 6 && time.hour < 18, dayLabel: dayDelta < 0 ? '昨天' : dayDelta > 0 ? '明天' : '今天', difference: delta === 0 ? '同一时区' : `${delta > 0 ? '+' : '−'}${Math.abs(delta)}小时` };
}
