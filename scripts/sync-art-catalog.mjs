import {readFileSync,writeFileSync} from "node:fs";
import {resolve,dirname} from "node:path";
import {fileURLToPath} from "node:url";
const root=resolve(dirname(fileURLToPath(import.meta.url)),"..");
const catalog=JSON.parse(readFileSync(resolve(root,"packages/shared/assets/catalog.json"),"utf8"));
const output="// Generated from packages/shared/assets/catalog.json. Original artwork is unchanged.\nexport const bundledArt = "+JSON.stringify(catalog.assets,null,2)+" as const;\n";
const target=resolve(root,"apps/vtt-ui/src/bundled-art-data.ts");
if(process.argv.includes("--check")){
 if(readFileSync(target,"utf8").replace(/\r\n/g,"\n")!==output){console.error("Bundled art data is stale. Run node scripts/sync-art-catalog.mjs");process.exitCode=1;}
 else console.log("Bundled art catalog is current.");
}else{writeFileSync(target,output);console.log("Updated bundled art catalog.");}
