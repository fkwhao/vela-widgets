import assert from "node:assert/strict";
import test from "node:test";
import {readFileSync} from "node:fs";
import ts from "typescript";
const source=readFileSync(new URL("../src/features/clock/clockThemes.ts",import.meta.url),"utf8");
const output=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext}}).outputText;
const {clockCityTime,clockZoneTime}=await import(`data:text/javascript;base64,${Buffer.from(output).toString("base64")}`);
test("world clocks follow local dates and daylight saving instead of fixed offsets",()=>{
  const paris={name:"巴黎",timeZone:"Europe/Paris"};
  assert.equal(clockCityTime(new Date("2026-01-03T12:00:00Z"),paris,"Asia/Shanghai").difference,"−7小时");
  assert.equal(clockCityTime(new Date("2026-07-03T12:00:00Z"),paris,"Asia/Shanghai").difference,"−6小时");
  const la=clockCityTime(new Date("2026-10-03T01:00:00Z"),{name:"洛杉矶",timeZone:"America/Los_Angeles"},"Asia/Shanghai");
  assert.equal(la.dayLabel,"昨天");assert.equal(la.daylight,false);
  assert.deepEqual(clockZoneTime(new Date("2026-10-03T00:00:05Z"),"Asia/Tokyo"),{hour:9,minute:0,second:5});
  const tokyo=clockCityTime(new Date("2026-10-03T15:30:00Z"),{name:"东京",timeZone:"Asia/Tokyo"},"Asia/Shanghai");
  assert.equal(tokyo.dayLabel,"明天");
});
