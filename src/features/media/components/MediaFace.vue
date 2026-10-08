<script setup lang="ts">
import { computed, ref, watch } from "vue";
import AppIcon from "../../../shared/ui/AppIcon.vue";
import type { MediaTheme, WidgetSize } from "../../../shared/types";
import { normalizeMediaTheme } from "../mediaThemes";
import { useArtworkPalette } from "../useArtworkPalette";
import { mediaPosition, mediaSource, mediaTime, safeArtwork, mediaSpectrumBands, type MediaAction, type MediaSession, type MediaSpectrum } from "../media";
const props = defineProps<{ session: MediaSession; spectrum?: MediaSpectrum; size: WidgetSize; theme?: MediaTheme; now: number; busy: boolean; drag?: string }>();
const theme = computed(() => normalizeMediaTheme(props.theme));
const emit = defineEmits<{ action: [input: MediaAction] }>();
const failedArtwork = ref<string | null>(null);
const artwork = computed(() => safeArtwork(props.session.artwork));
const paletteArtwork = computed(() => (theme.value === 'atmosphere' || theme.value === 'vinyl') && failedArtwork.value !== artwork.value ? artwork.value : null);
const palette = useArtworkPalette(paletteArtwork);
const paletteStyle = computed(() => ({ '--media-color-1': palette.value.first, '--media-color-2': palette.value.second, '--media-color-3': palette.value.third }));
const showSkip = computed(() => props.size !== 'small' || theme.value === 'vinyl' || theme.value === 'minimal');
const playing = computed(() => props.session.playbackStatus === "playing");
const bands = computed(() => mediaSpectrumBands(props.spectrum,props.session));
const spectrumHint = computed(() => !props.spectrum ? '示例频谱动画' : !playing.value ? '播放时显示当前播放器真实频谱' : props.spectrum.status === 'ready' && props.spectrum.sessionId === props.session.id ? '当前播放器真实频谱' : '暂时无法读取当前播放器频谱');
const position = computed(() => mediaPosition(props.session, props.now));
const canToggle = computed(() => playing.value ? props.session.controls.pause : props.session.controls.play);
const seeking = ref(false);
const seekPosition = ref(0);
const changingVolume = ref(false);
const volumeDraft = ref(0);
const volumeLevel = computed(() => changingVolume.value ? volumeDraft.value : Math.round(props.session.volume?.level ?? 0));
watch([() => props.session.id, () => props.session.title, () => props.session.durationMs], () => { seeking.value = false; changingVolume.value = false; });
watch(() => props.session.controls.seek, allowed => { if (!allowed) seeking.value = false; });
watch(() => props.session.volume, volume => { if (!volume) changingVolume.value = false; });
function act(action: MediaAction["action"], positionMs?: number) {
  emit("action", { sessionId: props.session.id, action, ...(positionMs === undefined ? {} : { positionMs }) });
}
function previewSeek(event: Event) { seeking.value = true; seekPosition.value = Number((event.target as HTMLInputElement).value); }
function seek(event: Event) {
  const value = Number((event.target as HTMLInputElement).value);
  seeking.value = false;
  if (props.session.controls.seek && !props.busy) act("seek", Math.round(value));
}
function previewVolume(event: Event) { changingVolume.value = true; volumeDraft.value = Number((event.target as HTMLInputElement).value); }
function changeVolume(event: Event) {
  const volumeLevel = Number((event.target as HTMLInputElement).value);
  changingVolume.value = false;
  if (props.session.volume && !props.busy) emit("action", { sessionId: props.session.id, action: "volume", volumeLevel });
}
function toggleMute() {
  if (props.session.volume && !props.busy) emit("action", { sessionId: props.session.id, action: "mute", muted: !props.session.volume.muted });
}
</script>
<template>
  <div class="media-face" :class="[`media-${size}`, `media-theme-${theme}`, { 'is-playing': playing, 'spectrum-demo': !spectrum }]" :style="paletteStyle">
    <div v-if="theme === 'vinyl'" class="media-vinyl-panel" aria-hidden="true" :data-tauri-drag-region="drag"></div>
    <span v-if="theme === 'vinyl'" class="media-tonearm" aria-hidden="true" :data-tauri-drag-region="drag"></span>
    <span v-if="theme === 'minimal'" class="media-playing-mark" :data-tooltip="spectrumHint" aria-hidden="true"><i v-for="(band,index) in bands" :key="index" :style="spectrum ? {transform:`scaleY(${.13+.87*band})`} : undefined"></i></span>
    <div class="media-artwork" :data-tauri-drag-region="drag">
      <img v-if="artwork && failedArtwork !== artwork" :src="artwork" alt="" draggable="false" @error="failedArtwork = artwork" />
      <div v-else class="media-artwork-placeholder"><span class="media-record"><AppIcon name="music" :size="size === 'large' ? 30 : 22" /></span></div>
    </div>
    <div class="media-details" :data-tauri-drag-region="drag">
      <span class="media-source" :data-tooltip="session.source">{{ mediaSource(session.source) || '媒体' }}</span>
      <strong class="media-title" :data-tooltip="session.title || '未提供标题'">{{ session.title || '未提供标题' }}</strong>
      <span v-if="size !== 'small' || theme !== 'default'" class="media-artist" :data-tooltip="session.artist || session.album">{{ session.artist || session.album || '未提供歌手信息' }}</span>
    </div>
    <div class="media-timeline">
      <template v-if="session.durationMs > 0">
        <input class="media-progress" type="range" aria-label="播放进度" :aria-valuetext="`${mediaTime(seeking ? seekPosition : position)}，共 ${mediaTime(session.durationMs)}`" min="0" :max="session.durationMs" step="1000" :value="seeking ? seekPosition : position" :disabled="busy || !session.controls.seek" :style="{ '--media-progress': `${100 * (seeking ? seekPosition : position) / session.durationMs}%` }" @input="previewSeek" @change="seek" @keydown.esc="seeking = false" />
        <div v-if="size !== 'small'" class="media-times"><span>{{ mediaTime(seeking ? seekPosition : position) }}</span><span>{{ mediaTime(session.durationMs) }}</span></div>
      </template>
      <span v-else class="media-live">未提供进度</span>
    </div>
    <div class="media-controls" role="group" aria-label="媒体播放控制">
      <button v-if="showSkip" type="button" class="media-skip" aria-label="上一首" data-tooltip="上一首" :disabled="busy || !session.controls.previous" @click="act('previous')"><AppIcon name="previous" :size="19" /></button>
      <button type="button" class="media-toggle" :aria-label="playing ? '暂停' : '播放'" :data-tooltip="canToggle ? (playing ? '暂停' : '播放') : '播放器未提供播放控制'" :disabled="busy || !canToggle" @click="act(playing ? 'pause' : 'play')"><AppIcon :name="playing ? 'pause' : 'play'" :size="size === 'small' ? 17 : 22" /></button>
      <button v-if="showSkip" type="button" class="media-skip" aria-label="下一首" data-tooltip="下一首" :disabled="busy || !session.controls.next" @click="act('next')"><AppIcon name="next" :size="19" /></button>
    </div>
    <div class="media-footer">
      <span v-if="size === 'medium'" class="media-status" :data-tauri-drag-region="drag">{{ playing ? '正在播放' : session.playbackStatus === 'paused' ? '已暂停' : '已停止' }}</span>
      <div class="media-volume" role="group" aria-label="播放器音量">
        <button type="button" class="media-mute" :aria-label="session.volume?.muted ? '取消静音' : '静音'" :aria-pressed="session.volume?.muted ?? false" :data-tooltip="session.volume ? (session.volume.muted ? '取消静音' : '静音') : '暂时无法读取播放器音量'" :disabled="busy || !session.volume" @click="toggleMute"><AppIcon :name="session.volume?.muted || volumeLevel === 0 ? 'volume-off' : 'volume'" :size="14" /></button>
        <input class="media-progress media-volume-slider" type="range" aria-label="播放器音量" :aria-valuetext="session.volume ? `${volumeLevel}%${session.volume.muted ? '，已静音' : ''}` : '音量不可用'" min="0" max="100" step="1" :value="volumeLevel" :disabled="busy || !session.volume" :style="{ '--media-progress': `${volumeLevel}%` }" @input="previewVolume" @change="changeVolume" @keydown.esc="changingVolume = false" />
        <output class="media-volume-value">{{ session.volume ? `${volumeLevel}%` : '—' }}</output>
      </div>
    </div>
  </div>
</template>
