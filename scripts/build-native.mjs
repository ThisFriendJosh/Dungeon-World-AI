import {mkdirSync,copyFileSync,existsSync,writeFileSync} from "node:fs";
import {homedir} from "node:os";
import {join,dirname} from "node:path";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
const root=join(dirname(fileURLToPath(import.meta.url)),"..");
if(process.platform!=="win32")throw Error("This native development launcher currently targets Windows.");
const env={...process.env,CARGO_INCREMENTAL:"0"};env.CARGO_TARGET_DIR||=join(root,"build","cargo-target");
const ui=spawnSync(env.ComSpec||"cmd.exe",["/d","/s","/c","npm.cmd run build:ui"],{cwd:root,env,stdio:"inherit",windowsHide:true});
if(ui.error)throw ui.error;if(ui.status!==0)process.exit(ui.status??1);
const installedCargo=join(env.CARGO_HOME||join(homedir(),".cargo"),"bin","cargo.exe");
const rust=spawnSync(existsSync(installedCargo)?installedCargo:"cargo",["build","--offline","--manifest-path","apps/desktop/src-tauri/Cargo.toml","--features","tauri/custom-protocol"],{cwd:root,env,stdio:"inherit",windowsHide:true});
if(rust.error)throw rust.error;if(rust.status!==0)process.exit(rust.status??1);
const output=join(root,"build","native");mkdirSync(output,{recursive:true});
try{copyFileSync(join(env.CARGO_TARGET_DIR,"debug","dungeon-world-ai-desktop.exe"),join(output,"dungeon-world-ai-desktop.exe"));}
catch(error){throw Error("Close the running native app and build again. "+String(error));}
writeFileSync(join(output,"launch-settings.json"),JSON.stringify({dataDir:env.DWA_DATA_DIR||null},null,2)+"\n","utf8");
console.log("Native app prepared. Open Start-Dungeon-World-AI.cmd from File Explorer.");
