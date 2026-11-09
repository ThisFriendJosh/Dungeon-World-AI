import {existsSync,readdirSync} from "node:fs";
import {homedir} from "node:os";
import {join,dirname,basename} from "node:path";
import {fileURLToPath} from "node:url";
import {spawn} from "node:child_process";
import {requireInteractiveLaunch} from "./native-launch-settings.mjs";

const root=join(dirname(fileURLToPath(import.meta.url)),"..");
const env={...process.env};
requireInteractiveLaunch(root,env);
// Development convenience only: reuse the already installed workbench runtime
// and model. The application owns its own process; no Unsloth server is needed.
if(!env.LOCAL_LLM_SERVER_PATH?.trim()){
  const runtime=join(homedir(),".unsloth","llama.cpp","build","bin",process.platform==="win32"?"Release":"","llama-server"+(process.platform==="win32"?".exe":""));
  if(existsSync(runtime))env.LOCAL_LLM_SERVER_PATH=runtime;
}
if(!env.LOCAL_LLM_MODEL_PATH?.trim()){
  const snapshots=join(homedir(),".cache","huggingface","hub","models--unsloth--Qwen3-8B-GGUF","snapshots");
  if(existsSync(snapshots)){
    for(const entry of readdirSync(snapshots,{withFileTypes:true}).filter(e=>e.isDirectory()).sort((a,b)=>a.name.localeCompare(b.name))){
      const model=join(snapshots,entry.name,"Qwen3-8B-Q4_K_M.gguf");
      if(existsSync(model)){env.LOCAL_LLM_MODEL_PATH=model;break;}
    }
  }
}
for(const key of ["LOCAL_LLM_SERVER_PATH","LOCAL_LLM_MODEL_PATH"]){
  if(!env[key]||!existsSync(env[key]))throw new Error(`Set ${key} to an existing local file before running dev:dm. This launcher does not download files.`);
}
console.log("Local Dungeon Master: "+basename(env.LOCAL_LLM_MODEL_PATH));
const child=spawn(process.execPath,[join(root,"scripts","desktop.mjs"),"dev"],{cwd:root,env,stdio:"inherit",windowsHide:true});
child.on("error",error=>{console.error(error.message);process.exitCode=1;});
child.on("exit",code=>{process.exitCode=code??1;});
process.on("SIGINT",()=>child.kill("SIGINT"));
process.on("SIGTERM",()=>child.kill("SIGTERM"));
