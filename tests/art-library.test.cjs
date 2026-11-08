const {test}=require('node:test'),assert=require('node:assert/strict'),path=require('node:path'),vm=require('node:vm');
const {createRequire}=require('node:module');
const esbuild=createRequire(path.resolve('apps/vtt-ui/package.json'))('esbuild');
function bundle(file,invoke=async()=>{throw Error('Unexpected native call');}){const result=esbuild.buildSync({entryPoints:['apps/vtt-ui/src/'+file],bundle:true,platform:'node',format:'cjs',write:false,alias:{'@tauri-apps/api/core':path.resolve('tests/tauri-interaction-stub.cjs')}});const fixture={exports:{}};vm.runInNewContext(result.outputFiles[0].text,{module:fixture,exports:fixture.exports,__nativeInvoke:invoke,crypto:globalThis.crypto,structuredClone,Uint32Array,console});return fixture.exports;}
function node(value=''){return {value,checked:true,addEventListener(event,handler){this[event]=handler;}};}
const entry=(id,title,tags,role='token')=>({id,metadata:{title,categories:['Creatures','Token portraits'],tags,role,note:'Friendly guide'},width:512,height:512,hasSvg:true,generated:false});
test('uploads and saved tokens share categories, custom tags and search without leaking another campaign',()=>{
  const {ArtLibrary}=bundle('art-library.ts'),panel=new ArtLibrary();panel.load('a',[entry('one','Forest <bear>',['Aerathis','<script>bad()</script>']),entry('two','Goblin scout',['caves'])]);
  let html=panel.html();assert.ok(html.includes('Choose images'));assert.ok(html.includes('multiple'));assert.ok(html.includes('.svg'));assert.ok(html.includes('Filter by tag'));assert.ok(html.includes('&lt;bear&gt;'));assert.ok(!html.includes('<script>bad()'));
  panel.tag='Aerathis';html=panel.html();assert.ok(html.includes('data-view-art="one"'));assert.ok(!html.includes('data-view-art="two"'));
  panel.tag='';panel.query='caves';assert.ok(panel.html().includes('data-view-art="two"'));assert.ok(!panel.html().includes('data-view-art="one"'));
  panel.load('b',[]);assert.ok(!panel.html().includes('data-art-tag="Aerathis"'));assert.ok(!panel.html().includes('data-view-art="one"'));assert.equal(panel.dirty,false);
});
test('a failed image save keeps its reviewed metadata and remaining queue for retry',async()=>{
  const tasks=[],calls=[];let fail=true,saved=0;
  const {ArtLibrary}=bundle('art-library.ts',async(command,args)=>{assert.equal(command,'import_art');calls.push(args.request);if(fail)throw Error('Disk unavailable');return {value:{id:'saved'},warning:null};});
  const panel=new ArtLibrary();panel.load('a',[]);const item={image:{name:'bear.png',dataUrl:'PNG',thumbnailDataUrl:'THUMB',width:512,height:512,sourceSvg:null},metadata:entry('x','Bear',[]).metadata,error:''};panel.pending=[item,{...item,image:{...item.image,name:'goblin.png'}}];
  const form=node();form.dataset={artImport:'0'};const fields={'#upload0Title':node('Forest bear'),'#upload0Tags':node('forest, friendly, FOREST'),'#upload0Role':node('token'),'#upload0Note':node('Campaign guide')};
  const root={querySelector:s=>fields[s]??null,querySelectorAll:s=>s==='[data-art-import]'?[form]:s==='input[name="upload0Category"]:checked'?[node('Creatures'),node('Token portraits')]:[]};
  panel.bind(root,{campaignId:'a',render(){},onSaved:async()=>saved++,action(work){const task=work();tasks.push(task);return task;}});
  form.submit({preventDefault(){}});await tasks.at(-1);assert.equal(panel.pending.length,2);assert.match(item.error,/Disk unavailable/);assert.equal(saved,0);assert.equal(calls[0].metadata.tags.join(','),'FOREST,friendly');
  fail=false;form.submit({preventDefault(){}});await tasks.at(-1);assert.equal(panel.pending.length,1);assert.equal(saved,1);assert.equal(calls.length,2);assert.equal(calls[1].metadata.note,'Campaign guide');
});
test('AI sprite generation exposes progress and stop while retaining the previous image after failure',async()=>{
  let rejectGeneration,finished;const done=new Promise(resolve=>finished=resolve),calls=[];
  const {AssetWorkshop}=bundle('asset-workshop.ts',async(command,args)=>{calls.push(command);if(command==='generate_token_art'){args.progress.onmessage('Drawing the sprite…');return new Promise((_,reject)=>rejectGeneration=reject);}if(command==='cancel_token_art'){rejectGeneration(Error('Image generation cancelled.'));return true;}throw Error('Unexpected '+command);});
  const panel=new AssetWorkshop();panel.load('a',[],{available:true,detail:'Ready',model:'local'});panel.artwork={pngDataUrl:'previous',recipe:{prompt:'old bear',seed:8,model:'local',revision:'one',transparentBackground:true}};
  const fields={'#assetName':node('Bear'),'#assetLabel':node('NPC'),'#assetColor':node('#345678'),'#assetShape':node('circle'),'#assetPrompt':node('A forest bear'),'#assetSeed':node('42'),'#assetTransparent':node(),'#assetGenerate':node(),'#assetCancel':node()};const root={querySelector:s=>fields[s]??null,querySelectorAll:()=>[]};
  panel.bind(root,{campaignId:'a',render(){if(!panel.pending&&panel.error)finished();},action(){throw Error('Generation should remain cancellable');},onSaved(){throw Error('No asset should be saved');}});
  fields['#assetGenerate'].click();assert.equal(panel.pending,true);assert.match(panel.html(),/Drawing the sprite/);assert.match(panel.html(),/id="assetCancel" >Stop/);assert.ok(panel.html().includes('<fieldset disabled>'));fields['#assetCancel'].click();await done;
  assert.equal(panel.pending,false);assert.equal(panel.artwork.pngDataUrl,'previous');assert.equal(calls.join(','),'generate_token_art,cancel_token_art');
});
test('sprite creation uses a description and retains frame and color controls',()=>{const {AssetWorkshop}=bundle('asset-workshop.ts'),panel=new AssetWorkshop();panel.load('a',[],{available:false,detail:'Run Setup-Local-Art.cmd',model:'local'});const html=panel.html();assert.ok(html.includes('Sprite description'));assert.ok(html.includes('assetShape'));assert.ok(html.includes('assetColor'));assert.ok(!html.includes('id="assetSymbol"'));assert.ok(html.includes('Setup-Local-Art.cmd'));assert.match(html,/id="assetGenerate"[^>]*disabled/);});
