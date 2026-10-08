import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import ts from 'typescript';
const source=readFileSync(new URL('../src/features/desktop/layout.ts',import.meta.url),'utf8');
const output=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext}}).outputText;
const {widgetFootprint,clampPosition,snapPosition,templateLayout,restoreLayout,visibleLayout}=await import(`data:text/javascript;base64,${Buffer.from(output).toString('base64')}`);
const overlaps=(a,b)=>a.x<b.x+b.width&&a.x+a.width>b.x&&a.y<b.y+b.height&&a.y+a.height>b.y;
function settingsFor(entries,theme='atmosphere') {
  return {media:{theme}, widgets:Object.fromEntries(entries.map(([kind,size,extra={}])=>[kind,{enabled:true,size,locked:false,x:null,y:null,...extra}]))};
}
function assertFits(placements,settings,bounds) {
  const rects=placements.map(p=>({...p,...widgetFootprint(p.kind,p.size,settings.media.theme)}));
  for(const r of rects)assert.ok(r.x>=0&&r.y>=0&&r.x+r.width<=bounds.width&&r.y+r.height<=bounds.height,`${r.kind} fits`);
  for(let i=0;i<rects.length;i++)for(let j=i+1;j<rects.length;j++)assert.equal(overlaps(rects[i],rects[j]),false,`${rects[i].kind} avoids ${rects[j].kind}`);
}
test('templates arrange the enabled set, preserve sizes and fit mixed and portrait widgets',()=>{
  const choices=[[['habit','small']],[['calendar','large'],['countdown','small'],['media','medium']],
    [['calendar','large'],['clock','small'],['todo','medium'],['note','large']],
    [['calendar','medium'],['clock','small'],['todo','medium'],['note','large'],['habit','small'],['countdown','small'],['media','medium']]];
  for(const entries of choices)for(const bounds of [{width:1920,height:1040},{width:1280,height:720}])for(const theme of ['atmosphere','vinyl','minimal'])for(const id of ['left','right','bottom','focus']){
    const settings=settingsFor([...entries,['note','large',{enabled:false}]].filter((entry,i,all)=>all.findIndex(other=>other[0]===entry[0])===i),theme);
    const before=structuredClone(settings),result=templateLayout(id,bounds,settings);
    const expected=Object.entries(settings.widgets).filter(([,w])=>w.enabled).map(([kind,w])=>({kind,size:w.size})).sort((a,b)=>a.kind.localeCompare(b.kind));
    assert.deepEqual(result.map(({kind,size})=>({kind,size})).sort((a,b)=>a.kind.localeCompare(b.kind)),expected);
    assertFits(result,settings,bounds);assert.deepEqual(settings,before);
  }
});
test('portrait media aligns both edges with a two-card stack in edge templates',()=>{
  const bounds={width:500,height:404},settings=settingsFor([['media','medium'],['todo','small'],['habit','small']]);
  for(const id of ['left','right','bottom']) {
    const result=templateLayout(id,bounds,settings);
    const media=result.find(p=>p.kind==='media');
    const stack=result.filter(p=>p.kind!=='media').sort((a,b)=>a.y-b.y);
    assert.equal(stack[0].x,stack[1].x);
    assert.equal(stack[0].y,media.y);
    assert.equal(stack[1].y-stack[0].y,170+16);
    assert.equal(stack[1].y+170,media.y+widgetFootprint('media','medium','atmosphere').height);
    assertFits(result,settings,bounds);
  }
});

test('layouts avoid locked cards and preserve their position for every template',()=>{
  const bounds={width:1280,height:720};
  const settings=settingsFor([['note','large',{locked:true,x:416,y:210}],['clock','small'],['todo','medium'],['media','medium']]);
  for(const id of ['left','right','bottom','focus']){
    const result=templateLayout(id,bounds,settings);
    assert.deepEqual(result.find(p=>p.kind==='note'),{kind:'note',size:'large',x:416,y:210});assertFits(result,settings,bounds);
  }
});
test('empty desktops and all-locked layouts are safe; insufficient space never mutates settings',()=>{
  assert.deepEqual(templateLayout('left',{width:800,height:600},settingsFor([])),[]);
  const locked=settingsFor([['clock','small',{locked:true,x:45,y:60}]]);
  assert.deepEqual(templateLayout('bottom',{width:800,height:600},locked),[{kind:'clock',size:'small',x:45,y:60}]);
  const settings=settingsFor([['note','large'],['calendar','large']]),before=structuredClone(settings);
  assert.throws(()=>templateLayout('focus',{width:364,height:384},settings),/工作区/);assert.deepEqual(settings,before);
});
test('restoring old layouts uses current enabled widgets and sizes and auto-places additions',()=>{
  const bounds={width:2000,height:1200};
  const settings=settingsFor([['clock','medium'],['habit','small'],['note','large',{enabled:false}]]);
  const saved={name:'Old',width:1000,height:600,placements:[{kind:'clock',size:'small',x:50,y:50},{kind:'note',size:'medium',x:500,y:50}]};
  const result=restoreLayout(saved,settings,bounds);
  assert.deepEqual(result.find(p=>p.kind==='clock'),{kind:'clock',size:'medium',x:100,y:100});
  assert.deepEqual(result.map(p=>p.kind).sort(),['clock','habit']);assertFits(result,settings,bounds);
});
test('default placement is stable across native map order and avoids existing positioned widgets',()=>{
  const widgets={clock:{enabled:true,size:'small',x:null,y:null},todo:{enabled:true,size:'medium',x:24,y:24},note:{enabled:true,size:'medium',x:null,y:null}};
  const settings={widgets,media:{theme:'atmosphere'}}, bounds={width:1280,height:720};
  const result=visibleLayout(settings,bounds);
  assert.deepEqual(result,visibleLayout({...settings,widgets:Object.fromEntries(Object.entries(widgets).reverse())},bounds));
  const rects=result.map(p=>({...p,...widgetFootprint(p.kind,p.size,'atmosphere')}));
  for(let i=0;i<rects.length;i++)for(let j=i+1;j<rects.length;j++)assert.equal(overlaps(rects[i],rects[j]),false);
  assert.deepEqual(visibleLayout({widgets:{},media:{theme:'vinyl'}},bounds),[]);
});
test('drag snapping aligns neighbours and can be disabled; invalid or off-screen positions are clamped',()=>{
  const bounds={width:1200,height:800},others=[{x:100,y:100,width:170,height:170}];
  assert.deepEqual(snapPosition(285,99,170,170,bounds,others),{x:286,y:100});
  assert.deepEqual(snapPosition(285,99,170,170,bounds,others,false),{x:285,y:99});
  assert.deepEqual(clampPosition(Infinity,-200,170,170,bounds),{x:0,y:0});
  assert.deepEqual(clampPosition(2000,2000,364,384,bounds),{x:836,y:416});
});
