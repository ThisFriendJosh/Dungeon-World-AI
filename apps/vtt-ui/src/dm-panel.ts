import {invoke} from "@tauri-apps/api/core";
import {escapeHtml as e} from "./html";
import {newId} from "./uuid";
import type {CharacterSummary,Saved} from "./types";
import type {DiceRoll} from "./dice-types";
import type {DmHistory,DmRuntimeStatus,DmTurn} from "./dm-types";

type Context={campaignId:string;render():void;onSaved(warning:string|null):Promise<void>};

export class DmPanel {
  private campaignId="";
  private history:DmHistory={turns:[],totalTurns:0,latestEventSeq:0};
  private runtime:DmRuntimeStatus|null=null;
  private actorId="";
  private draft="";
  private requestId:string|null=null;
  private error="";
  private cancelling=false;
  private latestRoll:DiceRoll|null=null;
  pending=false;
  get hasDraft(){return this.draft.trim().length>0;}

  load(campaignId:string,history:DmHistory,runtime:DmRuntimeStatus|null,rolls:DiceRoll[]){
    if(this.campaignId!==campaignId){this.actorId="";this.draft="";this.requestId=null;this.error="";}
    this.campaignId=campaignId;this.history=structuredClone(history);this.runtime=runtime;
    this.latestRoll=rolls[0]??null;
    // A refresh after a lost IPC response can recover an already committed turn.
    if(this.requestId&&history.turns.some(t=>t.requestId===this.requestId)){
      this.draft="";this.requestId=null;this.error="";
    }
  }

  html(characters:CharacterSummary[]):string{
    if(this.actorId&&!characters.some(c=>c.id===this.actorId))this.actorId="";
    const ready=this.runtime?.configured??false;
    const last=this.history.turns[this.history.turns.length-1];
    return `<section class="dm-panel" aria-label="AI Dungeon Master">
      <header class="dm-heading"><div><p class="eyebrow">The next chapter is yours</p><h2>Dungeon Master</h2></div>
        <span class="dm-model-state">${this.pending?"Preparing your reply…":ready?"Local AI · Offline":"Local model unavailable"}</span></header>
      <p class="subtle dm-scope">Describe what you do, ask an NPC a question, or set an opening scene. Accepted turns save automatically. Use Dice for checks and your character sheet for game statistics.</p>
      ${!ready?'<p class="dm-notice">The local model has not been configured for this build. You can read saved conversations and continue using the tabletop.</p>':""}
      <div class="dm-transcript" role="log" aria-label="Saved adventure conversation">
        ${this.history.turns.length?this.history.turns.map(turn=>`<article class="dm-turn">
          <div class="dm-player"><p class="eyebrow">${e(characters.find(c=>c.id===turn.actorId)?.name??"You")}</p><p>${e(turn.playerText)}</p></div>
          <div class="dm-narration"><p class="eyebrow">Dungeon Master</p><p>${e(turn.reply.narration)}</p>
            <time datetime="${e(turn.createdAt)}">Saved ${e(new Date(turn.createdAt).toLocaleString())}</time></div></article>`).join(""):
          '<div class="dm-opening"><span aria-hidden="true">✦</span><h3>A world waiting to unfold</h3><p>Tell the Dungeon Master where your adventure begins, or ask it to introduce an opening scene for your party.</p></div>'}
      </div>
      ${this.history.totalTurns>12?'<p class="subtle">Your earlier turns stay saved. This first version uses recent conversation; include a reminder when returning to older events.</p>':""}
      ${this.error?`<p class="dm-error" role="alert">${e(this.error)}</p>`:""}
      ${this.pending?`<div class="dm-progress" role="status"><span class="dm-spinner" aria-hidden="true"></span><p>${this.cancelling?"Stopping generation…":"The Dungeon Master is thinking. Loading the model for the first turn can take a moment."}</p><button id="dmCancel" type="button" ${this.cancelling?"disabled":""}>Stop</button></div>`:""}
      <form id="dmForm"><fieldset ${this.pending?"disabled":""}>
        <label>Speak or act as<select id="dmActor"><option value="">The player / party</option>${characters.map(c=>`<option value="${e(c.id)}" ${c.id===this.actorId?"selected":""}>${e(c.name)}</option>`).join("")}</select></label>
        <label>Your action<textarea id="dmAction" rows="4" maxlength="4000" required placeholder="I approach the gatekeeper and ask about the missing caravan…">${e(this.draft)}</textarea></label>
        <div class="dm-actions"><button class="primary" type="submit" ${!ready?"disabled":""}>Send to Dungeon Master</button>
          ${this.latestRoll?'<button id="dmUseRoll" type="button">Use latest saved roll</button>':""}<span class="subtle">Saved on this device</span></div>
        ${last?.reply.choices.length?`<div class="dm-choices" aria-label="Possible next actions">${last.reply.choices.map((choice,index)=>`<button type="button" data-dm-choice="${index}">${e(choice)}</button>`).join("")}</div>`:""}
      </fieldset></form>
      ${this.history.totalTurns>50?`<p class="subtle">Showing the latest 50 of ${this.history.totalTurns} saved turns.</p>`:""}
    </section>`;
  }

