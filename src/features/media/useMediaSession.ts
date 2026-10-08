import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { isNativeApp } from "../../infrastructure/backend";
import { snapshot } from "../../app/store";
import type { MediaAction, MediaSnapshot, MediaSpectrum } from "./media";

export function useMediaSession() {
  const native = isNativeApp();
  const media = ref<MediaSnapshot>({ revision: -1, status: native ? "disabled" : "unavailable", session: null, error: null });
  const spectrum = ref<MediaSpectrum>({revision:-1,sessionId:null,status:'disabled',bands:[0,0,0,0]});
  const busy = ref(false);
  const error = ref("");
  const now = ref(Date.now());
  const session = computed(() => media.value.session);
  let disposed = false;
  let unlisten: (() => void) | undefined;
  let unlistenSpectrum: (() => void) | undefined;
  let spectrumTimer: ReturnType<typeof setTimeout> | undefined;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let errorTimer: ReturnType<typeof setTimeout> | undefined;

  function apply(next: MediaSnapshot) {
    if (!disposed && next.revision >= media.value.revision) media.value = next;
  }
  function applySpectrum(next: MediaSpectrum) {
    if (disposed || next.revision < spectrum.value.revision) return;
    spectrum.value = next;
    if (spectrumTimer) clearTimeout(spectrumTimer);
    spectrumTimer = undefined;
    if (next.status === 'ready') spectrumTimer = setTimeout(() => {
      spectrum.value = {...spectrum.value,status:'unavailable',bands:[0,0,0,0]};
    }, 350);
  }
  async function refresh() {
    if (!native) return;
    try { apply(await invoke<MediaSnapshot>("get_media_snapshot")); }
    catch { if (!disposed) media.value = { ...media.value, status: "unavailable", session: null, error: "暂时无法读取媒体信息。" }; }
    try { applySpectrum(await invoke<MediaSpectrum>('get_media_spectrum')); }
    catch { if (!disposed) spectrum.value = {...spectrum.value,status:'unavailable',bands:[0,0,0,0]}; }
  }
  function tick() {
    if (timer) clearTimeout(timer);
    timer = undefined;
    if (disposed) return;
    now.value = Date.now();
    // Project progress locally in every layout. No system polling.
    if (document.hidden || session.value?.playbackStatus !== "playing" || session.value.durationMs <= 0) return;
    timer = setTimeout(tick, 1020 - Date.now() % 1000);
  }
  function resumed() { tick(); if (!document.hidden) void refresh(); }
  function showError(message: string) {
    error.value = message;
    if (errorTimer) clearTimeout(errorTimer);
    errorTimer = setTimeout(() => { error.value = ""; }, 5000);
  }
  async function act(input: MediaAction) {
    if (busy.value || !native) return;
    busy.value = true;
    error.value = "";
    try { await invoke("media_action", { input }); }
    catch (reason) { if (!disposed) showError(typeof reason === "string" ? reason : "操作未完成，请重试。"); }
    finally { busy.value = false; }
  }
  async function retry() {
    if (busy.value || !native) return;
    busy.value = true;
    try { await invoke("refresh_media"); }
    catch { showError("媒体服务暂时不可用，请重新启动 Vela。"); }
    finally { busy.value = false; }
  }

  watch([session, () => snapshot.value.settings.widgets.media.size], tick);
  onMounted(async () => {
    document.addEventListener("visibilitychange", resumed);
    window.addEventListener("focus", resumed);
    window.addEventListener("pageshow", resumed);
    if (native) {
      try {
        const stop = await listen<MediaSnapshot>("vela://media-updated", event => apply(event.payload));
        if (disposed) { stop(); return; }
        unlisten = stop;
        const stopSpectrum = await listen<MediaSpectrum>('vela://media-spectrum', event => applySpectrum(event.payload));
        if (disposed) { stopSpectrum(); return; }
        unlistenSpectrum = stopSpectrum;
        await refresh();
      } catch { if (!disposed) media.value = { ...media.value, status: "unavailable", error: "媒体服务暂时不可用。" }; }
    }
    tick();
  });
  onUnmounted(() => {
    disposed = true;
    unlisten?.();
    unlistenSpectrum?.();
    if (spectrumTimer) clearTimeout(spectrumTimer);
    if (timer) clearTimeout(timer);
    if (errorTimer) clearTimeout(errorTimer);
    document.removeEventListener("visibilitychange", resumed);
    window.removeEventListener("focus", resumed);
    window.removeEventListener("pageshow", resumed);
  });
  return { media, session, spectrum, busy, error, now, native, act, retry };
}
