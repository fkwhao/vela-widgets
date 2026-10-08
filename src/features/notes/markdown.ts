import MarkdownIt from "markdown-it";
import footnote from "markdown-it-footnote";
import taskLists from "markdown-it-task-lists";
import type { HLJSApi } from 'highlight.js';
let highlighter: HLJSApi | undefined;
let mathReady = false;
let mathLoading: Promise<void> | undefined;
let highlightLoading: Promise<void> | undefined;

export const markdown = new MarkdownIt({
  html: false, linkify: true, breaks: false,
  highlight(code, language) {
    if (language && highlighter?.getLanguage(language)) {
      try { return highlighter.highlight(code, { language, ignoreIllegals: true }).value; } catch { /* Render unknown syntax as plain code. */ }
    }
    return "";
  },
}).use(footnote).use(taskLists);

export async function renderNoteMarkdown(text: string, noteId: string): Promise<string> {
  const tokens = markdown.parse(text, {});
  const needMath = tokens.some(token => token.type === 'inline' && /\$|\\[([]/.test(token.content));
  const needHighlight = tokens.some(token => token.type === 'fence' && token.info.trim() && token.info.trim() !== 'mermaid');
  const pending: Promise<void>[] = [];
  if (needMath && !mathReady) {
    mathLoading ??= Promise.all([import('markdown-it-texmath'), import('katex')]).then(([{default:texmath}, {default:katex}]) => {
      markdown.use(texmath, {engine:katex, delimiters:['dollars','brackets'], katexOptions:{throwOnError:false,trust:false,strict:'ignore'}});
      mathReady = true;
    }).catch(reason => { mathLoading = undefined; throw reason; });
    pending.push(mathLoading);
  }
  if (needHighlight && !highlighter) {
    highlightLoading ??= import('highlight.js/lib/common').then(({default:hljs}) => { highlighter=hljs; })
      .catch(reason => { highlightLoading=undefined; throw reason; });
    pending.push(highlightLoading);
  }
  await Promise.all(pending);
  return markdown.render(text, {noteId});
}
const defaultFence = markdown.renderer.rules.fence!;
markdown.renderer.rules.fence = (tokens, index, options, env, renderer) => {
  if (tokens[index].info.trim() === "mermaid") return `<div class="note-diagram" data-mermaid="${markdown.utils.escapeHtml(encodeURIComponent(tokens[index].content))}"><span>正在绘制图表…</span></div>`;
  return defaultFence(tokens, index, options, env, renderer);
};
markdown.renderer.rules.image = (tokens, index) => {
  const token = tokens[index];
  const url = String(token.attrGet("src") ?? "");
  const alt = token.content || "图片";
  // Images are loaded only after an explicit click, so notes remain offline by default.
  if (!/^(https?:\/\/|data:image\/(png|jpeg|gif|webp);base64,)/i.test(url)) return `<span class="note-image-placeholder">${markdown.utils.escapeHtml(alt)}（不支持此图片路径）</span>`;
  return `<button class="note-image-load" type="button" data-image-url="${markdown.utils.escapeHtml(url)}" data-image-alt="${markdown.utils.escapeHtml(alt)}">加载图片 · ${markdown.utils.escapeHtml(alt)}</button>`;
};
markdown.renderer.rules.footnote_anchor_name = (tokens, index, _options, env) => `${env?.noteId ?? "note"}-${Number(tokens[index].meta?.id ?? 0) + 1}`;
export function noteTitle(text: string): string {
  const line = text.split(/\r?\n/).find(line => line.trim())?.trim() ?? "";
  if (!line) return "新便签";
  const tokens = markdown.parse(line, {});
  const inline = tokens.find(token => token.type === "inline");
  const title = inline?.children?.filter(token => ["text", "code_inline", "image", "math_inline"].includes(token.type)).map(token => token.content).join("") ?? line.replace(/^\s*(`{3,}|~{3,}|#{1,6}|>+|[-*+]\s|\d+[.)]\s)/, "");
  return (title.replace(/^\s*\[[ xX]\]\s*/, "").trim() || "新便签").slice(0, 80);
}
