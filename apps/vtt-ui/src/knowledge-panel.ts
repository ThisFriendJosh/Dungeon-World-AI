import {invoke,Channel} from "@tauri-apps/api/core";
import {escapeHtml as e} from "./html";
import type {Saved} from "./types";
import {prepareTextFile,titleFor,validateSource} from "./reference-import";
import type {PreparedImport} from "./reference-import";
export type KnowledgeSource={id:string;title:string;sourceRef:string;edition:string;gmOnly:boolean;chunkCount:number;importedAt:string};
type Hit={id:string;title:string;kind:string;book:string;edition:string;gmOnly:boolean;sourceRef:string;excerpt:string};
type Context={campaignId:string;render():void;action(work:()=>Promise<void>):Promise<void>;onSaved(warning:string|null):Promise<void>;onMessage(message:string):void;onDirty():void};
const editions=["unassigned","homebrew","5e-2014","5e-2024","AD&D-2e"];
const editionOptions=(selected:string)=>editions.map(value=>`<option ${value===selected?"selected":""}>${e(value)}</option>`).join("");
const base64File=(file:File)=>new Promise<string>((resolve,reject)=>{
  const reader=new FileReader();reader.onload=()=>resolve(String(reader.result).split(",",2)[1]??"");reader.onerror=()=>reject(Error("Could not read "+file.name));reader.readAsDataURL(file);
});

