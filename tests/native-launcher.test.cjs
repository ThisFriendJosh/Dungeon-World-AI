const {test}=require('node:test'),assert=require('node:assert/strict');
const {mkdtempSync,mkdirSync,writeFileSync,rmSync}=require('node:fs');
const {tmpdir}=require('node:os');const path=require('node:path');const {pathToFileURL}=require('node:url');const {spawnSync}=require('node:child_process');
const settings=()=>import(pathToFileURL(path.resolve('scripts/native-launch-settings.mjs')).href);
test('native launcher accepts canonical, timestamped and versioned builds without silently falling back',async()=>{
 const {resolveNativeLaunch}=await settings(),root=path.resolve('build');
 for(const name of ['dungeon-world-ai-desktop.exe','dungeon-world-ai-desktop-1794144611802.exe','dungeon-world-ai-desktop-0.2.2.exe','dungeon-world-ai-desktop-0.2.3-beta.1.exe'])assert.equal(path.basename(resolveNativeLaunch(root,{appFile:name}).executable),name);
 for(const name of ['../older.exe','dungeon-world-ai-desktop/old.exe','other.exe',null,42])assert.throws(()=>resolveNativeLaunch(root,{appFile:name}),/invalid/);
});

test('restricted interactive launches fail before building or opening an app while check stays read-only',async()=>{
 const {interactiveLaunchProblem,requireInteractiveLaunch}=await settings();
 assert.equal(interactiveLaunchProblem('fixture',{}),null);
 assert.match(interactiveLaunchProblem('fixture',{CODEX_SANDBOX_NETWORK_DISABLED:'1'}),/10013/);
 assert.throws(()=>requireInteractiveLaunch('fixture',{CODEX_SANDBOX_NETWORK_DISABLED:'1'}),/File Explorer/);
 const folder=mkdtempSync(path.join(tmpdir(),'dwa-restricted-launch-'));
 try{
  const app=path.join(folder,'fixture.exe');writeFileSync(app,'Not a runnable app.');
  const env={...process.env,DWA_NATIVE_APP_PATH:app,CODEX_SANDBOX_NETWORK_DISABLED:'1'};
  const launch=spawnSync(process.execPath,['scripts/native-dm.mjs'],{env,encoding:'utf8',windowsHide:true});
  assert.notEqual(launch.status,0);assert.match(launch.stderr,/networking disabled/);assert.match(launch.stderr,/10013/);
  const dev=spawnSync(process.execPath,['scripts/desktop.mjs','dev'],{env,encoding:'utf8',windowsHide:true});
  assert.notEqual(dev.status,0);assert.match(dev.stderr,/networking disabled/);
  const check=spawnSync(process.execPath,['scripts/native-dm.mjs','--check'],{env,encoding:'utf8',windowsHide:true});
  assert.equal(check.status,0,check.stderr);const report=JSON.parse(check.stdout);
  assert.equal(report.interactiveLaunchAllowed,false);assert.equal(report.build.current,false);
 }finally{assert.ok(folder.startsWith(path.join(tmpdir(),'dwa-restricted-launch-')));rmSync(folder,{recursive:true,force:true});}
});

