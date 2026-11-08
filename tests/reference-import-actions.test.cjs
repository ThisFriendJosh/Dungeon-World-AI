const {test}=require('node:test'),assert=require('node:assert/strict'),path=require('node:path'),vm=require('node:vm');
const {createRequire}=require('node:module');
const esbuild=createRequire(path.resolve('apps/vtt-ui/package.json'))('esbuild');
function panelWith(invoke){const bundle=esbuild.buildSync({entryPoints:['apps/vtt-ui/src/knowledge-panel.ts'],bundle:true,platform:'node',format:'cjs',write:false,alias:{'@tauri-apps/api/core':path.resolve('tests/tauri-interaction-stub.cjs')}});const fixture={exports:{}};vm.runInNewContext(bundle.outputFiles[0].text,{module:fixture,exports:fixture.exports,TextEncoder,__nativeInvoke:invoke});return new fixture.exports.KnowledgePanel();}
const source=title=>({source:{title,text:'Mournwell campaign lore',edition:'homebrew',gmOnly:true,sourceRef:title+'.txt'},method:'Document text',warnings:[]});
test('reviewed batch imports retain unsaved sources and retry without duplicating committed ones',async()=>{
  const calls=[],errors=[],tasks=[];let fail=true,saved=0;
  const panel=panelWith(async(command,args)=>{calls.push(args.request.title);if(args.request.title==='Second'&&fail)throw Error('Disk temporarily unavailable');return {value:{},warning:null};});
  panel.load('campaign',[]);panel.prepared=[source('First'),source('Second'),source('Third')];
  const button={addEventListener(event,handler){this[event]=handler;}},root={querySelector:selector=>selector==='#knowledgeSavePrepared'?button:null,querySelectorAll:()=>[]};
  panel.bind(root,{campaignId:'campaign',render(){},onDirty(){},onMessage(){},onSaved:async()=>saved++,action(work){const task=work().catch(error=>errors.push(String(error)));tasks.push(task);return task;}});
  button.click();await tasks.at(-1);
  assert.equal(saved,1);assert.equal(panel.prepared.length,2);assert.equal(panel.prepared[0].source.title,'Second');assert.ok(errors[0].includes('1 source(s) saved'));
  fail=false;button.click();await tasks.at(-1);assert.equal(panel.prepared.length,0);assert.equal(calls.join(','),'First,Second,Second,Third');assert.equal(saved,2);
});
test('preparing and reviewing text does not commit an import',()=>{
  let calls=0;const panel=panelWith(async()=>{calls++;});panel.load('campaign',[]);panel.prepared=[source('<script>Unsafe title</script>')];
  const html=panel.html();assert.equal(calls,0);assert.ok(html.includes('Review before adding'));assert.ok(html.includes('&lt;script&gt;'));assert.ok(!html.includes('<script>'));assert.equal(panel.dirty,true);
  panel.load('different-campaign',[]);assert.equal(panel.dirty,false);assert.equal(panel.prepared.length,0);
});
