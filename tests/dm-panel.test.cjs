const {test}=require('node:test');
const assert=require('node:assert/strict'),path=require('node:path'),vm=require('node:vm');
const {createRequire}=require('node:module');
const esbuild=createRequire(path.resolve('apps/vtt-ui/package.json'))('esbuild');
const bundle=esbuild.buildSync({entryPoints:['apps/vtt-ui/src/dm-panel.ts'],bundle:true,platform:'node',format:'cjs',write:false,alias:{'@tauri-apps/api/core':path.resolve('tests/tauri-render-stub.cjs')}});
const fixture={exports:{}};
vm.runInNewContext(bundle.outputFiles[0].text,{module:fixture,exports:fixture.exports,crypto:globalThis.crypto,structuredClone,console});
const {DmPanel}=fixture.exports;
const runtime={configured:true,running:false,busy:false,model:'test-model',message:'Ready'};
function history(){return {turns:[{id:'turn',campaignId:'campaign',requestId:'request',actorId:'actor',playerText:'<script>player()</script>',reply:{schemaVersion:1,narration:'<img src=x onerror=bad()>\nThe gate opens.',choices:['<script>choice()</script>']},createdAt:'2026-11-08T00:00:00Z',eventSeq:4}],totalTurns:1,latestEventSeq:4};}
test('model, player, and actor text render as escaped text',()=>{
  const panel=new DmPanel();panel.load('campaign',history(),runtime,[]);
  const html=panel.html([{id:'actor',name:'<b>Orrek</b>'}]);
  assert.ok(!html.includes('<script>'));assert.ok(!html.includes('<img src='));assert.ok(!html.includes('<b>Orrek'));
  assert.ok(html.includes('&lt;img'));assert.ok(html.includes('&lt;b&gt;Orrek'));assert.ok(html.includes('Saved on this device'));
});
test('pending generation keeps stop enabled and composer disabled',()=>{
  const panel=new DmPanel();panel.load('campaign',history(),runtime,[]);panel.pending=true;
  const html=panel.html([]);assert.ok(html.includes('<fieldset disabled>'));assert.ok(html.includes('id="dmCancel" type="button" >Stop'));
});
test('campaign switching clears prior transcript and unavailable model still shows saved history',()=>{
  const panel=new DmPanel();panel.load('campaign',history(),{...runtime,configured:false},[]);
  assert.ok(panel.html([]).includes('The gate opens.'));assert.ok(panel.html([]).includes('type="submit" disabled'));
  panel.load('another',{turns:[],totalTurns:0,latestEventSeq:1},runtime,[]);
  assert.ok(!panel.html([]).includes('The gate opens.'));
});
