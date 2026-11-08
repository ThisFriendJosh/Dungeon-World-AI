const {test}=require('node:test');
const assert=require('node:assert/strict'),path=require('node:path'),vm=require('node:vm');
const {createRequire}=require('node:module');
const esbuild=createRequire(path.resolve('apps/vtt-ui/package.json'))('esbuild');
function component(file,name){const bundle=esbuild.buildSync({entryPoints:['apps/vtt-ui/src/'+file],bundle:true,platform:'node',format:'cjs',write:false,alias:{'@tauri-apps/api/core':path.resolve('tests/tauri-render-stub.cjs')}});const fixture={exports:{}};vm.runInNewContext(bundle.outputFiles[0].text,{module:fixture,exports:fixture.exports,crypto:globalThis.crypto,structuredClone,console});return fixture.exports[name];}
const HistoryPanel=component('history-panel.ts','HistoryPanel');
const KnowledgePanel=component('knowledge-panel.ts','KnowledgePanel');
const AssetWorkshop=component('asset-workshop.ts','AssetWorkshop');
test('history removal controls require edit mode and a selected entry gets a confirmation dialog',()=>{
  const panel=new HistoryPanel();panel.load('campaign');const entry={seq:5,eventType:'dm.turn.saved',payload:{},createdAt:'2026-11-08T00:00:00Z',removed:false,canRemove:true};
  assert.ok(!panel.html([entry],()=>'<script>test</script>').includes('data-history-seq'));
  panel.editing=true;panel.entries=[entry];assert.ok(panel.html([entry],()=>'<script>test</script>').includes('data-history-seq="5"'));
  panel.selected=entry;const html=panel.html([entry],()=>'<script>test</script>');assert.ok(html.includes('id="historyConfirm"'));assert.ok(html.includes('Restore')||html.includes('restore'));assert.ok(!html.includes('<script>'));
  panel.load('another');assert.ok(!panel.html([],()=> 'entry').includes('historyConfirm'));
});
test('source imports preserve provenance and GM labels while escaping text',()=>{
  const panel=new KnowledgePanel();panel.load('campaign',[{id:'source',title:'<img src=x>',sourceRef:'<script>bad()</script>',edition:'homebrew',gmOnly:true,chunkCount:3}]);
  const html=panel.html();assert.ok(html.includes('GM only'));assert.ok(html.includes('&lt;img'));assert.ok(!html.includes('<script>'));assert.ok(html.includes('schemaVersion'));
  panel.load('another',[]);assert.ok(!panel.html().includes('&lt;img'));
});
test('asset workshop advertises local formats and escapes recipe names',()=>{
  const panel=new AssetWorkshop();panel.load('campaign',[{id:'asset',spec:{name:'<script>bad()</script>',label:'NPC',color:'#345678',shape:'circle',symbol:'shield'},createdAt:'2026-11-08T00:00:00Z'}]);
  const html=panel.html();assert.ok(html.includes('Save PNG + SVG'));assert.ok(html.includes('Place token'));assert.ok(!html.includes('<script>'));assert.ok(html.includes('&lt;script&gt;'));
});
