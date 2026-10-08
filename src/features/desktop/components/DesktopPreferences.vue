<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { applySnapshot, snapshot } from '../../../app/store';
import { setDesktopEditing, applyDesktopLayout, getDesktopBounds } from '../../../infrastructure/backend';
import { desktopTemplates, templateLayout, restoreLayout, widgetFootprint, type SavedLayout } from '../layout';
const busy=ref(false),error=ref('');
const enabledCount=computed(()=>Object.values(snapshot.value.settings.widgets).filter(w=>w.enabled).length);
const bounds=ref({width:1920,height:1080});
onMounted(()=>{void getDesktopBounds().then(value=>bounds.value=value).catch(()=>undefined);});
const previews=computed(()=>Object.fromEntries(desktopTemplates.map(t=>{
  try {return[t.id,templateLayout(t.id,bounds.value,snapshot.value.settings).map(p=>{const size=widgetFootprint(p.kind,p.size,snapshot.value.settings.media.theme);return{kind:p.kind,x:1+p.x/bounds.value.width*198,y:1+p.y/bounds.value.height*108,width:size.width/bounds.value.width*198,height:size.height/bounds.value.height*108,locked:snapshot.value.settings.widgets[p.kind].locked};})];}
  catch {return[t.id,[]];}
})));
async function run(action:()=>Promise<void>){if(busy.value)return;busy.value=true;error.value='';try{await action();}catch(reason){error.value=typeof reason==='string'?reason:reason instanceof Error?reason.message:'布局操作未完成，请重试。';}finally{busy.value=false;}}
async function choose(id:string){await run(async()=>{const bounds=await getDesktopBounds();applySnapshot(await applyDesktopLayout(templateLayout(id,bounds,snapshot.value.settings)));});}
async function restore(layout:SavedLayout){await run(async()=>{const bounds=await getDesktopBounds();applySnapshot(await applyDesktopLayout(restoreLayout(layout,snapshot.value.settings,bounds)));});}
function arrange(){void run(async()=>{applySnapshot(await setDesktopEditing(true));if(!window.__TAURI_INTERNALS__)location.search='?view=desktop';});}
</script>
<template>
  <p class="page-subtitle">为当前开启的 {{enabledCount}} 个组件调整桌面排布。</p>
  <div v-if="error" class="info-bar" role="alert">{{error}}</div>
  <section class="settings-card"><div class="card-text"><strong>编排桌面</strong><span>拖动组件标题调整位置，支持网格吸附和方向键微调</span></div><button class="button-primary desktop-open-editor" :disabled="busy" @click="arrange">开始编排</button></section>
  <h2 class="section-title">布局模板</h2>
  <div class="desktop-layout-options">
    <button v-for="template in desktopTemplates" :key="template.id" :disabled="busy || !enabledCount" @click="choose(template.id)">
      <svg viewBox="0 0 200 110" aria-hidden="true"><rect x="1" y="1" width="198" height="108" rx="8" class="layout-screen" /><rect v-for="rect in previews[template.id]" :key="rect.kind" :x="rect.x" :y="rect.y" :width="rect.width" :height="rect.height" rx="3" :class="{'layout-locked':rect.locked}" /></svg>
      <strong>{{template.label}}</strong><span>{{template.description}}</span>
    </button>
  </div>
  <p class="desktop-layout-help">模板只排列当前开启的组件，保留尺寸和内容，并避开已锁定组件。保存的布局也按当前开启的组件应用，新开启的组件会自动补排。{{!enabledCount?'请先在“我的组件”中开启组件。':''}}</p>
  <template v-if="snapshot.settings.desktop.savedLayouts.length"><h2 class="section-title">我的布局</h2><section class="settings-card" v-for="layout in snapshot.settings.desktop.savedLayouts" :key="layout.name"><div class="card-text"><strong>{{layout.name}}</strong><span>保存时 {{layout.placements.length}} 个组件</span></div><button class="button-secondary" :disabled="busy || !enabledCount" @click="restore(layout)">应用</button></section></template>
  <div class="settings-card"><div class="card-text"><strong>共享桌面窗口</strong><span>当前在主显示器工作区显示。置顶会作用于整张画布，组件锁定仍分别生效。</span></div></div>
</template>
<style scoped>
.desktop-open-editor {padding:7px 16px;border:0;border-radius:7px;background:var(--accent);color:var(--accent-contrast);font:inherit;font-size:12px;cursor:pointer;}
.desktop-layout-options {display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px;}
.desktop-layout-options>button {display:flex;flex-direction:column;gap:7px;padding:16px;border:1px solid var(--outline);border-radius:10px;background:var(--surface-soft);color:var(--text);text-align:left;cursor:pointer;}
.desktop-layout-options>button:hover {border-color:var(--accent);background:var(--surface);}
.desktop-layout-options>button:disabled {opacity:.55;cursor:default;}
.desktop-layout-options svg {width:100%;height:100px;fill:color-mix(in srgb,var(--accent) 40%,var(--surface));}
.desktop-layout-options .layout-screen {fill:var(--surface);stroke:var(--outline-strong);}
.desktop-layout-options .layout-locked {fill:var(--outline-strong);stroke:var(--muted);stroke-dasharray:2 2;stroke-width:.5;}
.desktop-layout-options strong {font-size:13px;font-weight:600;}.desktop-layout-options span,.desktop-layout-help {font-size:11px;color:var(--muted);line-height:19px;}
.desktop-layout-help {margin:14px 0 24px;}
</style>
