import {invoke} from "@tauri-apps/api/core";
import type {CampaignEvent,Saved} from "./types";
import {escapeHtml as e} from "./html";
type Entry=CampaignEvent&{removed:boolean;canRemove:boolean};
type Context={campaignId:string;expectedSeq:number;render():void;isBusy():boolean;action(work:()=>Promise<void>):Promise<void>;onSaved(warning:string|null,message:string):Promise<void>;onError(message:string):void};
export class HistoryPanel{
  private campaignId="";private editing=false;private entries:Entry[]=[];private showRemoved=false;private selected:Entry|null=null;
  load(campaignId:string){if(campaignId!==this.campaignId){this.editing=false;this.entries=[];this.showRemoved=false;this.selected=null;}this.campaignId=campaignId;}
  html(events:CampaignEvent[],label:(event:CampaignEvent)=>string):string{
    const entries=this.editing?this.entries.filter(event=>this.showRemoved||!event.removed):events.map(event=>({...event,removed:false,canRemove:false}));
    return `<section class="history"><div class="section-heading"><div><h2>Campaign activity</h2><p class="subtle">${this.editing?"Edit one entry at a time. Current sheets and scenes are preserved.":"Latest 50 changes"}</p></div><button type="button" id="historyEdit" aria-pressed="${this.editing}">${this.editing?"Done editing":"Edit history"}</button></div>
      ${this.editing?`<label class="history-removed-toggle"><input id="historyShowRemoved" type="checkbox" ${this.showRemoved?"checked":""}> Show removed entries and restore options</label>`:""}
      <ol>${entries.map(event=>`<li class="${event.removed?"history-removed":""}"><span class="event-marker" aria-hidden="true"></span><div><strong>${e(label(event))}</strong><time datetime="${e(event.createdAt)}">${e(new Date(event.createdAt).toLocaleString())}${event.removed?" · Removed":""}</time></div>
        ${this.editing&&event.canRemove?`<button type="button" class="history-remove" data-history-seq="${event.seq}" aria-label="${event.removed?"Restore":"Remove"} ${e(label(event))}">${event.removed?"Restore":"×"}</button>`:""}</li>`).join("")}</ol>
      ${this.selected?`<dialog id="historyConfirm" class="history-confirm"><h2>${this.selected.removed?"Restore":"Remove"} this entry?</h2><p><strong>${e(label(this.selected))}</strong></p><p class="subtle">${e(new Date(this.selected.createdAt).toLocaleString())}</p><p>${this.selected.removed?"The entry will return to the timeline and become available to the Dungeon Master again.":"This removes the entry from the visible timeline and DM memory. Current character and scene values stay as they are. Later replies can still refer to it; those can be removed separately. You can restore this entry in Edit history."}</p><div><button type="button" id="historyKeep">Cancel</button><button type="button" class="primary" id="historyConfirmChange">${this.selected.removed?"Restore entry":"Remove entry"}</button></div></dialog>`:""}
    </section>`;
  }
  bind(root:HTMLElement,context:Context){
    root.querySelector("#historyEdit")?.addEventListener("click",()=>{
      if(context.isBusy())return;
      if(this.editing){this.editing=false;this.selected=null;context.render();return;}
      void context.action(async()=>{this.entries=await invoke<Entry[]>("load_editable_history",{campaignId:context.campaignId});this.editing=true;context.render();});
    });
    root.querySelector<HTMLInputElement>("#historyShowRemoved")?.addEventListener("change",event=>{this.showRemoved=(event.target as HTMLInputElement).checked;context.render();});
    root.querySelectorAll<HTMLButtonElement>("[data-history-seq]").forEach(button=>button.addEventListener("click",()=>{if(context.isBusy())return;this.selected=this.entries.find(e=>e.seq===Number(button.dataset.historySeq))??null;context.render();}));
    const dialog=root.querySelector<HTMLDialogElement>("#historyConfirm");
    if(dialog){dialog.showModal();const cancel=()=>{this.selected=null;context.render();};root.querySelector("#historyKeep")?.addEventListener("click",cancel);dialog.addEventListener("cancel",event=>{event.preventDefault();cancel();});
      root.querySelector("#historyConfirmChange")?.addEventListener("click",()=>{
        if(!this.selected||context.isBusy())return;const entry=this.selected;
        void context.action(async()=>{const result=await invoke<Saved<unknown>>("edit_history_entry",{campaignId:context.campaignId,request:{eventSeq:entry.seq,expectedSeq:context.expectedSeq,removed:!entry.removed,confirmed:true}});
          this.selected=null;this.entries=await invoke<Entry[]>("load_editable_history",{campaignId:context.campaignId});await context.onSaved(result.warning,entry.removed?"History entry restored.":"History entry removed; current game state preserved.");});
      });
    }
  }
}
