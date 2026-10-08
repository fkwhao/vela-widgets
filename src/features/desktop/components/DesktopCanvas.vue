<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, provide, ref, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { snapshot, ready, applySnapshot, saveWidgetPosition, setWidgetLocked, setWidgetSize } from '../../../app/store';
import { isNativeApp, openManager, setDesktopEditing, applyDesktopLayout, saveDesktopLayout } from '../../../infrastructure/backend';
import { desktopCanvasKey } from '../context';
import { clampPosition, snapPosition, templateLayout, restoreLayout, visibleLayout, widgetFootprint, desktopTemplates, type Placement, type SavedLayout } from '../layout';
import { widgetRegistry, widgetSizeOptions, type WidgetKind, type WidgetSize } from '../../../shared/types';
import AppIcon from '../../../shared/ui/AppIcon.vue';
import { widgetComponents as components } from '../../../shared/widgets/widgetComponents';
import './desktop.css';

provide(desktopCanvasKey, ref(true));
const bounds=ref({width:innerWidth,height:innerHeight});
const editing=computed(()=>snapshot.value.settings.desktop.editing);
const placements=computed(()=>visibleLayout(snapshot.value.settings,bounds.value));
const drafts=ref<Partial<Record<WidgetKind,{x:number;y:number}>>>({});
const selected=ref<WidgetKind|null>(null),templatesOpen=ref(false),name=ref(''),saving=ref(false),error=ref('');
const saveOpen=ref(false);
const toolbarBottom=ref(true);
let drag:{kind:WidgetKind;id:number;startX:number;startY:number;x:number;y:number;target:HTMLElement}|null=null;
let observer:MutationObserver|undefined,resizeObserver:ResizeObserver|undefined,timer:ReturnType<typeof setTimeout>|undefined;
let regionTimer:ReturnType<typeof setTimeout>|undefined,lastRegion='',disposed=false,regionBusy=false;
function resize(){bounds.value={width:innerWidth,height:innerHeight};scheduleRegions();}
function position(p:Placement){return drafts.value[p.kind]??p;}
function style(p:Placement){const size=widgetFootprint(p.kind,p.size,snapshot.value.settings.media.theme),point=position(p);return{left:`${point.x}px`,top:`${point.y}px`,width:`${size.width}px`,height:`${size.height}px`,zIndex:selected.value===p.kind?2:1};}
function notice(reason:unknown){error.value=typeof reason==='string'?reason:reason instanceof Error?reason.message:'操作未完成，请重试。';if(timer)clearTimeout(timer);timer=setTimeout(()=>error.value='',6000);}
async function run(action:()=>Promise<unknown>){if(saving.value)return;saving.value=true;try{await action();}catch(reason){notice(reason);}finally{saving.value=false;}}
async function mode(value:boolean){await run(async()=>{applySnapshot(await setDesktopEditing(value));templatesOpen.value=false;saveOpen.value=false;});}
function others(kind:WidgetKind){return placements.value.filter(p=>p.kind!==kind).map(p=>({...position(p),...widgetFootprint(p.kind,p.size,snapshot.value.settings.media.theme)}));}
function startDrag(event:PointerEvent,p:Placement){
  if(!editing.value||snapshot.value.settings.widgets[p.kind].locked||saving.value||event.button!==0)return;
  const target=event.currentTarget as HTMLElement,point=position(p);
  target.setPointerCapture(event.pointerId);event.preventDefault();selected.value=p.kind;
  drag={kind:p.kind,id:event.pointerId,startX:event.clientX,startY:event.clientY,x:point.x,y:point.y,target};
}
function moveDrag(event:PointerEvent){
  if(!drag||drag.id!==event.pointerId)return;
  const widget=snapshot.value.settings.widgets[drag.kind],footprint=widgetFootprint(drag.kind,widget.size,snapshot.value.settings.media.theme);
  drafts.value={...drafts.value,[drag.kind]:snapPosition(drag.x+event.clientX-drag.startX,drag.y+event.clientY-drag.startY,footprint.width,footprint.height,bounds.value,others(drag.kind),!event.altKey)};
  scheduleRegions();
}
async function stopDrag(event:PointerEvent,cancel=false){
  if(!drag||drag.id!==event.pointerId)return;
  const {kind,target,id}=drag;drag=null;
  if(target.hasPointerCapture(id))target.releasePointerCapture(id);
  const point=drafts.value[kind];
  if(point&&!cancel)await run(()=>saveWidgetPosition(kind,point));
  delete drafts.value[kind];scheduleRegions();
}
async function keyboard(event:KeyboardEvent,p:Placement){
  if(!editing.value||event.target!==event.currentTarget||snapshot.value.settings.widgets[p.kind].locked)return;
  const moves:Record<string,[number,number]>={ArrowLeft:[-1,0],ArrowRight:[1,0],ArrowUp:[0,-1],ArrowDown:[0,1]};
  const delta=moves[event.key];if(!delta)return;event.preventDefault();
  const point=position(p),size=widgetFootprint(p.kind,p.size,snapshot.value.settings.media.theme),step=event.shiftKey?16:1;
  await run(()=>saveWidgetPosition(p.kind,clampPosition(point.x+delta[0]*step,point.y+delta[1]*step,size.width,size.height,bounds.value)));
}
async function template(id:string){await run(async()=>{applySnapshot(await applyDesktopLayout(templateLayout(id,bounds.value,snapshot.value.settings)));templatesOpen.value=false;});}
async function restore(layout:SavedLayout){await run(async()=>{
  const placements=restoreLayout(layout,snapshot.value.settings,bounds.value);
  applySnapshot(await applyDesktopLayout(placements));templatesOpen.value=false;
});}
async function save(){await run(async()=>{const title=name.value.trim();if(!title)throw '请填写布局名称。';applySnapshot(await saveDesktopLayout({name:title,...bounds.value,placements:placements.value.map(p=>({...p}))}));name.value='';saveOpen.value=false;});}
function scheduleRegions(){if(disposed||regionTimer)return;regionTimer=setTimeout(()=>{regionTimer=undefined;void updateRegions();},40);}
async function updateRegions(){
  if(!isNativeApp()||!ready.value||disposed)return;
  if(regionBusy){scheduleRegions();return;}
  const nodes=Array.from(document.querySelectorAll<HTMLElement>('.desktop-widget,[data-canvas-overlay],.widget-context-menu,[data-reka-popper-content-wrapper],.vela-hover-hint'));
  const rects=nodes.flatMap(node=>{const r=node.getBoundingClientRect(),style=getComputedStyle(node);if(!r.width||!r.height||style.visibility==='hidden'||style.display==='none')return[];const widget=node.classList.contains('desktop-widget');return[{x:r.left,y:r.top,width:r.width,height:r.height,radius:widget?snapshot.value.settings.widgetCornerRadius:8,backdrop:widget}];});
  const signature=JSON.stringify([rects,editing.value,snapshot.value.settings.widgetTransparency,bounds.value]);if(signature===lastRegion)return;
  regionBusy=true;
  try{await invoke('update_desktop_regions',{rects,editing:editing.value});lastRegion=signature;}
  catch(reason){notice(reason);}
  finally{regionBusy=false;if(!disposed)scheduleRegions();}
}
function escape(event:KeyboardEvent){if(event.key==='Escape'&&editing.value&&!drag){event.preventDefault();void mode(false);}}
watch(()=>snapshot.value.settings,async()=>{await nextTick();scheduleRegions();},{deep:true});
watch(ready,()=>scheduleRegions());
onMounted(()=>{
  document.documentElement.dataset.surface='desktop';
  window.addEventListener('resize',resize);window.addEventListener('keydown',escape);
  observer=new MutationObserver(records=>{if(records.some(record=>!(record.type==='attributes'&&record.target instanceof Element&&record.target.closest('.media-playing-mark'))))scheduleRegions();});observer.observe(document.body,{childList:true,subtree:true,attributes:true,characterData:true});
  resizeObserver=new ResizeObserver(scheduleRegions);resizeObserver.observe(document.body);
  scheduleRegions();
});
onUnmounted(()=>{disposed=true;observer?.disconnect();resizeObserver?.disconnect();window.removeEventListener('resize',resize);window.removeEventListener('keydown',escape);if(regionTimer)clearTimeout(regionTimer);if(timer)clearTimeout(timer);delete document.documentElement.dataset.surface;});
</script>
<template>
  <main class="desktop-canvas" :class="{'is-arranging':editing}" aria-label="Vela 桌面画布">
    <section v-for="p in placements" :key="p.kind" class="desktop-widget" :class="{selected:selected===p.kind,locked:snapshot.settings.widgets[p.kind].locked}" :style="style(p)" :data-kind="p.kind" :tabindex="editing?0:undefined" :aria-label="widgetRegistry[p.kind].label" @keydown="keyboard($event,p)" @focus="selected=p.kind">
      <div class="desktop-widget-content" :inert="editing||undefined"><component :is="components[p.kind]" /></div>
      <div v-if="editing" class="desktop-widget-edit">
        <button class="desktop-drag-handle" :aria-label="`移动${widgetRegistry[p.kind].label}`" :disabled="snapshot.settings.widgets[p.kind].locked||saving" @pointerdown="startDrag($event,p)" @pointermove="moveDrag" @pointerup="stopDrag($event)" @pointercancel="stopDrag($event,true)" @lostpointercapture="stopDrag($event,true)"><AppIcon :name="widgetRegistry[p.kind].icon" :size="13" />{{widgetRegistry[p.kind].label}}<span>{{Math.round(position(p).x)}}, {{Math.round(position(p).y)}}</span></button>
        <button class="desktop-lock" :aria-label="`${snapshot.settings.widgets[p.kind].locked?'解锁':'锁定'}${widgetRegistry[p.kind].label}位置`" :aria-pressed="snapshot.settings.widgets[p.kind].locked" @click="run(()=>setWidgetLocked(p.kind,!snapshot.settings.widgets[p.kind].locked))"><AppIcon :name="snapshot.settings.widgets[p.kind].locked?'lock':'sliders'" :size="13" /></button>
        <select :value="p.size" :aria-label="`${widgetRegistry[p.kind].label}尺寸`" :disabled="saving" @change="run(()=>setWidgetSize(p.kind,($event.target as HTMLSelectElement).value as WidgetSize))"><option v-for="option in widgetSizeOptions" :key="option.value" :value="option.value">{{option.label}}</option></select>
      </div>
    </section>
    <nav v-if="!editing && !placements.length" class="desktop-quick-actions" data-canvas-overlay aria-label="中控入口"><button @click="run(()=>openManager('home'))"><AppIcon name="sliders" :size="15" />打开中控</button></nav>
    <nav v-if="editing" class="desktop-arrange-toolbar" :class="{'at-bottom':toolbarBottom}" data-canvas-overlay aria-label="桌面编排工具栏">
      <span class="desktop-arrange-title"><i></i>编排桌面</span>
      <button :aria-expanded="templatesOpen" @click="templatesOpen=!templatesOpen;saveOpen=false"><AppIcon name="grid" :size="14" />布局模板</button>
      <button :disabled="!placements.length" @click="saveOpen=!saveOpen;templatesOpen=false">保存布局</button>
      <button @click="run(()=>openManager('home'))">选择组件</button>
      <button :aria-label="toolbarBottom?'工具栏移到顶部':'工具栏移到底部'" @click="toolbarBottom=!toolbarBottom">{{toolbarBottom?'顶部':'底部'}}</button>
      <span class="desktop-arrange-hint">拖动标题 · Alt 取消吸附</span>
      <button class="desktop-arrange-done" :disabled="saving" @click="mode(false)">{{saving?'保存中…':'完成'}}</button>
      <div v-if="templatesOpen" class="desktop-template-menu" data-canvas-overlay>
        <p>快速排布<span>当前开启 {{placements.length}} 个 · 保留锁定位置</span></p>
        <button v-for="t in desktopTemplates" :key="t.id" :disabled="saving||!placements.length" @click="template(t.id)"><strong>{{t.label}}</strong><span>{{t.description}}</span></button>
        <template v-if="snapshot.settings.desktop.savedLayouts.length"><p>我的布局</p><button v-for="layout in snapshot.settings.desktop.savedLayouts" :key="layout.name" :disabled="saving||!placements.length" @click="restore(layout)"><strong>{{layout.name}}</strong><span>保存时 {{layout.placements.length}} 个组件</span></button></template>
      </div>
      <form v-if="saveOpen" class="desktop-save-layout" data-canvas-overlay @submit.prevent="save"><label>保存当前排布<input v-model="name" aria-label="布局名称" placeholder="例如：我的工作桌面" maxlength="40" /></label><button type="submit" :disabled="saving">保存</button></form>
    </nav>
    <p v-if="error" class="desktop-error" data-canvas-overlay role="alert">{{error}}</p>
  </main>
</template>
