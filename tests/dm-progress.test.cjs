const {test}=require('node:test'),assert=require('node:assert/strict'),path=require('node:path'),vm=require('node:vm');
const {createRequire}=require('node:module');const esbuild=createRequire(path.resolve('apps/vtt-ui/package.json'))('esbuild');
function fixture(invoke){
 const bundle=esbuild.buildSync({entryPoints:['apps/vtt-ui/src/dm-panel.ts'],bundle:true,platform:'node',format:'cjs',write:false,alias:{'@tauri-apps/api/core':path.resolve('tests/tauri-interaction-stub.cjs')}});
 const module={exports:{}};let tick,cleared=0;
 vm.runInNewContext(bundle.outputFiles[0].text,{module,exports:module.exports,structuredClone,console,__nativeInvoke:invoke,setInterval(callback){tick=callback;return 1;},clearInterval(){cleared++;}});
 const panel=new module.exports.DmPanel(),runtime={configured:true,running:true,busy:false,model:'fixture',message:'Ready'};
 const message={textContent:'Initial'},field={value:'',addEventListener(){}};
 const root={querySelector:selector=>selector==='#dmProgressMessage'?message:['#dmAction','#dmActor'].includes(selector)?field:null,querySelectorAll:()=>[]};
 const history={turns:[],totalTurns:0,latestEventSeq:0};panel.load('first',history,runtime,[]);panel.pending=true;panel.bind(root,{campaignId:'first',render(){},onSaved:async()=>{}});
 return{panel,message,history,runtime,tick:()=>tick(),cleared:()=>cleared,root};
}
test('pending DM replies show native progress and stop polling after completion',async()=>{
 const f=fixture(async command=>{assert.equal(command,'dm_runtime_status');return{...f.runtime,busy:true,message:'Reading campaign context — 40% · 2m 22s.'};});
 f.tick();await new Promise(resolve=>setImmediate(resolve));assert.match(f.message.textContent,/40%/);
 f.panel.pending=false;f.panel.bind(f.root,{campaignId:'first',render(){},onSaved:async()=>{}});assert.equal(f.cleared(),1);
});
test('a late progress response cannot replace the state of another campaign',async()=>{
 let resolve;const f=fixture(()=>new Promise(done=>resolve=done));f.tick();f.panel.load('second',f.history,f.runtime,[]);resolve({...f.runtime,busy:true,message:'Old request progress'});await new Promise(done=>setImmediate(done));assert.equal(f.message.textContent,'Initial');assert.equal(f.cleared(),1);
});
