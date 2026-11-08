import {invoke} from "@tauri-apps/api/core";
import type {Saved} from "./types";
import {escapeHtml as e} from "./html";
type Pack={id:string;name:string;kind:string;entryCount:number;importedAt:string};
type Entry={id:string;name:string;kind:string;book:string;publisher:string;sourceEdition:string};
type Results={total:number;entries:Entry[]};
type Detail={entry:{name:string;description:string;properties:Record<string,unknown>|null;publisher:string;book:string};sourceEdition:string;kind:string};
type Context={campaignId:string;action(work:()=>Promise<void>):void;render():void;isActive():boolean;onError(message:string):void;onMessage(message:string):void;onSaved(warning:string|null):Promise<void>};
const kinds=["backgrounds","classes","items","monsters","species","spells"];
const editionLabel=(value:string)=>value==="5e-2024"?"2024 / 2025 source title":value==="5e-2014"?"2014 source title":"Year not specified";
const format=(value:unknown)=>typeof value==="string"?value:JSON.stringify(value,null,2);
export class Compendium {
 invalidate(){this.ready=false;}
 private campaignId="";private packs:Pack[]=[];private books:string[]=[];private results:Results={total:0,entries:[]};
 private detail:Detail|null=null;private ready=false;private pending=false;
 private query={search:"",kind:"",edition:"",book:"",offset:0};
 selectCampaign(id:string){
  if(this.campaignId===id)return;
  this.campaignId=id;this.ready=false;this.pending=false;this.packs=[];this.books=[];this.results={total:0,entries:[]};this.detail=null;this.query={search:"",kind:"",edition:"",book:"",offset:0};
 }
 private async refresh(context:Context){
  const id=context.campaignId;
  const [packs,books,results]=await Promise.all([
   invoke<Pack[]>("list_content_packs",{campaignId:id}),invoke<string[]>("list_content_books",{campaignId:id}),
   invoke<Results>("search_content",{campaignId:id,query:this.query})
  ]);
  if(this.campaignId!==id)return;
  this.packs=packs;this.books=books;this.results=results;this.ready=true;
 }
 html():string{
  return `<section class="compendium"><header><p class="eyebrow">Your local collection</p><h2>Reference entries</h2><p class="subtle">Find spells, creatures, equipment, character options, and their original sources. Add books, adventures and other file types in Campaign documents & DM library.</p></header>
   <details class="compendium-import" ${!this.packs.length?"open":""}><summary>Import dnd-data JSON collections</summary>
    <p>Extract the supplied dnd-data archive, then choose one or more files from its <strong>data</strong> folder. Accepted files: backgrounds.json, classes.json, items.json, monsters.json, species.json, and spells.json.</p>
    <fieldset><label>Local JSON files<input id="contentFiles" type="file" accept=".json,application/json" multiple></label></fieldset>
    <p class="hint">Imports stay with this campaign on this device. Publisher, book, and original properties are retained. Reference entries do not change character builds or apply effects automatically.</p>
    <p class="hint">A printed year identifies a source title, not a verified rule revision. Entries without a year stay unassigned. Use the saved campaign rules profile when adjudicating play.</p>
   </details>
   <div class="compendium-packs" aria-label="Imported collections">${this.packs.map(p=>`<span>${e(p.name)} <strong>${p.entryCount.toLocaleString()}</strong></span>`).join("")||'<p class="subtle">No collections imported yet.</p>'}</div>
   <form id="contentSearch"><fieldset class="content-search"><label>Find by name or publisher<input id="contentQuery" type="search" maxlength="120" value="${e(this.query.search)}" placeholder="Fireball, goblin, longsword…"></label>
    <label>Dataset<select id="contentKind"><option value="">All datasets</option>${kinds.map(v=>`<option value="${v}" ${v===this.query.kind?"selected":""}>${v}</option>`).join("")}</select></label>
    <label>Source year<select id="contentEdition">${[["","All source years"],["5e-2024","2024 / 2025 in source title"],["5e-2014","2014 in source title"],["unassigned","Year not specified"]].map(([v,label])=>`<option value="${v}" ${v===this.query.edition?"selected":""}>${label}</option>`).join("")}</select></label>
    <label>Source book<select id="contentBook"><option value="">All books</option>${this.books.map(v=>`<option value="${e(v)}" ${v===this.query.book?"selected":""}>${e(v||"Unspecified")}</option>`).join("")}</select></label><button type="submit" class="primary">Search collection</button>
   </fieldset></form>
   <div class="content-layout"><section class="content-list" aria-label="Compendium results"><p class="subtle">${this.pending?"Loading local collection…":this.results.total.toLocaleString()+" matching entries"}</p>
    ${this.results.entries.map(v=>`<button type="button" data-content-entry="${e(v.id)}"><strong>${e(v.name)}</strong><span>${e(v.kind)} · ${e(v.book||"Unspecified source")}</span><small>${e(v.publisher||"Unspecified publisher")} · ${e(editionLabel(v.sourceEdition))}</small></button>`).join("")}
    ${this.results.total?`<nav class="content-pagination" aria-label="Result pages"><button type="button" id="contentPrevious" ${this.query.offset===0?"disabled":""}>Previous</button><span>${this.query.offset+1}–${Math.min(this.query.offset+30,this.results.total)} of ${this.results.total.toLocaleString()}</span><button type="button" id="contentNext" ${this.query.offset+30>=this.results.total?"disabled":""}>Next</button></nav>`:""}
   </section>
   <article class="content-detail" aria-label="Reference entry">${this.detail?`<p class="eyebrow">${e(this.detail.kind)} · reference entry</p><h3>${e(this.detail.entry.name)}</h3><p class="content-source">${e(this.detail.entry.book||"Unspecified source")}<br>${e(this.detail.entry.publisher||"Unspecified publisher")} · ${e(editionLabel(this.detail.sourceEdition))}</p><dl>${Object.entries(this.detail.entry.properties??{}).map(([key,value])=>`<div><dt>${e(key)}</dt><dd>${e(format(value))}</dd></div>`).join("")}</dl><div class="content-description">${e(this.detail.entry.description)}</div>`:'<p class="eyebrow">Reference desk</p><h3>Open an entry</h3><p>Select a result to read its original description and properties. Different books may contain entries with the same name; each remains a separate source entry.</p>'}</article>
   </div>
  </section>`;
 }
 bind(root:HTMLElement,context:Context){
  this.selectCampaign(context.campaignId);
  const search=async()=>{this.detail=null;await this.refresh(context);if(context.isActive())context.render();};
  root.querySelector("#contentSearch")?.addEventListener("submit",event=>{event.preventDefault();this.query={search:root.querySelector<HTMLInputElement>("#contentQuery")!.value,kind:root.querySelector<HTMLSelectElement>("#contentKind")!.value,edition:root.querySelector<HTMLSelectElement>("#contentEdition")!.value,book:root.querySelector<HTMLSelectElement>("#contentBook")!.value,offset:0};void context.action(search);});
  root.querySelector("#contentPrevious")?.addEventListener("click",()=>{this.query.offset=Math.max(0,this.query.offset-30);void context.action(search);});
  root.querySelector("#contentNext")?.addEventListener("click",()=>{this.query.offset+=30;void context.action(search);});
  for(const button of root.querySelectorAll<HTMLButtonElement>("[data-content-entry]"))button.addEventListener("click",()=>{const entryId=button.dataset.contentEntry!;void context.action(async()=>{this.detail=await invoke<Detail>("load_content_entry",{campaignId:context.campaignId,entryId});if(context.isActive())context.render();});});
  root.querySelector<HTMLInputElement>("#contentFiles")?.addEventListener("change",event=>{
   const files=[...(event.target as HTMLInputElement).files??[]];if(!files.length)return;
   void context.action(async()=>{
    let warning:string|null=null;let imported=0;
    try { for(const file of files){
     const kind=file.name.replace(/\.json$/i,"").toLowerCase();
     if(!kinds.includes(kind))throw Error("Choose supported JSON files from the dnd-data data folder.");
     if(file.size>40_000_000)throw Error(file.name+" exceeds the 40 MB file limit.");
     context.onMessage("Importing "+file.name+"…");
     const data=JSON.parse(await file.text()) as unknown;
     if(!Array.isArray(data))throw Error(file.name+" must contain a JSON array.");
     const result=await invoke<Saved<Pack>>("import_content",{campaignId:context.campaignId,request:{name:file.name,kind,entries:data}});
     warning=result.warning??warning;imported++;
    }} catch(error){if(imported){await this.refresh(context);if(context.isActive())context.render();throw Error(imported+" collection(s) were saved before an import failed: "+String(error));}throw error;}
    this.query.offset=0;await this.refresh(context);await context.onSaved(warning);
   });
  });
  if(!this.ready&&!this.pending){
   const id=context.campaignId;this.pending=true;void this.refresh(context).catch(error=>{if(this.campaignId===id){this.ready=true;context.onError(String(error));}}).finally(()=>{if(this.campaignId===id){this.pending=false;if(context.isActive())context.render();}});
  }
 }
}