test('workspace build records detect source changes, missing files and executable replacement without reading saves',async()=>{
 const {nativeInputs,nativeBuildStatus,buildRecord,workspaceSaveFolder}=await import(pathToFileURL(path.resolve('scripts/native-build-state.mjs')).href);
 const root=mkdtempSync(path.join(tmpdir(),'dwa-build-record-'));
 const put=(name,text)=>{const file=path.join(root,name);mkdirSync(path.dirname(file),{recursive:true});writeFileSync(file,text);return file;};
 try{
  put('apps/desktop/src-tauri/tauri.conf.json',JSON.stringify({version:'0.2.3'}));
  const source=put('apps/vtt-ui/src/main.ts','initial source');
  put('services/engine/migrations/001_init.sql','initial migration');
  put('apps/vtt-ui/public/rules/srd.json','reference content');
  const app=put('build/native/dungeon-world-ai-desktop-123.exe','compiled app');
  const snapshot=nativeInputs(root);
  assert.equal(nativeBuildStatus(root,app).current,false);
  const record=buildRecord(root,app,snapshot);
  const receipt=put('build/native/build-info.json',JSON.stringify(record));
  assert.equal(record.profile,'release');assert.equal(nativeBuildStatus(root,app).current,true);
  put('build/native/saved-games/campaigns/private.sqlite','private data');
  put('build/native/logs/dm-runtime.log','private runtime activity');
  put('models/model.gguf','model weights');
  assert.deepEqual(nativeInputs(root),snapshot);
  assert.equal(nativeBuildStatus(root,app).current,true);
  writeFileSync(source,'changed source');assert.match(nativeBuildStatus(root,app).reason,/source files have changed/);
  writeFileSync(source,'initial source');assert.equal(nativeBuildStatus(root,app).current,true);
  put('apps/vtt-ui/src/new-feature.ts','new source');assert.equal(nativeBuildStatus(root,app).current,false);
  rmSync(path.join(root,'apps/vtt-ui/src/new-feature.ts'));assert.equal(nativeBuildStatus(root,app).current,true);
  writeFileSync(app,'another build');assert.match(nativeBuildStatus(root,app).reason,/checksum/);
  rmSync(app);assert.match(nativeBuildStatus(root,app).reason,/missing/);
  writeFileSync(receipt,'broken JSON');assert.match(nativeBuildStatus(root,app).reason,/no valid build record/);
  writeFileSync(receipt,JSON.stringify({...record,appFile:{invalid:true}}));assert.equal(nativeBuildStatus(root,app).current,false);
  assert.equal(workspaceSaveFolder(root,{dataDir:'existing-saves'}),'existing-saves');
  assert.equal(workspaceSaveFolder(root,{}),path.join(root,'build/native/saved-games'));
 }finally{assert.ok(root.startsWith(path.join(tmpdir(),'dwa-build-record-')));rmSync(root,{recursive:true,force:true});}
});
test('a rebuild preserves the selected save folder unless explicitly overridden',async()=>{
 const {updatedNativeSettings,resolveNativeLaunch}=await settings();
 const previous={appFile:'dungeon-world-ai-desktop-0.2.2.exe',dataDir:path.resolve('build','saved-fixture'),extra:'keep'};
 const next=updatedNativeSettings(previous,{},'dungeon-world-ai-desktop.exe');
 assert.equal(next.dataDir,previous.dataDir);assert.equal(next.extra,'keep');assert.equal(previous.appFile,'dungeon-world-ai-desktop-0.2.2.exe');
 assert.equal(updatedNativeSettings(previous,{DWA_DATA_DIR:'another-save'},next.appFile).dataDir,'another-save');
 assert.equal(resolveNativeLaunch(path.resolve('.'),previous,{DWA_DATA_DIR:'explicit-save'}).dataDir,'explicit-save');
});
test('launcher check reports the selected executable without running it and rejects a missing build',()=>{
 const folder=mkdtempSync(path.join(tmpdir(),'dwa-launch-check-'));
 try{const app=path.join(folder,'fixture.exe');writeFileSync(app,'This is a read-only launcher fixture, not a program.');
  const env={...process.env,DWA_NATIVE_APP_PATH:app};const result=spawnSync(process.execPath,['scripts/native-dm.mjs','--check','--data',folder],{env,encoding:'utf8',windowsHide:true});
  assert.equal(result.status,0,result.stderr);const report=JSON.parse(result.stdout);assert.equal(report.executable,app);assert.equal(report.dataDir,folder);assert.equal(report.sha256.length,64);
  const missing=spawnSync(process.execPath,['scripts/native-dm.mjs','--check'],{env:{...env,DWA_NATIVE_APP_PATH:path.join(folder,'missing.exe')},encoding:'utf8',windowsHide:true});assert.notEqual(missing.status,0);assert.match(missing.stderr,/Build the native app first/);
 }finally{assert.ok(folder.startsWith(path.join(tmpdir(),'dwa-launch-check-')));rmSync(folder,{recursive:true,force:true});}
});