  bind(root:HTMLElement,context:Context){
    const textarea=root.querySelector<HTMLTextAreaElement>("#dmAction");
    const actor=root.querySelector<HTMLSelectElement>("#dmActor");
    if(!textarea||!actor)return;
    const read=()=>{
      if(this.draft!==textarea.value||this.actorId!==actor.value)this.requestId=null;
      this.draft=textarea.value;this.actorId=actor.value;
    };
    textarea.addEventListener("input",read);actor.addEventListener("change",read);
    root.querySelectorAll<HTMLButtonElement>("[data-dm-choice]").forEach(button=>button.addEventListener("click",()=>{
      if(this.pending)return;
      const turn=this.history.turns[this.history.turns.length-1];
      textarea.value=turn.reply.choices[Number(button.dataset.dmChoice)]??"";read();textarea.focus();
    }));
    root.querySelector("#dmUseRoll")?.addEventListener("click",()=>{
      if(this.pending||!this.latestRoll)return;
      const instruction=`Use the recorded roll with ID ${this.latestRoll.id} to resolve my pending action.`;
      textarea.value=[textarea.value,instruction].filter(Boolean).join("\n").slice(0,4000);read();textarea.focus();
    });
    root.querySelector("#dmCancel")?.addEventListener("click",()=>{
      if(!this.pending||this.cancelling)return;
      this.cancelling=true;context.render();
      void invoke<boolean>("cancel_dm_turn").then(stopped=>{
        if(!stopped&&this.pending){this.error="The reply has not started or has already finished. If it is still preparing, try Stop again.";this.cancelling=false;context.render();}
      }).catch(error=>{this.error=String(error);this.cancelling=false;context.render();});
    });
    root.querySelector("#dmForm")?.addEventListener("submit",event=>{
      event.preventDefault();if(this.pending)return;read();
      if(!this.draft.trim()||!this.runtime?.configured)return;
      this.requestId??=newId();
      const request={requestId:this.requestId,actorId:this.actorId||null,playerText:this.draft.trim(),expectedSeq:this.history.latestEventSeq};
      this.pending=true;this.cancelling=false;this.error="";context.render();
      void (async()=>{
        try{
          const result=await invoke<Saved<DmTurn>>("play_dm_turn",{campaignId:context.campaignId,request});
          // Retain the committed reply even if the subsequent campaign refresh fails.
          if(!this.history.turns.some(t=>t.id===result.value.id)){
            this.history.turns=[...this.history.turns,result.value].slice(-50);this.history.totalTurns++;
          }
          this.history.latestEventSeq=result.value.eventSeq;
          this.draft="";this.requestId=null;this.pending=false;this.cancelling=false;
          await context.onSaved(result.warning);
        }catch(error){this.error=String(error);}
        finally{
          this.pending=false;this.cancelling=false;context.render();
          root.querySelector(".dm-turn:last-child")?.scrollIntoView({block:"nearest"});
        }
      })();
    });
  }
}
