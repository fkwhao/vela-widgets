import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import ts from "typescript";
const require = createRequire(import.meta.url);
async function sourceModule(path) {
  const source = readFileSync(new URL(path, import.meta.url), "utf8");
  let output = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText;
  output = output.replace(/from "([^".][^"]*)"/g, (_, name) => `from "${pathToFileURL(require.resolve(name)).href}"`);
  output = output.replace(/import\((["'])([^"'.][^"']*)\1\)/g, (_, quote, name) => `import("${pathToFileURL(require.resolve(name)).href}")`);
  return import(`data:text/javascript;base64,${Buffer.from(output).toString("base64")}`);
}
const { normalizeNotes, nextNoteId } = await sourceModule("../src/features/notes/notes.ts");
const { markdown, noteTitle, renderNoteMarkdown } = await sourceModule("../src/features/notes/markdown.ts");
test("legacy notes migrate without losing content or color", () => {
  const settings = normalizeNotes({ text: "# 工作\n旧内容", color: "#12abcd" }, 1000);
  assert.equal(settings.notes.length, 1); assert.equal(settings.notes[0].createdAt, 1000);
  assert.equal(settings.text, "# 工作\n旧内容"); assert.equal(settings.color, "#12abcd");
  assert.deepEqual(normalizeNotes(settings, 2000), settings);
});
test("expiry respects the original creation date, keeps permanent notes and never resurrects the last deletion", () => {
  const settings = normalizeNotes({ text: "旧", color: "#12abcd" }, 1000);
  settings.notes[0].deleteAfterHours = 24;
  settings.notes.push({ id: 2, text: "永久", color: "#3b67b8", createdAt: 1000, deleteAfterHours: null });
  assert.equal(normalizeNotes(settings, 1000 + 24 * 3600000 - 1).notes.length, 2);
  const expired = normalizeNotes(settings, 1000 + 24 * 3600000);
  assert.equal(expired.activeId, 2); assert.equal(expired.text, "永久");
  assert.equal(normalizeNotes({ ...expired, notes: [], activeId: 0 }).notes.length, 0);
  assert.ok(nextNoteId([{ id: Date.now() + 1000 }]) > Date.now() + 1000);
});
test("titles come from the first nonempty line and strip Markdown formatting", () => {
  assert.equal(noteTitle("\n# **工作** [计划](https://example.com)\n正文"), "工作 计划");
  assert.equal(noteTitle("- [x] 完成 `设计`\n正文"), "完成 设计");
  assert.equal(noteTitle("\n\n"), "新便签");
});

test("plain notes don't load optional engines; concurrent math and code requests keep their own output", async () => {
  const basic = await renderNoteMarkdown('一条 **普通便签**', 'plain');
  assert.ok(basic.includes('<strong>普通便签</strong>'));
  assert.equal(require.cache[require.resolve('katex')], undefined);
  assert.equal(require.cache[require.resolve('highlight.js/lib/common')], undefined);
  const [brackets, dollars, code] = await Promise.all([
    renderNoteMarkdown('\\[x+y\\]', 'brackets'),
    renderNoteMarkdown('$z^2$', 'dollars'),
    renderNoteMarkdown('```js\nconst answer = 42;\n```', 'code'),
  ]);
  assert.ok(brackets.includes('katex'));
  assert.ok(dollars.includes('katex'));
  assert.ok(code.includes('hljs-keyword'));
  assert.ok(!code.includes('katex'));
});
test("Markdown renders nested lists, tables, tasks, footnotes, formulas and highlighted code", async () => {
  const html = await renderNoteMarkdown("# 标题\n\n> 引用\n\n- 列表\n  - 子项\n- [x] 完成\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n~~删除~~ **加粗** *斜体*\n\n注释[^1]\n\n[^1]: 脚注\n\n$x^2$\n\n$$\nx^2+y^2=1\n$$\n\n```js\nconst x = 1;\n```", "n1");
  for (const fragment of ["<h1>", "<blockquote>", "<table>", 'type="checkbox"', "footnote", "katex", "hljs-keyword", "<s>", "<strong>", "<em>"]) assert.ok(html.includes(fragment), fragment);
  assert.ok((html.match(/<ul/g) ?? []).length >= 2);
});
test("unsafe HTML and links remain inert; images load only on explicit action", () => {
  const html = markdown.render('<script>alert(1)</script>\n\n[点我](javascript:alert(1))\n\n![图片](https://example.com/image.png)');
  assert.ok(html.includes("&lt;script&gt;")); assert.ok(!html.includes('href="javascript:'));
  assert.ok(!html.includes("<img")); assert.ok(html.includes("data-image-url"));
  const diagram = markdown.render("```mermaid\nflowchart TD\nA --> B\n```\n\n```unknown\n<script>\n```");
  assert.ok(diagram.includes("data-mermaid")); assert.ok(diagram.includes("&lt;script&gt;"));
  const encoded = markdown.render("```mermaid\nflowchart TD\nA --> B\n```").match(/data-mermaid="([^"]+)"/)[1];
  assert.ok(!encoded.includes("-->")); assert.ok(decodeURIComponent(encoded).includes("A --> B"));
});

test("global retention affects inherited notes while individual and legacy overrides survive", () => {
  const base = { activeId: 1, defaultDeleteAfterHours: 24, notes: [
    { id: 1, text: "继承", color: "#3b67b8", createdAt: 1000, deleteAfterHours: null },
    { id: 2, text: "永久", color: "#3b67b8", createdAt: 1000, deleteAfterHours: null, retentionOverride: true },
    { id: 3, text: "旧独立", color: "#3b67b8", createdAt: 1000, deleteAfterHours: 168 }
  ] };
  const before = normalizeNotes(base, 1000 + 24 * 3600000 - 1);
  assert.equal(before.notes[2].retentionOverride, true);
  const expired = normalizeNotes(before, 1000 + 24 * 3600000);
  assert.deepEqual(expired.notes.map(n => n.id), [2,3]);
  expired.defaultDeleteAfterHours = 1;
  assert.equal(normalizeNotes(expired, 1000 + 25 * 3600000).notes.length, 2);
  expired.notes[1].retentionOverride = false; expired.notes[1].deleteAfterHours = null;
  assert.deepEqual(normalizeNotes(expired, 1000 + 25 * 3600000).notes.map(n => n.id), [2]);
});
test("empty note collections retain the global default through reload", () => {
  const settings = normalizeNotes({ activeId: 0, notes: [], defaultDeleteAfterHours: 168 }, 1000);
  assert.equal(settings.notes.length, 0);
  assert.equal(normalizeNotes(settings, 2000).defaultDeleteAfterHours, 168);
});
