import {spawnSync} from "node:child_process";
import {readdirSync} from "node:fs";
import {dirname,join} from "node:path";
import {fileURLToPath} from "node:url";

// The render fixtures resolve bundled assets from the repository root.
const root=join(dirname(fileURLToPath(import.meta.url)),"..");
const tests=readdirSync(join(root,"tests")).filter(name=>name.endsWith(".test.cjs")).sort().map(name=>join("tests",name));
const result=spawnSync(process.execPath,["--test",...tests],{cwd:root,stdio:"inherit",windowsHide:true});
if(result.error)throw result.error;
process.exitCode=result.status??1;
