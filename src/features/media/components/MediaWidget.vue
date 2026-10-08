<script setup lang="ts">
import WidgetFrame from "../../../shared/widgets/WidgetFrame.vue";
import AppIcon from "../../../shared/ui/AppIcon.vue";
import MediaFace from "./MediaFace.vue";
import { useMediaSession } from "../useMediaSession";
import "../media.css";
import { computed } from "vue";
import { snapshot } from "../../../app/store";
import { normalizeMediaTheme } from "../mediaThemes";
const theme = computed(() => normalizeMediaTheme(snapshot.value.settings.media?.theme));
const { media, session, spectrum, busy, error, now, native, act, retry } = useMediaSession();
</script>
<template>
  <WidgetFrame kind="media" :class="`media-theme-${theme}`" header-only drag-background v-slot="{ widget, drag }">
    <MediaFace v-if="session" :session="session" :spectrum="spectrum" :size="widget.size" :theme="theme" :now="now" :busy="busy" :drag="drag" @action="act" />
    <div v-else class="media-empty" :data-tauri-drag-region="drag">
      <span class="media-empty-icon"><AppIcon name="music" :size="28" /></span>
      <strong>正在播放</strong>
      <p v-if="!native">在桌面应用中查看当前媒体</p>
      <p v-else-if="media.status === 'unavailable'">{{ media.error || '暂时无法连接播放器' }}</p>
      <p v-else-if="media.status === 'disabled'">正在连接播放器…</p>
      <p v-else>播放一首喜欢的歌<br />或打开一个视频</p>
      <button v-if="native && media.status === 'unavailable'" type="button" class="media-retry" :disabled="busy" @click="retry">重新连接</button>
      <span v-else-if="native && widget.size !== 'small'" class="media-empty-hint">支持 Windows 系统媒体控制的播放器</span>
    </div>
    <p v-if="error" class="media-notice" role="alert">{{ error }}</p>
  </WidgetFrame>
</template>
