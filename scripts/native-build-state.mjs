import {createHash} from "node:crypto";
import {closeSync,existsSync,lstatSync,openSync,readFileSync,readSync,readdirSync} from "node:fs";
import {join,relative,resolve} from "node:path";

// Only compiler/bundler inputs. Saves, models, build output and logs are excluded.
const inputFiles=[
  "package.json","package-lock.json","apps/vtt-ui/package.json","apps/vtt-ui/index.html",
  "apps/vtt-ui/vite.config.ts","apps/vtt-ui/tsconfig.json",
  "apps/desktop/src-tauri/Cargo.toml","apps/desktop/src-tauri/Cargo.lock",
  "apps/desktop/src-tauri/build.rs","apps/desktop/src-tauri/tauri.conf.json",
  "apps/desktop/src-tauri/tauri.windows.conf.json",
  "services/engine/Cargo.toml","services/engine/Cargo.lock",
  "services/ai/Cargo.toml","services/ai/Cargo.lock","services/art/worker.py",
  "Start-Dungeon-World-AI.cmd","Update-Dungeon-World-AI.cmd",
  "scripts/build-native.mjs","scripts/native-dm.mjs","scripts/native-launch-settings.mjs",
  "scripts/native-build-state.mjs","scripts/desktop.mjs","scripts/dm-dev.mjs"
];
const inputFolders=[
  "apps/vtt-ui/src","apps/vtt-ui/public","apps/desktop/src-tauri/src",
  "apps/desktop/src-tauri/icons","apps/desktop/src-tauri/capabilities",
  "services/engine/src","services/engine/migrations","services/ai/src",
  "services/ai/prompts","packages/shared/assets"
];
export function fileDigest(file){
  const hash=createHash("sha256"),buffer=Buffer.allocUnsafe(64*1024),fd=openSync(file,"r");
  try{let length;while((length=readSync(fd,buffer,0,buffer.length,null))>0)hash.update(buffer.subarray(0,length));}
  finally{closeSync(fd);}
  return hash.digest("hex");
}
export function nativeInputs(root){
  const files=new Set(inputFiles.filter(file=>existsSync(join(root,file))));
  function visit(folder){
    if(!existsSync(join(root,folder)))return;
    for(const item of readdirSync(join(root,folder),{withFileTypes:true})){
      const name=folder+"/"+item.name;
      if(item.isSymbolicLink())throw Error("A workspace build input is a symbolic link: "+name);
      if(item.isDirectory())visit(name);else if(item.isFile())files.add(name);
    }
  }
  for(const folder of inputFolders)visit(folder);
  const entries=[...files].sort().map(path=>({path,sha256:fileDigest(join(root,path))}));
  return {sha256:createHash("sha256").update(JSON.stringify(entries)).digest("hex"),files:entries.length};
}
export function nativeBuildStatus(root,executable){
  const inputs=nativeInputs(root),file=join(root,"build/native/build-info.json");
  let receipt=null;
  try{receipt=JSON.parse(readFileSync(file,"utf8").replace(/^\uFEFF/,""));}catch{}
  const result={current:false,reason:"The workspace build has no valid build record.",inputs,receipt};
  if(!receipt||receipt.schemaVersion!==1||receipt.profile!=="release"||typeof receipt.appFile!=="string"||typeof receipt.sha256!=="string")return result;
  if(receipt.inputs?.sha256!==inputs.sha256)return {...result,reason:"Workspace source files have changed since the last build."};
  if(!existsSync(executable)||!lstatSync(executable).isFile())return {...result,reason:"The selected workspace executable is missing."};
  if(resolve(root,"build/native",receipt.appFile??"")!==resolve(executable))return {...result,reason:"The selected executable differs from the recorded workspace build."};
  if(receipt.sha256!==fileDigest(executable))return {...result,reason:"The selected executable differs from its recorded checksum."};
  return {...result,current:true,reason:"The workspace build matches the current source files."};
}
export function workspaceSaveFolder(root,settings){
  return settings.dataDir||join(root,"build/native/saved-games");
}
export function buildRecord(root,executable,inputs){
  const config=JSON.parse(readFileSync(join(root,"apps/desktop/src-tauri/tauri.conf.json"),"utf8"));
  return {schemaVersion:1,version:config.version,profile:"release",builtAt:new Date().toISOString(),
    appFile:relative(join(root,"build/native"),executable),inputs,sha256:fileDigest(executable)};
}
