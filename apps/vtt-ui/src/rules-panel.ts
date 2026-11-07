import {invoke} from "@tauri-apps/api/core";
import type {Saved} from "./types";
import {escapeHtml as e} from "./html";

export type RulesProfile="5e-manual"|"5e-2014"|"5e-2024";
export type RulesConfig={schemaVersion:1;profile:RulesProfile;houseRules:{flankingAdvantage:boolean;potionAsBonusAction:boolean;critMaxOneDie:boolean}};
export type RulesState={config:RulesConfig;revision:number;updatedAt:string;sourceVersion:string|null};
type RulesQuery={abilityScore:number;level:number;damage:number;exhaustion:number;speed:number};
type Calculation={profile:RulesProfile;abilityModifier:number;proficiencyBonus:number;trainedBonus:number;spellSaveDc:number;concentrationDc:number|null;exhaustionTestPenalty:number|null;speedAfterExhaustion:number|null;exhaustionState:string};
type Book={version:string;license:string;sourceUrl:string;toc:{title:string;page:number}[];pages:{number:number;text:string}[]};
type Context={campaignId:string;action(work:()=>Promise<void>):void;render():void;isActive():boolean;onDirty():void;onError(message:string):void;onSaved(warning:string|null):Promise<void>};
export const profileLabel=(profile:RulesProfile)=>({"5e-manual":"Manual / choose edition","5e-2014":"5e · 2014 rules","5e-2024":"5.5e · revised 2024 rules"})[profile];
const signed=(n:number)=>n>=0?"+"+n:String(n);