export class KnowledgePanel{
  private campaignId="";private sources:KnowledgeSource[]=[];private title="";private text="";private sourceRef="";
  private edition="unassigned";private gmOnly=true;private query="";private hits:Hit[]=[];private prepared:PreparedImport[]=[];
  get dirty(){return this.text.trim().length>0||this.prepared.length>0;}
  discard(){this.text="";this.title="";this.sourceRef="";this.prepared=[];}
  load(campaignId:string,sources:KnowledgeSource[]){if(this.campaignId!==campaignId){this.discard();this.query="";this.hits=[];}this.campaignId=campaignId;this.sources=sources;}
  html():string{return `<section class="knowledge-panel"><header><p class="eyebrow">Stories, books, and campaign reference</p><h2>Campaign documents</h2><p class="subtle">Add your adventures, setting lore, character references and other source material. The Dungeon Master can search their saved text during play.</p></header>
    <details class="knowledge-import" ${!this.sources.length||this.prepared.length?"open":""}><summary>Add documents or reference files</summary>
      <p>Choose PDF, Word DOCX, TXT, Markdown, CSV, HTML, general JSON, a campaign source pack, or the existing dnd-data collections. PDF text is extracted locally; scans use Windows OCR when available. Review the extracted text before saving.</p>
      <fieldset><label>Document edition<select id="knowledgeFileEdition">${editionOptions(this.edition)}</select></label><label class="inline-check"><input id="knowledgeFileGM" type="checkbox" ${this.gmOnly?"checked":""}> GM-only documents</label>
      <label>Add local files<input id="knowledgeFiles" type="file" accept=".pdf,.docx,.txt,.md,.markdown,.csv,.html,.htm,.json" multiple></label></fieldset>
      <p class="hint">Documents up to 75 MB; extracted source text up to 8 MB each. Older .doc files need a DOCX export. Files are processed on this device. Imported text is saved with this campaign.</p>
      ${this.prepared.length?`<section class="prepared-sources" aria-label="Documents ready for review"><h3>Review before adding (${this.prepared.length})</h3>
        ${this.prepared.map((item,index)=>`<article><p class="eyebrow">${e(item.method)}</p><label>Source title<input data-prepared-title="${index}" maxlength="120" value="${e(item.source.title)}"></label><p class="subtle">${e(item.source.sourceRef)} · ${item.collection?item.collection.entries.length+" reference entries":item.source.text.length.toLocaleString()+" characters"}</p>
          ${item.warnings.map(warning=>`<p class="import-warning">${e(warning)}</p>`).join("")}
          ${item.collection?`<p class="subtle">This structured collection will be added to Reference entries.</p>`:`<div class="two-columns"><label>Edition<select data-prepared-edition="${index}">${editionOptions(item.source.edition)}</select></label><label class="inline-check"><input data-prepared-gm="${index}" type="checkbox" ${item.source.gmOnly?"checked":""}> GM-only source</label></div><details><summary>Review or correct extracted text</summary><textarea aria-label="Extracted text for ${e(item.source.title)}" data-prepared-text="${index}" rows="10">${e(item.source.text)}</textarea></details>`}
          <button type="button" data-discard-prepared="${index}">Remove from import</button></article>`).join("")}
        <button type="button" id="knowledgeSavePrepared" class="primary">Add reviewed sources to Compendium</button></section>`:""}
      <form id="knowledgeForm"><fieldset><legend>Or paste a source</legend><label>Title<input id="knowledgeTitle" required maxlength="120" value="${e(this.title)}"></label>
      <label>Source filename or link<input id="knowledgeSourceRef" maxlength="1000" value="${e(this.sourceRef)}"></label><label>Edition<select id="knowledgeEdition">${editionOptions(this.edition)}</select></label>
      <label class="inline-check"><input id="knowledgeGM" type="checkbox" ${this.gmOnly?"checked":""}> GM-only source (contains secrets)</label><label>Text<textarea id="knowledgeText" rows="7" required>${e(this.text)}</textarea></label><div class="asset-actions"><button type="button" data-paste-into="knowledgeText">Paste text</button><button type="submit" class="primary">Save source for the DM</button></div></fieldset></form>
      <details><summary>Source-pack JSON format</summary><pre>${e(JSON.stringify({schemaVersion:1,sources:[{title:"Adventure title",text:"Your campaign text",sourceRef:"Original document name",edition:"homebrew",gmOnly:true}]},null,2))}</pre></details>
      <p class="hint">A source link records provenance. It does not download a website. Reference documents supply knowledge; they do not replace active character sheets or mark adventure events as completed.</p></details>
    <form id="knowledgeSearch"><label>Search all DM reference text<input id="knowledgeQuery" type="search" maxlength="500" value="${e(this.query)}" placeholder="Sunshard, Oldar, saving throw…"></label><button type="submit">Search Compendium text</button></form>
    <div class="knowledge-hits">${this.hits.map(hit=>`<article><p class="eyebrow">${e(hit.kind)} · ${hit.gmOnly?"GM only":"Reference"}</p><h3>${e(hit.title)}</h3><p class="subtle">${e(hit.book)} · ${e(hit.edition||"Edition unassigned")}</p><p>${e(hit.excerpt)}</p><small>${e(hit.sourceRef)}</small></article>`).join("")}</div>
    <h3>Saved campaign documents (${this.sources.length})</h3><ul class="knowledge-sources">${this.sources.map(source=>`<li><strong>${e(source.title)}</strong><span>${source.chunkCount} passages · ${source.gmOnly?"GM only":"Player-readable"} · ${e(source.edition||"Edition unassigned")}</span><small>${e(source.sourceRef)}</small></li>`).join("")||'<li class="subtle">Add an adventure, setting guide, NPC notes or reference book.</li>'}</ul></section>`;}
  bind(root:HTMLElement,context:Context){
    const read=()=>{this.title=root.querySelector<HTMLInputElement>("#knowledgeTitle")!.value;this.text=root.querySelector<HTMLTextAreaElement>("#knowledgeText")!.value;this.sourceRef=root.querySelector<HTMLInputElement>("#knowledgeSourceRef")!.value;this.edition=root.querySelector<HTMLSelectElement>("#knowledgeEdition")!.value;this.gmOnly=root.querySelector<HTMLInputElement>("#knowledgeGM")!.checked;context.onDirty();};
    root.querySelector("#knowledgeForm")?.addEventListener("input",read);
    root.querySelector("#knowledgeForm")?.addEventListener("submit",event=>{event.preventDefault();read();void context.action(async()=>{const request=validateSource({title:this.title,text:this.text,sourceRef:this.sourceRef,edition:this.edition,gmOnly:this.gmOnly});const result=await invoke<Saved<KnowledgeSource>>("import_knowledge",{campaignId:context.campaignId,request});this.text="";this.title="";this.sourceRef="";await context.onSaved(result.warning);});});
    root.querySelector("#knowledgeSearch")?.addEventListener("submit",event=>{event.preventDefault();this.query=root.querySelector<HTMLInputElement>("#knowledgeQuery")!.value;void context.action(async()=>{this.hits=await invoke<Hit[]>("search_dm_references",{campaignId:context.campaignId,query:this.query});context.render();});});
    root.querySelectorAll<HTMLInputElement>("[data-prepared-title]").forEach(input=>input.addEventListener("input",()=>{this.prepared[Number(input.dataset.preparedTitle)].source.title=input.value;context.onDirty();}));
    root.querySelectorAll<HTMLTextAreaElement>("[data-prepared-text]").forEach(input=>input.addEventListener("input",()=>{this.prepared[Number(input.dataset.preparedText)].source.text=input.value;context.onDirty();}));
    root.querySelectorAll<HTMLSelectElement>("[data-prepared-edition]").forEach(input=>input.addEventListener("change",()=>{this.prepared[Number(input.dataset.preparedEdition)].source.edition=input.value;context.onDirty();}));
    root.querySelectorAll<HTMLInputElement>("[data-prepared-gm]").forEach(input=>input.addEventListener("change",()=>{this.prepared[Number(input.dataset.preparedGm)].source.gmOnly=input.checked;context.onDirty();}));
    root.querySelectorAll<HTMLButtonElement>("[data-discard-prepared]").forEach(button=>button.addEventListener("click",()=>{this.prepared.splice(Number(button.dataset.discardPrepared),1);context.render();}));
    root.querySelector("#knowledgeSavePrepared")?.addEventListener("click",()=>{void context.action(async()=>{
      let count=0;let warning:string|null=null;
      try{while(this.prepared.length){const item=this.prepared[0];context.onMessage("Adding "+item.source.title+"…");
        const result=await invoke<Saved<unknown>>(item.collection?"import_content":"import_knowledge",{campaignId:context.campaignId,request:item.collection??validateSource(item.source)});
        warning=result.warning??warning;this.prepared.shift();count++;
      }}catch(error){if(count){await context.onSaved(warning);throw Error(count+" source(s) saved. Remaining imports are ready to retry: "+String(error));}throw error;}
      await context.onSaved(warning);
    });});
    root.querySelector<HTMLInputElement>("#knowledgeFiles")?.addEventListener("change",event=>{
      const input=event.target as HTMLInputElement,files=[...input.files??[]];if(!files.length)return;
      const edition=root.querySelector<HTMLSelectElement>("#knowledgeFileEdition")!.value,gmOnly=root.querySelector<HTMLInputElement>("#knowledgeFileGM")!.checked;
      void context.action(async()=>{
        let prepared=0;
        try{for(const file of files){
          if(file.size>75_000_000)throw Error(file.name+" exceeds 75 MB. Split it into smaller files.");
          context.onMessage("Reading "+file.name+"…");let items:PreparedImport[];
          if(/\.(pdf|docx)$/i.test(file.name)){
            const progress=new Channel<{page:number;total:number}>();progress.onmessage=value=>context.onMessage("Reading "+file.name+" — page "+value.page+" of "+value.total+"…");
            const document=await invoke<{text:string;method:string;pages:number;warnings:string[]}>("prepare_reference_document",{name:file.name,dataBase64:await base64File(file),onProgress:progress});
            items=[{source:validateSource({title:titleFor(file.name),text:document.text,sourceRef:file.name,edition,gmOnly}),method:document.method+(document.pages?" · "+document.pages+" pages":""),warnings:document.warnings}];
          }else{items=prepareTextFile(file.name,await file.text(),edition,gmOnly);}
          if(this.prepared.length+items.length>200)throw Error("Review and save the current documents before adding more (200 sources at a time).");
          this.prepared.push(...items);prepared+=items.length;
        }}catch(error){if(prepared){context.render();throw Error(prepared+" source(s) ready for review. "+String(error));}throw error;}
        finally{input.value="";}
        context.onMessage(prepared+" source(s) ready. Review their text, then add them to Compendium.");context.render();
      });
    });
  }
}
