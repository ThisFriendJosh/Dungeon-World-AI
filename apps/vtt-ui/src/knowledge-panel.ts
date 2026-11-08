import {invoke} from "@tauri-apps/api/core";
import {escapeHtml as e} from "./html";
import type {Saved} from "./types";
export type KnowledgeSource={id:string;title:string;sourceRef:string;edition:string;gmOnly:boolean;chunkCount:number;importedAt:string};
type Hit={id:string;title:string;kind:string;book:string;edition:string;gmOnly:boolean;sourceRef:string;excerpt:string};
type Context={campaignId:string;render():void;action(work:()=>Promise<void>):Promise<void>;onSaved(warning:string|null):Promise<void>;onMessage(message:string):void;onDirty():void};
export class KnowledgePanel{
  private campaignId="";private sources:KnowledgeSource[]=[];private title="";private text="";private sourceRef="";private edition="homebrew";private gmOnly=true;private query="";private hits:Hit[]=[];
  get dirty(){return this.text.trim().length>0;}
  discard(){this.text="";this.title="";this.sourceRef="";}
  load(campaignId:string,sources:KnowledgeSource[]){if(this.campaignId!==campaignId){this.discard();this.query="";this.hits=[];}this.campaignId=campaignId;this.sources=sources;}
  html():string{return `<section class="knowledge-panel"><header><p class="eyebrow">Story, lore, and source material</p><h2>DM library</h2><p class="subtle">Import local campaign text and search the same material the Dungeon Master can retrieve. Sources stay with this campaign.</p></header>
    <details class="knowledge-import" ${!this.sources.length?"open":""}><summary>Add campaign or reference material</summary>
      <p>Choose TXT, Markdown, or a campaign source-pack JSON. For a PDF or Word book, extract its text into a source pack first; images and layout need separate review.</p><label>Import local sources<input id="knowledgeFiles" type="file" accept=".txt,.md,.json,text/plain,text/markdown,application/json" multiple></label>
      <form id="knowledgeForm"><fieldset><legend>Or paste a source</legend><label>Title<input id="knowledgeTitle" required maxlength="120" value="${e(this.title)}"></label>
      <label>Source filename or link<input id="knowledgeSourceRef" maxlength="1000" value="${e(this.sourceRef)}"></label><label>Edition<select id="knowledgeEdition">${["homebrew","unassigned","5e-2014","5e-2024","AD&D-2e"].map(value=>`<option ${value===this.edition?"selected":""}>${e(value)}</option>`).join("")}</select></label>
      <label class="inline-check"><input id="knowledgeGM" type="checkbox" ${this.gmOnly?"checked":""}> GM-only source (contains secrets)</label><label>Text<textarea id="knowledgeText" rows="7" required>${e(this.text)}</textarea></label><button type="submit" class="primary">Save source for the DM</button></fieldset></form>
      <details><summary>Source-pack JSON format</summary><pre>${e(JSON.stringify({schemaVersion:1,sources:[{title:"Adventure title",text:"Your campaign text",sourceRef:"Original document name",edition:"homebrew",gmOnly:true}]},null,2))}</pre></details>
      <p class="hint">A source link records provenance; it does not download material. The AI reads indexed passages as needed. Keep edition labels and GM secrets accurate.</p></details>
    <form id="knowledgeSearch"><label>Search campaign text, compendium, and rules<input id="knowledgeQuery" type="search" maxlength="500" value="${e(this.query)}" placeholder="Sunshard, Oldar, saving throw…"></label><button type="submit">Search DM library</button></form>
    <div class="knowledge-hits">${this.hits.map(hit=>`<article><p class="eyebrow">${e(hit.kind)} · ${hit.gmOnly?"GM only":"Reference"}</p><h3>${e(hit.title)}</h3><p class="subtle">${e(hit.book)} · ${e(hit.edition||"Edition unassigned")}</p><p>${e(hit.excerpt)}</p><small>${e(hit.sourceRef)}</small></article>`).join("")}</div>
    <h3>Campaign sources (${this.sources.length})</h3><ul class="knowledge-sources">${this.sources.map(source=>`<li><strong>${e(source.title)}</strong><span>${source.chunkCount} passages · ${source.gmOnly?"GM only":"Player-readable"} · ${e(source.edition||"Edition unassigned")}</span><small>${e(source.sourceRef)}</small></li>`).join("")||'<li class="subtle">Add a campaign outline, adventure, NPC guide, or local reference document.</li>'}</ul></section>`;}
  bind(root:HTMLElement,context:Context){
    const read=()=>{this.title=root.querySelector<HTMLInputElement>("#knowledgeTitle")!.value;this.text=root.querySelector<HTMLTextAreaElement>("#knowledgeText")!.value;this.sourceRef=root.querySelector<HTMLInputElement>("#knowledgeSourceRef")!.value;this.edition=root.querySelector<HTMLSelectElement>("#knowledgeEdition")!.value;this.gmOnly=root.querySelector<HTMLInputElement>("#knowledgeGM")!.checked;context.onDirty();};
    root.querySelector("#knowledgeForm")?.addEventListener("input",read);
    root.querySelector("#knowledgeForm")?.addEventListener("submit",event=>{event.preventDefault();read();void context.action(async()=>{const result=await invoke<Saved<KnowledgeSource>>("import_knowledge",{campaignId:context.campaignId,request:{title:this.title,text:this.text,sourceRef:this.sourceRef,edition:this.edition,gmOnly:this.gmOnly}});this.discard();await context.onSaved(result.warning);});});
    root.querySelector("#knowledgeSearch")?.addEventListener("submit",event=>{event.preventDefault();this.query=root.querySelector<HTMLInputElement>("#knowledgeQuery")!.value;void context.action(async()=>{this.hits=await invoke<Hit[]>("search_dm_references",{campaignId:context.campaignId,query:this.query});context.render();});});
    root.querySelector<HTMLInputElement>("#knowledgeFiles")?.addEventListener("change",event=>{
      const files=[...(event.target as HTMLInputElement).files??[]];if(!files.length)return;
      void context.action(async()=>{let count=0;let warning:string|null=null;
        try{for(const file of files){if(file.size>8_000_000)throw Error("Choose source files up to 8 MB each.");
          const text=await file.text();let sources:unknown;
          if(/\.json$/i.test(file.name)){const pack=JSON.parse(text);if(pack.schemaVersion!==1||!Array.isArray(pack.sources))throw Error("Choose a campaign source pack with schemaVersion 1 and a sources array. Use Compendium for dnd-data collections.");sources=pack.sources;if(Array.isArray(pack.notIndexed)&&pack.notIndexed.length)warning=pack.notIndexed.map((item:{source?:unknown;reason?:unknown})=>String(item.source??"Source")+": "+String(item.reason??"Not indexed")).join(" ");}
          else if(/\.(txt|md)$/i.test(file.name))sources=[{title:file.name.slice(0,120),text,sourceRef:file.name,edition:"unassigned",gmOnly:true}];else throw Error("Choose TXT, Markdown, or a source-pack JSON.");
          if(!Array.isArray(sources)||sources.length<1||sources.length>200)throw Error("A pack must contain 1–200 sources.");
          for(const source of sources){context.onMessage("Saving source "+(count+1)+"…");const result=await invoke<Saved<KnowledgeSource>>("import_knowledge",{campaignId:context.campaignId,request:source});warning=result.warning??warning;count++;}
        }}catch(error){if(count){await context.onSaved(warning);throw Error(count+" sources saved before an import failed: "+String(error));}throw error;}
        await context.onSaved(warning);
      });
    });
  }
}
