import type { MediaTheme, Settings, WidgetKind, WidgetSize } from '../../shared/types';
export interface Placement { kind: WidgetKind; size: WidgetSize; x: number; y: number }
export interface SavedLayout { name: string; width: number; height: number; placements: Placement[] }
export const desktopTemplates = [
  { id: 'left', label: '左侧工具栏', description: '常用信息集中在左侧' },
  { id: 'right', label: '右侧信息栏', description: '为桌面中央保留空间' },
  { id: 'bottom', label: '底部横排', description: '开启的组件沿底部横向排列' },
  { id: 'focus', label: '工作专注', description: '开启的组件紧凑集中排列' },
] as const;
export function widgetFootprint(kind: WidgetKind, size: WidgetSize, theme: MediaTheme) {
  if (kind === 'media' && size === 'medium' && theme === 'atmosphere') return { width: 224, height: 356 };
  return { width: size === 'small' ? 170 : 364, height: size === 'large' ? 384 : 170 };
}
export function clampPosition(x: number, y: number, width: number, height: number, bounds: { width: number; height: number }) {
  return { x: Math.round(Math.max(0, Math.min(Number.isFinite(x) ? x : 0, Math.max(0,bounds.width-width)))), y: Math.round(Math.max(0, Math.min(Number.isFinite(y) ? y : 0, Math.max(0,bounds.height-height)))) };
}
export function snapPosition(x: number, y: number, width: number, height: number, bounds: {width:number;height:number}, others: {x:number;y:number;width:number;height:number}[], snap = true) {
  const nearest = (value: number, candidates: number[]) => candidates.reduce((best,n) => Math.abs(value-n) < Math.abs(value-best) ? n : best, value + 9);
  if (snap) {
    const sx = nearest(x,[0,bounds.width-width,Math.round(x/16)*16,...others.flatMap(r=>[r.x,r.x+r.width+16,r.x-width-16,r.x+r.width-width])]);
    const sy = nearest(y,[0,bounds.height-height,Math.round(y/16)*16,...others.flatMap(r=>[r.y,r.y+r.height+16,r.y-height-16,r.y+r.height-height])]);
    if(Math.abs(sx-x)<=8)x=sx;if(Math.abs(sy-y)<=8)y=sy;
  }
  return clampPosition(x,y,width,height,bounds);
}
type Bounds = {width:number;height:number};
type LayoutRect = Placement & {width:number;height:number};
const gap = 16;
const margin = 24;
function intersects(a:LayoutRect,b:LayoutRect) {
  return a.x < b.x+b.width+gap && a.x+a.width+gap > b.x && a.y < b.y+b.height+gap && a.y+a.height+gap > b.y;
}
function asRect(p:Placement,theme:MediaTheme):LayoutRect {return {...p,...widgetFootprint(p.kind,p.size,theme)};}
function score(p:LayoutRect,id:string,bounds:Bounds):number[] {
  if(id==='right')return [bounds.width-p.x-p.width,p.y];
  if(id==='bottom')return [bounds.height-p.y-p.height,p.x];
  if(id==='focus')return [Math.abs(p.x+p.width/2-bounds.width/2)+Math.abs(p.y+p.height/2-bounds.height/2),p.y,p.x];
  return [p.x,p.y];
}
function candidates(p:Placement,occupied:LayoutRect[],bounds:Bounds,theme:MediaTheme,id:string):LayoutRect[] {
  const size=widgetFootprint(p.kind,p.size,theme);
  const xs=new Set([margin,bounds.width-margin-size.width]);
  const ys=new Set([margin,bounds.height-margin-size.height]);
  if(id==='focus') {xs.add(Math.round((bounds.width-size.width)/2));ys.add(Math.round((bounds.height-size.height)/2));}
  for(const r of occupied) {
    for(const x of [r.x,r.x+r.width-size.width,r.x+r.width+gap,r.x-size.width-gap])xs.add(x);
    for(const y of [r.y,r.y+r.height-size.height,r.y+r.height+gap,r.y-size.height-gap])ys.add(y);
  }
  return [...ys].flatMap(y=>[...xs].map(x=>({...p,...size,x,y})))
    .filter(r=>r.x>=margin && r.y>=margin && r.x+r.width<=bounds.width-margin && r.y+r.height<=bounds.height-margin && occupied.every(other=>!intersects(r,other)))
    .sort((a,b)=>{const sa=score(a,id,bounds),sb=score(b,id,bounds);for(let i=0;i<sa.length;i++){if(sa[i]!==sb[i])return sa[i]-sb[i];}return 0;});
}
function pack(entries:Placement[],reserved:LayoutRect[],bounds:Bounds,theme:MediaTheme,id:string):Placement[] {
  // Larger cards first; limited backtracking prevents a small card blocking the
  // only space that can fit a portrait cover or a large calendar.
  const pending=[...entries].sort((a,b)=>{const x=widgetFootprint(a.kind,a.size,theme),y=widgetFootprint(b.kind,b.size,theme);return y.width*y.height-x.width*x.height;});
  if(pending.some(p=>{const s=widgetFootprint(p.kind,p.size,theme);return s.width>bounds.width-margin*2||s.height>bounds.height-margin*2;}) || [...pending.map(p=>asRect(p,theme)),...reserved].reduce((sum,r)=>sum+r.width*r.height,0)>(bounds.width-margin*2)*(bounds.height-margin*2)) {
    throw new Error('当前工作区放不下这些组件，请缩小组件尺寸或减少开启数量。');
  }
  let attempts=0;
  function place(index:number,occupied:LayoutRect[]):LayoutRect[]|null {
    if(index===pending.length)return occupied;
    if(++attempts>5000)return null;
    for(const candidate of candidates(pending[index],occupied,bounds,theme,id)) {
      const result=place(index+1,[...occupied,candidate]);if(result)return result;
    }
    return null;
  }
  const packed=place(0,reserved);
  if(!packed)throw new Error('当前工作区无法避开锁定组件完成排布，请调整锁定位置或缩小组件尺寸。');
  return packed.map(({kind,size,x,y})=>({kind,size,x,y}));
}
export function templateLayout(id:string,bounds:Bounds,settings:Settings):Placement[] {
  const current=visibleLayout(settings,bounds);
  const locked=current.filter(p=>settings.widgets[p.kind].locked).map(p=>asRect(p,settings.media.theme));
  const movable=current.filter(p=>!settings.widgets[p.kind].locked);
  if(!movable.length)return current;
  const packed=pack(movable,locked,bounds,settings.media.theme,id);
  return current.map(p=>packed.find(entry=>entry.kind===p.kind)!);
}
export function restoreLayout(layout:SavedLayout,settings:Settings,bounds:Bounds):Placement[] {
  const current=visibleLayout(settings,bounds),theme=settings.media.theme;
  const reserved=current.filter(p=>settings.widgets[p.kind].locked).map(p=>asRect(p,theme));
  const remaining:Placement[]=[];
  for(const p of current.filter(p=>!settings.widgets[p.kind].locked)) {
    const saved=layout.placements.find(entry=>entry.kind===p.kind);
    if(saved && layout.width>0 && layout.height>0) {
      const size=widgetFootprint(p.kind,p.size,theme);
      const point=clampPosition(saved.x*bounds.width/layout.width,saved.y*bounds.height/layout.height,size.width,size.height,bounds);
      const r=asRect({...p,...point},theme);
      if(reserved.every(other=>!intersects(r,other))){reserved.push(r);continue;}
    }
    remaining.push(p);
  }
  const packed=remaining.length?pack(remaining,reserved,bounds,theme,'focus'):reserved;
  return current.map(p=>{const saved=packed.find(entry=>entry.kind===p.kind)!;return{kind:p.kind,size:p.size,x:saved.x,y:saved.y};});
}
// Rust serializes widget settings from a HashMap; visual order must not depend on it.
const desktopOrder: WidgetKind[] = ['calendar', 'clock', 'todo', 'note', 'habit', 'countdown', 'media'];
export function visibleLayout(settings: Settings, bounds: {width:number;height:number}): Placement[] {
  const active = desktopOrder.filter(kind => settings.widgets[kind]?.enabled);
  const occupied: {x:number;y:number;width:number;height:number}[] = [];
  const placed = new Map<WidgetKind, Placement>();
  for (const kind of active) {
    const widget = settings.widgets[kind], size = widgetFootprint(kind, widget.size, settings.media.theme);
    if (widget.x == null || widget.y == null) continue;
    const point = clampPosition(widget.x, widget.y, size.width, size.height, bounds);
    placed.set(kind, {kind, size:widget.size, ...point});
    occupied.push({...point, ...size});
  }
  for (const kind of active) {
    if (placed.has(kind)) continue;
    const widget = settings.widgets[kind], size = widgetFootprint(kind, widget.size, settings.media.theme);
    const xs = [24, ...occupied.map(r => r.x+r.width+16)];
    const ys = [24, ...occupied.map(r => r.y+r.height+16)];
    const candidates = ys.sort((a,b)=>a-b).flatMap(y=>xs.sort((a,b)=>a-b).map(x=>({x,y})))
      .filter(p=>p.x+size.width<=bounds.width && p.y+size.height<=bounds.height);
    const free = candidates.find(p=>occupied.every(r=>p.x+size.width<=r.x || p.x>=r.x+r.width || p.y+size.height<=r.y || p.y>=r.y+r.height));
    const point = free ?? clampPosition(24,24,size.width,size.height,bounds);
    placed.set(kind,{kind,size:widget.size,...point});occupied.push({...point,...size});
  }
  return active.map(kind=>placed.get(kind)!);
}
