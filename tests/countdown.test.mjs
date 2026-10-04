import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import ts from "typescript";
const source = readFileSync(new URL("../src/features/countdown/countdown.ts", import.meta.url), "utf8");
const output = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText;
const { countdownInfo, sortedCountdowns } = await import(`data:text/javascript;base64,${Buffer.from(output).toString("base64")}`);
const event = (overrides = {}) => ({ id: 1, title: "生日", date: "2024-02-29", yearly: true, countUp: false, createdDate: "2024-01-01", ...overrides });
test("Feb 29 anniversaries use Feb 28 and advance after the anniversary", () => {
  assert.equal(countdownInfo(event(), new Date(2025,1,27)).days, 1);
  assert.equal(countdownInfo(event(), new Date(2025,1,28)).signed, 0);
  assert.equal(countdownInfo(event(), new Date(2025,2,1)).target.getFullYear(), 2026);
});
test("count-up uses the most recent annual occurrence", () => {
  const info = countdownInfo(event({ date: "2020-10-20", countUp: true }), new Date(2026,9,2));
  assert.equal(info.target.getFullYear(), 2025); assert.equal(info.caption, "已经过去");
  assert.ok(info.progress > 90 && info.progress < 100);
});
test("calendar day differences do not depend on daylight saving hours", () => {
  const previousZone = process.env.TZ;
  process.env.TZ = "America/New_York";
  try {
    assert.notEqual(new Date(2026,2,7).getTimezoneOffset(), new Date(2026,2,9).getTimezoneOffset());
    assert.equal(countdownInfo(event({ date: "2026-03-09", yearly: false }), new Date(2026,2,7)).days, 2);
  } finally { if (previousZone === undefined) delete process.env.TZ; else process.env.TZ = previousZone; }
  assert.equal(countdownInfo(event({ date: "2026-10-02", yearly: false }), new Date(2026,9,2,23,59)).days, 0);
});
test("nearest event is first, and passed events remain visible", () => {
  const items = [event({ id: 1, date: "2026-12-01", yearly: false }), event({ id: 2, date: "2026-10-03", yearly: false }), event({ id: 3, date: "2026-10-01", yearly: false })];
  const list = sortedCountdowns(items, new Date(2026,9,2)); assert.deepEqual(list.map(c => c.item.id), [2,3,1]); assert.equal(list[1].caption, "已经过去");
});
