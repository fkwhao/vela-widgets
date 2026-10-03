<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, watch } from "vue";
import DOMPurify from "dompurify";
import { markdown } from "../lib/markdown";
import { openNoteLink } from "../lib/backend";
import "katex/dist/katex.min.css";
const props = defineProps<{ text: string; noteId: number }>();
const root = ref<HTMLElement | null>(null);
const error = ref("");
const html = computed(() => DOMPurify.sanitize(markdown.render(props.text, { noteId: `note-${props.noteId}` }), { ADD_ATTR: ["data-mermaid", "data-image-url", "data-image-alt"], FORBID_TAGS: ["img", "style", "iframe", "form"] }));
let revision = 0;
let stopped = false;
watch(html, async () => {
  const current = ++revision;
  await nextTick();
  const diagrams = Array.from(root.value?.querySelectorAll<HTMLElement>("[data-mermaid]") ?? []);
  if (!diagrams.length) return;
  const { default: mermaid } = await import("mermaid");
  if (stopped || current !== revision) return;
  mermaid.initialize({ startOnLoad: false, securityLevel: "strict", suppressErrorRendering: true, htmlLabels: false, theme: document.documentElement.dataset.theme === "dark" ? "dark" : "default", flowchart: { htmlLabels: false }, maxTextSize: 20000 });
  for (const diagram of diagrams) {
    try {
      const { svg } = await mermaid.render(`diagram-${crypto.randomUUID()}`, decodeURIComponent(diagram.dataset.mermaid ?? ""));
      if (stopped || current !== revision) return;
      diagram.innerHTML = DOMPurify.sanitize(svg, { USE_PROFILES: { svg: true, svgFilters: true } });
    } catch {
      if (stopped || current !== revision) return;
      diagram.textContent = `图表语法有误\n${decodeURIComponent(diagram.dataset.mermaid ?? "")}`;
      diagram.classList.add("note-diagram-error");
    }
  }
}, { immediate: true });
onUnmounted(() => { stopped = true; revision++; });
async function click(event: MouseEvent) {
  const target = event.target as HTMLElement;
  const image = target.closest<HTMLElement>("[data-image-url]");
  if (image) {
    const node = document.createElement("img"); node.alt = image.dataset.imageAlt ?? ""; node.src = image.dataset.imageUrl!;
    node.referrerPolicy = "no-referrer"; node.loading = "lazy";
    node.onerror = () => { image.textContent = "图片加载失败，点击重试"; node.replaceWith(image); };
    image.replaceWith(node); return;
  }
  const anchor = target.closest<HTMLAnchorElement>("a");
  if (!anchor) return;
  event.preventDefault();
  const href = anchor.getAttribute("href") ?? "";
  if (href.startsWith("#")) { root.value?.querySelector<HTMLElement>(`[id="${CSS.escape(href.slice(1))}"]`)?.scrollIntoView({ block: "nearest" }); return; }
  try { await openNoteLink(href); error.value = ""; } catch { error.value = "链接未能打开。"; }
}
</script>
<template><div ref="root" class="note-markdown" @click="click"><div v-html="html"></div><p v-if="error" role="alert">{{ error }}</p></div></template>
