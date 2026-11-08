import {invoke,isTauri} from "@tauri-apps/api/core";

type TextField=HTMLInputElement|HTMLTextAreaElement;
const nativeClipboard=()=>isTauri()&&/Windows/i.test(navigator.userAgent);
const editable=(value:EventTarget|null):value is TextField=>
  (value instanceof HTMLTextAreaElement||value instanceof HTMLInputElement)
  &&!value.disabled&&!value.readOnly&&value.type!=="password"&&value.selectionStart!==null;

export function replacement(value:string,start:number,end:number,text:string,maxLength:number){
  const room=maxLength<0?text.length:Math.max(0,maxLength-value.length+end-start);
  let insertion=text.slice(0,room);
  if(insertion.length<text.length&&/[\uD800-\uDBFF]$/.test(insertion)&&/^[\uDC00-\uDFFF]/.test(text.slice(insertion.length)))insertion=insertion.slice(0,-1);
  return {value:value.slice(0,start)+insertion+value.slice(end),insertion,cursor:start+insertion.length,truncated:insertion.length<text.length};
}

export async function pasteInto(field:TextField):Promise<boolean>{
  if(!editable(field))return false;
  const before=field.value,start=field.selectionStart!,end=field.selectionEnd!;
  const text=nativeClipboard()?await invoke<string>("read_clipboard_text"):await navigator.clipboard.readText();
  if(!field.isConnected||field.value!==before)throw Error("The field changed while pasting. Click it and paste again.");
  const next=replacement(before,start,end,text,field.maxLength);
  field.focus();field.setSelectionRange(start,end);
  // Keep the browser's undo history when supported, with a selection-safe fallback.
  if(!document.execCommand("insertText",false,next.insertion)&&field.value===before){
    field.setRangeText(next.insertion,start,end,"end");
  }
  field.dispatchEvent(new Event("input",{bubbles:true}));
  return next.truncated;
}

/** Only explicit edit gestures read/write the clipboard. No polling or startup reads. */
export function installClipboard(root:HTMLElement,onMessage:(message:string)=>void){
  root.addEventListener("keydown",event=>{
    if(!nativeClipboard()||!event.ctrlKey||event.altKey||event.metaKey)return;
    const key=event.key.toLowerCase();const field=editable(event.target)?event.target:null;
    if(key==="v"&&field){
      event.preventDefault();
      void pasteInto(field).then(truncated=>{if(truncated)onMessage("Pasted up to this field's length limit. Import a document for longer text.");}).catch(error=>onMessage(String(error)));
    }else if(key==="c"||key==="x"){
      const input=event.target instanceof HTMLInputElement||event.target instanceof HTMLTextAreaElement?event.target:null;
      if(input?.type==="password")return;
      const start=field?.selectionStart??0,end=field?.selectionEnd??0;
      const text=input?input.value.slice(input.selectionStart??0,input.selectionEnd??0):window.getSelection()?.toString()??"";
      if(!text||key==="x"&&!field)return;
      event.preventDefault();const original=field?.value;
      void invoke("write_clipboard_text",{text}).then(()=>{
        if(key==="x"&&field&&field.isConnected&&field.value===original){field.focus();field.setSelectionRange(start,end);if(!document.execCommand("delete")){field.setRangeText("",start,end,"end");}field.dispatchEvent(new Event("input",{bubbles:true}));}
      }).catch(error=>onMessage(String(error)));
    }
  });
  root.addEventListener("click",event=>{
    const button=(event.target as Element).closest<HTMLButtonElement>("[data-paste-into]");
    if(!button)return;
    const field=root.querySelector<TextField>("#"+button.dataset.pasteInto);
    if(!field)return;
    void pasteInto(field).then(truncated=>{if(truncated)onMessage("Pasted up to this field's length limit. Import a document for longer text.");}).catch(error=>onMessage(String(error)));
  });
}
