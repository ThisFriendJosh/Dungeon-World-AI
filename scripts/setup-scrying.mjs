import {existsSync} from "node:fs";
import {homedir} from "node:os";
import {dirname,join} from "node:path";
import {fileURLToPath} from "node:url";
import {spawnSync} from "node:child_process";
const root=join(dirname(fileURLToPath(import.meta.url)),"..");
const python=process.env.DWA_SETUP_PYTHON_PATH||process.env.DWA_ART_PYTHON_PATH||join(homedir(),".unsloth","studio","unsloth_studio","Scripts","python.exe");
if(!existsSync(python))throw Error("Set DWA_SETUP_PYTHON_PATH to a Python 3.11+ executable for the one-time package setup.");
const result=spawnSync(python,["-I",join(root,"scripts","setup_scrying.py")],{cwd:root,stdio:"inherit",windowsHide:true});if(result.error)throw result.error;process.exitCode=result.status??1;
