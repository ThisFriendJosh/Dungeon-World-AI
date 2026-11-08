import {existsSync,readFileSync} from "node:fs";
import {join,resolve} from "node:path";

const appName=/^dungeon-world-ai-desktop(?:-\d+|-\d+\.\d+\.\d+(?:-beta\.\d+)?)?\.exe$/i;
function validateSettings(settings){
  if(!settings||typeof settings!=="object"||Array.isArray(settings))throw Error("The native launch settings must be a JSON object.");
  if(settings.dataDir!==undefined&&settings.dataDir!==null&&typeof settings.dataDir!=="string")throw Error("The saved campaign folder in launch-settings.json is invalid.");
}
function validateAppFile(value){
  if(typeof value!=="string"||!appName.test(value))throw Error("The configured native app filename is invalid. Rebuild the app or correct launch-settings.json; no older executable was opened.");
  return value;
}
export function readNativeSettings(root){
  const file=join(root,"build","native","launch-settings.json");
  if(!existsSync(file))return {};
  const settings=JSON.parse(readFileSync(file,"utf8").replace(/^\uFEFF/,""));
  validateSettings(settings);return settings;
}
export function resolveNativeLaunch(root,settings={},env={}){
  validateSettings(settings);
  const appFile=validateAppFile(settings.appFile===undefined?"dungeon-world-ai-desktop.exe":settings.appFile);
  const override=env.DWA_NATIVE_APP_PATH?.trim();
  return {appFile,executable:override?resolve(root,override):join(root,"build","native",appFile),dataDir:env.DWA_DATA_DIR?.trim()||settings.dataDir?.trim()||null};
}
export function updatedNativeSettings(previous,env,appFile){
  validateSettings(previous);validateAppFile(appFile);
  return {...previous,appFile,dataDir:env.DWA_DATA_DIR?.trim()||previous.dataDir?.trim()||null};
}
