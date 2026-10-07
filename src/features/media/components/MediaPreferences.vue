<script setup lang="ts">
import { computed, ref } from "vue";
import AppIcon from "../../../shared/ui/AppIcon.vue";
import { snapshot, setMediaTheme } from "../../../app/store";
import { widgetSizeOptions, type MediaTheme, type WidgetSize } from "../../../shared/types";
import { mediaThemes, mediaWidgetDimensions } from "../mediaThemes";
import { createMediaPreviewSession } from "../mediaPreview";
import MediaFace from "./MediaFace.vue";
import "../media.css";

const selected = computed(() => snapshot.value.settings.media?.theme ?? "default");
const size = ref<WidgetSize>("medium");
const busy = ref(false), error = ref("");
const session = createMediaPreviewSession();
function previewStyle(theme: MediaTheme) {
  const dimensions = mediaWidgetDimensions(size.value, theme);
  const scale = size.value === "small" ? .9 : size.value === "medium" ? .65 : .55;
  return { width: `${dimensions.width * scale}px`, height: `${dimensions.height * scale}px` };
}
function widgetStyle(theme: MediaTheme) {
  const dimensions = mediaWidgetDimensions(size.value, theme);
  return { width: `${dimensions.width}px`, height: `${dimensions.height}px` };
}
async function choose(theme: MediaTheme) {
  if (busy.value || selected.value === theme) return;
  busy.value = true; error.value = "";
  try { await setMediaTheme(theme); }
  catch (reason) { error.value = typeof reason === "string" ? reason : "主题没有保存成功，请重试。"; }
  finally { busy.value = false; }
}
</script>
<template>
  <h2 class="section-title">播放器主题</h2>
  <section class="settings-card stacked media-theme-settings" aria-label="播放器主题选择">
    <div class="media-theme-settings-header">
      <div class="card-text"><strong>选一个喜欢的款式</strong><span>点击即保存，封面卡片的中号为竖向布局</span></div>
      <nav class="media-preview-sizes" aria-label="播放器主题预览尺寸">
        <button v-for="option in widgetSizeOptions" :key="option.value" type="button" :aria-pressed="size === option.value" @click="size = option.value">{{ option.label }}</button>
      </nav>
    </div>
    <div class="media-theme-options" :class="`previews-${size}`">
      <article v-for="theme in mediaThemes" :key="theme.value" class="media-theme-option" :class="{ selected: selected === theme.value }">
        <button type="button" class="media-theme-select" :aria-label="`选择${theme.label}主题`" :aria-pressed="selected === theme.value" :disabled="busy" @click="choose(theme.value)"></button>
        <span class="media-theme-preview" :class="`preview-${size}`" :style="previewStyle(theme.value)" aria-hidden="true" inert>
          <span class="widget-window media-window" :class="[`size-${size}`, `media-theme-${theme.value}`]" :style="widgetStyle(theme.value)">
            <MediaFace :session="session" :theme="theme.value" :size="size" :now="0" :busy="false" />
          </span>
        </span>
        <span class="media-theme-option-label">{{ theme.label }}<AppIcon v-if="selected === theme.value" name="tick" :size="15" /></span>
        <span class="media-theme-option-description">{{ theme.description }}</span>
      </article>
    </div>
    <p class="media-preview-caption">示例曲目仅用于展示外观，桌面组件显示当前播放器的内容。</p>
    <p v-if="error" class="info-bar" role="alert">{{ error }}</p>
  </section>
  <div class="settings-card media-support-note"><AppIcon name="info" :size="18" /><div class="card-text"><strong>跟随系统正在播放的媒体</strong><span>封面、曲目信息和控制由播放器提供。当前暂不支持歌词；播放器未提供的进度、音量或切歌按钮会显示为不可用。</span></div></div>
</template>
