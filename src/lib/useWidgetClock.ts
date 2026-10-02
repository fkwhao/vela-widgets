import { onMounted, onUnmounted, ref, watch, type Ref } from "vue";
// Align to the next wall-clock boundary; resume from actual time after sleep or hiding.
export function useWidgetClock(seconds?: Ref<boolean>, daily = false) {
  const now = ref(new Date());
  let timer: ReturnType<typeof setTimeout> | undefined;
  function schedule() {
    if (timer) clearTimeout(timer);
    now.value = new Date();
    if (document.hidden) return;
    const interval = seconds?.value ? 1000 : 60000;
    const next = daily ? new Date(now.value.getFullYear(), now.value.getMonth(), now.value.getDate() + 1).getTime() : Math.floor(Date.now() / interval) * interval + interval;
    timer = setTimeout(schedule, Math.max(1, next - Date.now() + 25));
  }
  if (seconds) watch(seconds, schedule);
  onMounted(() => { schedule(); document.addEventListener("visibilitychange", schedule); window.addEventListener("focus", schedule); window.addEventListener("pageshow", schedule); });
  onUnmounted(() => { if (timer) clearTimeout(timer); document.removeEventListener("visibilitychange", schedule); window.removeEventListener("focus", schedule); window.removeEventListener("pageshow", schedule); });
  return now;
}