export class RulesPanel {
 private saved:RulesState|null=null;
 private draft:RulesConfig|null=null;
 dirty=false;
 private book:Book|null=null;private loading=false;private bookError="";
 private search="";private page=5;private original=false;private legal=false;
 private query:RulesQuery={abilityScore:16,level:1,damage:20,exhaustion:0,speed:30};
 private result:Calculation|null=null;
 load(value:RulesState,preserve=false){
  const keep=preserve&&this.saved!==null&&this.dirty;
  this.saved=structuredClone(value);
  if(!keep){this.draft=structuredClone(value.config);this.dirty=false;this.result=null;}
 }
 discard(){if(this.saved)this.draft=structuredClone(this.saved.config);this.dirty=false;}
 label(){return this.saved?profileLabel(this.saved.config.profile):"Rules profile";}
 html():string{
  if(!this.saved||!this.draft)return '<p class="empty-note">Open a campaign to choose its rules.</p>';
  const draft=this.draft,profile=this.saved.config.profile,hr=draft.houseRules;
  const result=this.result;
  const answers=result?[
   ["Ability modifier",signed(result.abilityModifier)],["Proficiency bonus",signed(result.proficiencyBonus)],
   ["Proficient check / attack",signed(result.trainedBonus)],["Spell save DC",String(result.spellSaveDc)],
   ["Concentration save DC",this.query.damage===0?"No damage check":result.concentrationDc===null?"Choose an edition":String(result.concentrationDc)],
   ["Exhaustion D20 penalty",result.exhaustionState==="dead"?"Dead":result.exhaustionTestPenalty===null?"Resolve manually":signed(result.exhaustionTestPenalty)],
   ["Speed with exhaustion",result.speedAfterExhaustion===null?"Resolve manually":result.speedAfterExhaustion+" ft"]
  ]:null;
  const page=this.book?.pages.find(p=>p.number===this.page);
  const matching=this.book&&this.search.trim()?this.book.pages.filter(p=>p.text.toLowerCase().includes(this.search.trim().toLowerCase())):[];
  return `<section class="rules-panel">
   <header><p class="eyebrow">The rules at your table</p><h2>Rules & reference</h2><p class="subtle">Current campaign: ${e(this.label())}. Edition changes keep your saved characters, scenes, and rolls intact.</p></header>
   <div class="rules-tools">
    <form id="rulesSettings" class="rules-card"><h3>Campaign rules profile</h3>
     <label>Edition<select id="rulesProfile">${(["5e-manual","5e-2014","5e-2024"] as RulesProfile[]).map(p=>`<option value="${p}" ${draft.profile===p?"selected":""}>${e(profileLabel(p))}</option>`).join("")}</select></label>
     <p class="hint">New campaigns start with the revised rules. Older saves stay manual until you choose their edition. Changing a profile does not convert a character build.</p>
     <fieldset class="house-rules"><legend>Table preferences</legend>
      <label><input type="checkbox" id="flankingAdvantage" ${hr.flankingAdvantage?"checked":""}> Flanking grants advantage</label>
      <label><input type="checkbox" id="potionAsBonusAction" ${hr.potionAsBonusAction?"checked":""}> Drink a potion as a Bonus Action</label>
      <label><input type="checkbox" id="critMaxOneDie" ${hr.critMaxOneDie?"checked":""}> Maximize one critical damage die</label>
     </fieldset>
     <p class="hint">These preferences are recorded for your table. Combat automation does not apply them yet.</p>
     <button type="submit" class="primary">Save rules profile</button>
    </form>
    <form id="rulesCalculator" class="rules-card"><h3>Rules calculator</h3><p class="hint">Uses the saved campaign profile. Base values assume proficiency and exclude temporary bonuses.</p>
     <div class="rules-inputs">${[["abilityScore","Ability score",1,30],["level","Character level",1,20],["damage","Damage taken (one source)",0,10000],["exhaustion","Exhaustion level",0,6],["speed","Base speed (ft)",0,1000]].map(([key,label,min,max])=>`<label>${label}<input data-rules-number="${key}" type="number" required min="${min}" max="${max}" step="1" value="${this.query[key as keyof RulesQuery]}"></label>`).join("")}</div>
     <button type="submit">Calculate</button>
     ${this.dirty?'<p class="hint">Save your pending profile selection before calculating with it.</p>':""}
     ${answers?`<dl class="rules-results">${answers.map(([label,value])=>`<div><dt>${label}</dt><dd>${e(value)}</dd></div>`).join("")}</dl><p class="hint">Exhaustion is shown separately; subtract its penalty from the relevant D20 Test. Concentration assumes damage interrupted an active spell.</p>`:""}
     ${profile==="5e-manual"?'<p class="hint">Edition-specific concentration and exhaustion remain manual until an edition is chosen.</p>':""}
    </form>
   </div>
   <section class="rules-book"><header class="section-heading"><div><p class="eyebrow">Available offline · 364 pages</p><h3>System Reference Document 5.2.1</h3></div><span class="reference-badge">5.5e / revised core</span></header>
    <p class="subtle">The complete open SRD is included as a reference. Class features, spells, monster abilities, and combat turns are adjudicated manually; this reader does not automate the full rulebook.</p>
    ${profile==="5e-2014"?'<p class="art-reference-note">Your campaign uses 2014 rules. This book describes the revised rules; check the edition before applying an entry.</p>':""}
    ${!this.book?`<p class="subtle">${e(this.bookError||"Loading the local rulebook…")}</p>${this.bookError?'<button type="button" id="retryRulesBook">Retry loading</button>':""}`:`
     <div class="rules-book-tools"><label>Search this book<input type="search" id="rulesSearch" placeholder="Concentration, grapple, exhaustion…" value="${e(this.search)}"></label><label>Chapter<select id="rulesChapter"><option value="">Jump to a chapter</option>${this.book.toc.map(c=>`<option value="${c.page}">${e(c.title)} · p. ${c.page}</option>`).join("")}</select></label></div>
     ${this.search.trim()?`<div class="rules-search-results" aria-label="Rulebook search results"><p>${matching.length} matching page${matching.length===1?"":"s"}</p>${matching.map(p=>{const n=p.text.toLowerCase().indexOf(this.search.trim().toLowerCase());return `<button type="button" data-rules-page="${p.number}"><strong>Page ${p.number}</strong><span>${e(p.text.slice(Math.max(0,n-45),n+160).replace(/\s+/g," "))}</span></button>`;}).join("")}</div>`:""}
     <div class="rules-page-toolbar"><button type="button" id="rulesPrevious" ${this.page<=1?"disabled":""}>Previous</button><label>Page<input id="rulesPage" type="number" min="1" max="364" step="1" value="${this.page}"></label><span>of 364</span><button type="button" id="rulesNext" ${this.page>=364?"disabled":""}>Next</button><button type="button" id="rulesOriginal">${this.original?"Read text":"View original PDF"}</button></div>
     ${this.original?`<iframe class="rules-pdf" title="Original SRD 5.2.1 PDF" src="/rules/srd-5.2.1.pdf#page=${this.page}"></iframe><p class="hint">If this desktop webview cannot display PDFs, use Read text above.</p>`:`<article class="rules-page" aria-label="SRD page ${this.page}"><p class="rules-page-title">SRD 5.2.1 · page ${this.page}</p><div>${e(page?.text??"Page unavailable.")}</div></article>`}
    `}
    <footer class="rules-attribution">SRD 5.2.1 · CC BY 4.0. The original PDF is unchanged; searchable text is extracted for this reader. <button type="button" id="rulesLegal">Attribution & license</button>${this.legal?'<iframe class="rules-license" title="SRD attribution and license" src="/rules/attribution.html"></iframe>':""}</footer>
   </section>
  </section>`;
 }
 private async loadBook(context:Context){
  if(this.book||this.loading)return;this.loading=true;this.bookError="";
  try{
   const response=await fetch("/rules/srd-5.2.1.json");
   if(!response.ok)throw Error("Could not load the bundled SRD.");
   const book=await response.json() as Book;
   if(book.version!=="5.2.1"||book.pages?.length!==364||!Array.isArray(book.toc))throw Error("The bundled SRD has an unsupported format.");
   this.book=book;
  }catch(error){this.bookError=String(error);}
  finally{this.loading=false;if(context.isActive())context.render();}
 }
 bind(root:HTMLElement,context:Context){
  const form=root.querySelector<HTMLFormElement>("#rulesSettings");
  const readConfig=():RulesConfig=>({schemaVersion:1,profile:(root.querySelector<HTMLSelectElement>("#rulesProfile")!.value as RulesProfile),houseRules:{
   flankingAdvantage:root.querySelector<HTMLInputElement>("#flankingAdvantage")!.checked,
   potionAsBonusAction:root.querySelector<HTMLInputElement>("#potionAsBonusAction")!.checked,
   critMaxOneDie:root.querySelector<HTMLInputElement>("#critMaxOneDie")!.checked
  }});
  form?.addEventListener("change",()=>{this.draft=readConfig();this.dirty=true;context.onDirty();});
  form?.addEventListener("submit",event=>{event.preventDefault();const config=readConfig(),revision=this.saved!.revision;void context.action(async()=>{
   const result=await invoke<Saved<RulesState>>("save_rules_settings",{campaignId:context.campaignId,config,expectedRevision:revision});
   this.load(result.value);await context.onSaved(result.warning);
  });});
  root.querySelector<HTMLFormElement>("#rulesCalculator")?.addEventListener("submit",event=>{event.preventDefault();for(const input of root.querySelectorAll<HTMLInputElement>("[data-rules-number]"))this.query[input.dataset.rulesNumber as keyof RulesQuery]=Number(input.value);
   void context.action(async()=>{this.result=await invoke<Calculation>("calculate_rules",{campaignId:context.campaignId,query:this.query});context.render();});
  });
  for(const input of root.querySelectorAll<HTMLInputElement>("[data-rules-number]"))input.addEventListener("input",()=>{this.query[input.dataset.rulesNumber as keyof RulesQuery]=Number(input.value);this.result=null;root.querySelector(".rules-results")?.remove();});
  const search=root.querySelector<HTMLInputElement>("#rulesSearch");
  search?.addEventListener("input",()=>{const caret=search.selectionStart;this.search=search.value;context.render();const next=root.querySelector<HTMLInputElement>("#rulesSearch");next?.focus();if(caret!==null)next?.setSelectionRange(caret,caret);});
  const go=(page:number)=>{if(Number.isInteger(page)&&page>=1&&page<=364){this.page=page;context.render();}};
  root.querySelector<HTMLSelectElement>("#rulesChapter")?.addEventListener("change",event=>go(Number((event.target as HTMLSelectElement).value)));
  root.querySelector<HTMLInputElement>("#rulesPage")?.addEventListener("change",event=>go(Number((event.target as HTMLInputElement).value)));
  for(const button of root.querySelectorAll<HTMLButtonElement>("[data-rules-page]"))button.addEventListener("click",()=>{this.search="";go(Number(button.dataset.rulesPage));});
  root.querySelector("#rulesPrevious")?.addEventListener("click",()=>go(this.page-1));
  root.querySelector("#rulesNext")?.addEventListener("click",()=>go(this.page+1));
  root.querySelector("#rulesOriginal")?.addEventListener("click",()=>{this.original=!this.original;context.render();});
  root.querySelector("#rulesLegal")?.addEventListener("click",()=>{this.legal=!this.legal;context.render();});
  root.querySelector("#retryRulesBook")?.addEventListener("click",()=>{void this.loadBook(context);});
  if(!this.book&&!this.loading&&!this.bookError)void this.loadBook(context);
 }
}
