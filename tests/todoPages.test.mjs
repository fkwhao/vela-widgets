import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import ts from "typescript";
const source = readFileSync(new URL("../src/lib/todoPages.ts", import.meta.url), "utf8");
const output = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText;
const { paginateTodos } = await import(`data:text/javascript;base64,${Buffer.from(output).toString("base64")}`);
const items = Array.from({length: 11}, (_,i) => ({id:i+1, title:`任务 ${i+1}`, dueDate: null, completed:false, createdAt:""}));
test("compact pages preserve every task in order and fill the preset", () => {
  for (const [size,count] of [["small",3],["medium",4]]) {
    const pages = paginateTodos(items,size); assert.equal(pages[0].length,count); assert.deepEqual(pages.flat(),items); assert.ok(pages.every(page => page.length <= count));
  }
});
test("large pages allow room for dates and inline editing without losing tasks", () => {
  const withDates = items.map(item => ({...item,dueDate:"2026-10-03"}));
  const pages = paginateTodos(withDates,"large",3); assert.deepEqual(pages.flat(),withDates);
  const editedPage = pages.find(page => page.some(item => item.id === 3)); assert.ok(editedPage.length <= 3);
  assert.equal(paginateTodos(items,"large")[0].length,5);
});
test("empty and shrinking task sets always have a valid page", () => {
  assert.deepEqual(paginateTodos([],"small"),[[]]); assert.equal(paginateTodos(items.slice(0,1),"small").length,1);
});

test("inline composer reserves list space without losing tasks", () => {
  for (const [size, reserved, capacity] of [["small",48,1],["medium",48,2],["large",68,3]]) {
    const pages = paginateTodos(items,size,null,reserved);
    assert.equal(pages[0].length,capacity);
    assert.deepEqual(pages.flat(),items);
  }
});
