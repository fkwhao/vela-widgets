<script setup lang="ts">
import { computed, ref } from 'vue';
import ClockThemeFace from './ClockThemeFace.vue';
import AppIcon from '../../../shared/ui/AppIcon.vue';
import { clockThemes, type ClockTheme, type WidgetSize } from '../../../shared/types';
import { defaultClockCities } from '../clockThemes';
import { snapshot, setClockSettings } from '../../../app/store';
import { useWidgetClock } from '../../../shared/composables/useWidgetClock';
const now=useWidgetClock(ref(true));
const clock=computed(()=>snapshot.value.settings.clock);
const size=ref<WidgetSize>('small'),busy=ref(false),error=ref('');
async function choose(theme:ClockTheme) {
  if(busy.value)return;
  busy.value=true;error.value='';
  try {await setClockSettings({...clock.value,theme,cities:theme==='world' && !clock.value.cities.length ? [...defaultClockCities] : clock.value.cities});}
  catch(e){error.value=typeof e==='string'?e:'主题没有保存成功。';}
  finally{busy.value=false;}
}
</script>
<template>
  <h2 class="section-title clock-theme-heading">时钟主题<button class="holiday-info-button" aria-label="时钟主题说明" data-tooltip="点击主题后自动保存。预览中的秒针与桌面一样实时走动。&#10;预览尺寸只改变这里的展示，主题只用于时钟模式。"><AppIcon name="info" :size="16" /></button></h2>
  <section class="settings-card stacked clock-theme-settings" aria-label="时钟主题选择">
    <div class="clock-theme-settings-header"><div class="card-text"><strong>选择表盘</strong><span>实时动画预览</span></div><nav class="clock-theme-preview-sizes" aria-label="主题预览尺寸"><button v-for="option in [{value:'small',label:'小'},{value:'medium',label:'中'}]" :key="option.value" :aria-pressed="size===option.value" @click="size=option.value as WidgetSize">{{ option.label }}</button></nav></div>
    <div class="clock-theme-options" :class="{ 'wide-previews':size==='medium' }"><button v-for="theme in clockThemes" :key="theme.value" class="clock-theme-option" :aria-label="theme.value==='default' ? '选择默认主题' : `选择${theme.label}主题`" :aria-pressed="clock.theme===theme.value" :disabled="busy" @click="choose(theme.value)"><div class="clock-theme-preview" :class="`preview-${size}`"><ClockThemeFace :now="now" :settings="clock" :theme="theme.value" :size="size" /></div><span class="clock-theme-option-label">{{ theme.label }}<AppIcon v-if="clock.theme===theme.value" name="tick" :size="14" /></span></button></div>
    <p v-if="error" class="info-bar" role="alert">{{ error }}</p>
  </section>
</template>
