<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, type CSSProperties } from "vue";
const props = defineProps<{ manager: boolean }>();
const content = ref("");
const hint = ref<HTMLElement>();
const position = ref<CSSProperties>({ left: "0px", top: "0px", maxWidth: "320px", maxHeight: "160px", visibility: "hidden" });
const hintId = "vela-hover-hint";
let trigger: HTMLElement | null = null;
let timer: ReturnType<typeof setTimeout> | undefined;
let generation = 0;
let previousDescription: string | null = null;

function close(): void {
  generation++;
  if (timer) clearTimeout(timer);
  if (trigger) {
    if (previousDescription === null) trigger.removeAttribute("aria-describedby");
    else trigger.setAttribute("aria-describedby", previousDescription);
  }
  trigger = null;
  content.value = "";
}
function targetFor(target: EventTarget | null): HTMLElement | null {
  if (!(target instanceof Element)) return null;
  return target.closest<HTMLElement>("[data-tooltip], .widget-icon-button[aria-label]");
}
async function show(target: HTMLElement, ticket: number): Promise<void> {
  if (ticket !== generation || !target.isConnected) return;
  const text = target.getAttribute("data-tooltip") ?? target.getAttribute("aria-label") ?? "";
  if (!text.trim()) return;
  const wasVisible = Boolean(hint.value);
  content.value = text;
  const windowBounds = { left: 0, top: 0, right: innerWidth, bottom: innerHeight };
  const bounds = props.manager ? windowBounds : target.closest('.widget-window')?.getBoundingClientRect() ?? windowBounds;
  position.value = { ...position.value, maxWidth: `${Math.min(340, bounds.right - bounds.left - 20)}px`, maxHeight: `${bounds.bottom - bounds.top - 20}px`, visibility: wasVisible ? "visible" : "hidden" };
  await nextTick();
  if (ticket !== generation || !hint.value || !target.isConnected) return;
  const anchor = target.getBoundingClientRect();
  const box = hint.value.getBoundingClientRect();
  const left = Math.max(bounds.left + 10, Math.min(anchor.left + anchor.width / 2 - box.width / 2, bounds.right - box.width - 10));
  const above = anchor.top - box.height - 8;
  const top = Math.max(bounds.top + 10, Math.min(above >= bounds.top + 10 ? above : anchor.bottom + 8, bounds.bottom - box.height - 10));
  position.value = { ...position.value, left: `${left}px`, top: `${top}px`, visibility: "visible" };
  target.setAttribute("aria-describedby", [previousDescription, hintId].filter(Boolean).join(" "));
}
function open(target: HTMLElement | null, immediate = false): void {
  if (target === trigger) {
    if (immediate && target && !content.value) { if (timer) clearTimeout(timer); void show(target, generation); }
    return;
  }
  close();
  if (!target || !(target.getAttribute("data-tooltip") ?? target.getAttribute("aria-label"))?.trim()) return;
  trigger = target;
  previousDescription = target.getAttribute("aria-describedby");
  const ticket = generation;
  if (immediate) void show(target, ticket);
  else timer = setTimeout(() => void show(target, ticket), 350);
}
function pointerOver(event: PointerEvent): void { if (event.pointerType !== "touch") open(targetFor(event.target)); }
function pointerOut(event: PointerEvent): void {
  if (event.relatedTarget instanceof Node && trigger?.contains(event.relatedTarget)) return;
  if (!trigger?.contains(document.activeElement)) close();
}
function focusIn(event: FocusEvent): void { open(targetFor(event.target), true); }
function focusOut(event: FocusEvent): void { if (!(event.relatedTarget instanceof Node && trigger?.contains(event.relatedTarget))) close(); }
function keyDown(event: KeyboardEvent): void { if (event.key === "Escape") close(); }
function reposition(): void { if (trigger && content.value) void show(trigger, generation); }

onMounted(() => {
  document.addEventListener("pointerover", pointerOver);
  document.addEventListener("pointerout", pointerOut);
  document.addEventListener("focusin", focusIn);
  document.addEventListener("focusout", focusOut);
  document.addEventListener("keydown", keyDown);
  document.addEventListener("pointerdown", close);
  document.addEventListener("scroll", close, true);
  window.addEventListener("resize", reposition);
});
onUnmounted(() => {
  close();
  document.removeEventListener("pointerover", pointerOver);
  document.removeEventListener("pointerout", pointerOut);
  document.removeEventListener("focusin", focusIn);
  document.removeEventListener("focusout", focusOut);
  document.removeEventListener("keydown", keyDown);
  document.removeEventListener("pointerdown", close);
  document.removeEventListener("scroll", close, true);
  window.removeEventListener("resize", reposition);
});
</script>

<template>
  <Teleport to="body"><div v-if="content" :id="hintId" ref="hint" role="tooltip" class="vela-hover-hint" :class="{ 'widget-hint': !manager }" :style="position">{{ content }}</div></Teleport>
</template>
