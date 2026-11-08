import {existsSync,readdirSync,readFileSync} from "node:fs";
import {homedir} from "node:os";
import {join,dirname} from "node:path";
import {fileURLToPath} from "node:url";
import {spawn} from "node:child_process";

const root=join(dirname(fileURLToPath(import.meta.url)),"..");
const env={...process.env};
const settingsFile=join(root,"build","native","launch-settings.json");
if(!env.DWA_DATA_DIR&&existsSync(settingsFile)){
  const settings=JSON.parse(readFileSync(settingsFile,"utf8"));
  if(typeof settings.dataDir==="string"&&settings.dataDir.trim())env.DWA_DATA_DIR=settings.dataDir;
}
const executable=env.DWA_NATIVE_APP_PATH||join(root,"build","native","dungeon-world-ai-desktop.exe");
if(!existsSync(executable))throw Error("Build the native app first: npm run build:native");
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
// Default application saves are used unless the user explicitly supplies a test folder.
const dataArg=process.argv.indexOf("--data");
if(dataArg>=0){if(!process.argv[dataArg+1])throw Error("Provide a save folder after --data.");env.DWA_DATA_DIR=process.argv[dataArg+1];}
const wait=process.argv.includes("--wait");
const child=spawn(executable,[],{cwd:root,env,stdio:"ignore",detached:!wait,windowsHide:false});
child.on("error",error=>{console.error("Could not open Dungeon World AI: "+error.message);process.exitCode=1;});
if(wait)child.on("exit",code=>{process.exitCode=code??1;});else child.unref();
