import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import ts from "typescript";
const source = readFileSync(new URL("../src/lib/holidays.ts", import.meta.url), "utf8");
const output = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText;
const { validateHolidayData, fetchHolidayUpdate, mergeHolidayData, normalizeHolidayCache, visibleHoliday, holidayUpdateDue } = await import(`data:text/javascript;base64,${Buffer.from(output).toString("base64")}`);
const bundled = JSON.parse(readFileSync(new URL("../data/holidays/china.json", import.meta.url), "utf8"));
const options = { showHolidays: true, showWorkdays: true, autoUpdate: false };
const mock = (documents, calls = []) => async url => { calls.push(url); const document = documents.shift(); return document instanceof Response ? document : new Response(JSON.stringify(document)); };
test("official 2026 schedule distinguishes holidays, adjusted workdays and ordinary weekends", () => {
  validateHolidayData(bundled);
  const days = bundled.years[0].days;
  assert.equal(days.filter(d => d.type === "holiday").length, 33);
  assert.deepEqual(days.filter(d => d.type === "workday").map(d => d.date), ["2026-01-04", "2026-02-14", "2026-02-28", "2026-05-09", "2026-09-20", "2026-10-10"]);
  assert.equal(days.find(d => d.date === "2026-10-03").name, "国庆节");
  assert.equal(days.find(d => d.date === "2026-10-11"), undefined);
  assert.equal(bundled.years.find(y => y.year === 2027), undefined);
});
test("holiday and adjusted-workday visibility are independent", () => {
  const rest = bundled.years[0].days.find(d => d.type === "holiday");
  const work = bundled.years[0].days.find(d => d.type === "workday");
  assert.equal(visibleHoliday(rest, {...options, showHolidays:false}), undefined);
  assert.equal(visibleHoliday(work, {...options, showHolidays:false}), work);
  assert.equal(visibleHoliday(work, {...options, showWorkdays:false}), undefined);
});
test("unchanged manifest never downloads the annual file", async () => {
  const calls = [];
  assert.equal(await fetchHolidayUpdate(bundled, mock([{schemaVersion:1,revision:bundled.revision}],calls)), null);
  assert.equal(calls.length, 1); assert.ok(calls[0].endsWith("manifest.json"));
});
test("newer manifest downloads and validates matching annual data", async () => {
  const next = structuredClone(bundled); next.revision++;
  const result = await fetchHolidayUpdate(bundled, mock([{schemaVersion:1,revision:next.revision},next]));
  assert.equal(result.revision, next.revision);
});
test("invalid dates, duplicate days and unofficial source URLs are rejected", () => {
  for (const mutate of [d => d.years[0].days[0].date="2026-02-29", d => d.years[0].days.push(d.years[0].days[0]), d => d.years[0].source="https://example.com/notice", d => d.years[0].days[0].type="weekend"]) {
    const next = structuredClone(bundled); mutate(next); assert.throws(() => validateHolidayData(next));
  }
});
test("partial publication and rollbacks never replace current data", async () => {
  await assert.rejects(fetchHolidayUpdate(bundled, mock([{schemaVersion:1,revision:bundled.revision+1},bundled])), /尚未同步/);
  await assert.rejects(fetchHolidayUpdate(bundled, mock([{schemaVersion:1,revision:bundled.revision-1}])), /较旧/);
  await assert.rejects(fetchHolidayUpdate(bundled, mock([new Response("missing",{status:404})])), /尚未发布/);
  await assert.rejects(fetchHolidayUpdate(bundled, mock([new Response("x".repeat(1_048_577))])), /过大/);
});
test("new years preserve older years offline and damaged cache falls back to bundle", () => {
  const next = structuredClone(bundled); next.revision++; next.years = [{year:2027,source:bundled.years[0].source,days:[{date:"2027-01-01",name:"元旦",type:"holiday"}]}];
  const merged = mergeHolidayData(bundled,next); assert.deepEqual(merged.years.map(y => y.year),[2026,2027]);
  assert.equal(normalizeHolidayCache({data:{},lastError:"offline"},bundled).data.revision,bundled.revision);
  assert.equal(normalizeHolidayCache({data:merged,lastError:"offline"},bundled).data.revision,next.revision);
});
test("failed attempts are throttled for 24 hours and manual checks can bypass timing", () => {
  assert.equal(holidayUpdateDue(null,0),true);
  assert.equal(holidayUpdateDue(10,86_400_009),false);
  assert.equal(holidayUpdateDue(10,86_400_010),true);
  assert.equal(holidayUpdateDue(20,10),true);
});
