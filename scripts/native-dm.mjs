import {existsSync,readdirSync,readFileSync,statSync} from "node:fs";
import {createHash} from "node:crypto";
import {homedir} from "node:os";
import {join,dirname} from "node:path";
import {fileURLToPath} from "node:url";
import {spawn} from "node:child_process";
import {readNativeSettings,resolveNativeLaunch} from "./native-launch-settings.mjs";

const root=join(dirname(fileURLToPath(import.meta.url)),"..");
const env={...process.env};
const settings=readNativeSettings(root);
const dataArg=process.argv.indexOf("--data");
if(dataArg>=0){if(!process.argv[dataArg+1])throw Error("Provide a save folder after --data.");env.DWA_DATA_DIR=process.argv[dataArg+1];}
const launch=resolveNativeLaunch(root,settings,env);
if(launch.dataDir)env.DWA_DATA_DIR=launch.dataDir;
const executable=launch.executable;
if(!existsSync(executable))throw Error("Build the native app first: npm run build:native");
if(process.argv.includes("--check")){
  console.log(JSON.stringify({executable,dataDir:launch.dataDir,bytes:statSync(executable).size,sha256:createHash("sha256").update(readFileSync(executable)).digest("hex")},null,2));
  process.exit(0);
}
if(!env.LOCAL_LLM_SERVER_PATH){
  const runtime=join(homedir(),".unsloth","llama.cpp","build","bin","Release","llama-server.exe");
  if(existsSync(runtime))env.LOCAL_LLM_SERVER_PATH=runtime;
}
if(!env.LOCAL_LLM_MODEL_PATH){
  const cache=join(homedir(),".cache","huggingface","hub","models--unsloth--Qwen3-8B-GGUF","snapshots");
  if(existsSync(cache))for(const folder of readdirSync(cache,{withFileTypes:true}).filter(e=>e.isDirectory()).sort((a,b)=>a.name.localeCompare(b.name))){
    const model=join(cache,folder.name,"Qwen3-8B-Q4_K_M.gguf");if(existsSync(model)){env.LOCAL_LLM_MODEL_PATH=model;break;}
  }
}
const wait=process.argv.includes("--wait");
const child=spawn(executable,[],{cwd:root,env,stdio:"ignore",detached:!wait,windowsHide:false});
child.on("error",error=>{console.error("Could not open Dungeon World AI: "+error.message);process.exitCode=1;});
if(wait)child.on("exit",code=>{process.exitCode=code??1;});else child.unref();
