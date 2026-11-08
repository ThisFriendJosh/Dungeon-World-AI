const {test}=require('node:test'),assert=require('node:assert/strict');
const {mkdtempSync,writeFileSync,rmSync}=require('node:fs');
const {tmpdir}=require('node:os');const path=require('node:path');const {pathToFileURL}=require('node:url');const {spawnSync}=require('node:child_process');
const settings=()=>import(pathToFileURL(path.resolve('scripts/native-launch-settings.mjs')).href);
test('native launcher accepts canonical, timestamped and versioned builds without silently falling back',async()=>{
 const {resolveNativeLaunch}=await settings(),root=path.resolve('build');
 for(const name of ['dungeon-world-ai-desktop.exe','dungeon-world-ai-desktop-1794144611802.exe','dungeon-world-ai-desktop-0.2.2.exe','dungeon-world-ai-desktop-0.2.3-beta.1.exe'])assert.equal(path.basename(resolveNativeLaunch(root,{appFile:name}).executable),name);
 for(const name of ['../older.exe','dungeon-world-ai-desktop/old.exe','other.exe',null,42])assert.throws(()=>resolveNativeLaunch(root,{appFile:name}),/invalid/);
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
