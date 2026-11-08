import { invoke } from "@tauri-apps/api/core";
import type { CharacterSummary, Saved } from "./types";
import type { DiceMode, DiceRoll } from "./dice-types";
import { describeRoll, describeTerm } from "./dice-format";
import { escapeHtml as e } from "./html";
import {mountDiceAnimation} from "./dice-animation";

type Context = {
  campaignId: string;
  action(work: () => Promise<void>): Promise<void>;
  onSaved(roll: DiceRoll, warning: string | null): Promise<void>;
  isBusy(): boolean;
  render(): void;
};
export class DicePanel {
  private rolls: DiceRoll[] = [];
  private expression = "1d20";
  private mode: DiceMode = "normal";
  private actorId = "";
  private label = "";
  private campaignId = "";
  private visualRoll:DiceRoll|null=null;private animateNext=false;private enabled=true;private color="#d0ad65";private cleanup:(()=>void)|null=null;
  constructor(){try{if(typeof localStorage!=="undefined"){const value=JSON.parse(localStorage.getItem("dwa:dice-visual:v1")??"null");if(value){this.enabled=value.enabled!==false;if(/^#[a-f0-9]{6}$/i.test(value.color))this.color=value.color;}}}catch{}}
  detach(){this.cleanup?.();this.cleanup=null;}
  private preferences(){try{localStorage.setItem("dwa:dice-visual:v1",JSON.stringify({enabled:this.enabled,color:this.color}));}catch{}}
  load(campaignId: string, rolls: DiceRoll[]) {
    if (this.campaignId !== campaignId) { this.actorId = ""; this.label = "";this.visualRoll=null;this.animateNext=false;this.detach(); }
    this.campaignId = campaignId;
    this.rolls = structuredClone(rolls);
    if(this.visualRoll?.id!==this.rolls[0]?.id){this.visualRoll=this.rolls[0]??null;this.animateNext=false;}
  }
  html(characters: CharacterSummary[]): string {
    if (!characters.some(character => character.id === this.actorId)) this.actorId = "";
    return `
      <section class="dice-panel">
        <p class="eyebrow">Open rolls</p><h2>3D Dice tray</h2>
        <p class="subtle">Roll locally and keep the result in this campaign's history. Each roll shows every die face.</p>
        <div class="dice-visual">${this.visualRoll?`<canvas id="diceAnimation" role="img" aria-label="${e(describeRoll(this.visualRoll))} · saved dice result"></canvas>`:'<div class="dice-visual-empty"><strong>Your dice are ready</strong><p>Choose a die or enter a formula to roll.</p></div>'}<div class="dice-visual-toolbar"><label><input id="diceAnimate" type="checkbox" ${this.enabled?"checked":""}>Animate rolls</label><label>Dice color<input id="diceColor" type="color" value="${e(this.color)}"></label><button type="button" id="diceReplay" ${this.visualRoll?"":"disabled"}>Replay visual</button></div><p id="diceVisualSummary" class="dice-visual-caption">${this.visualRoll?e(describeRoll(this.visualRoll)):"d4 · d6 · d8 · d10 · d12 · d20 · percentile dice"}</p></div>
        <form id="diceForm"><fieldset><legend class="sr-only">Dice roll</legend>
          <div class="dice-presets" aria-label="Dice presets">${[4,6,8,10,12,20,100].map(sides => `<button type="button" data-die-preset="${sides}" aria-label="Set formula to 1d${sides}">d${sides}</button>`).join("")}</div>
          <label>Formula<input id="diceExpression" required maxlength="120" value="${e(this.expression)}" placeholder="1d20+5 or 2d6+1d4-2" autocomplete="off" spellcheck="false"></label>
          <div class="two-columns">
            <label>Roll mode<select id="diceMode">
              <option value="normal" ${this.mode === "normal" ? "selected" : ""}>Normal</option>
              <option value="advantage" ${this.mode === "advantage" ? "selected" : ""}>Advantage</option>
              <option value="disadvantage" ${this.mode === "disadvantage" ? "selected" : ""}>Disadvantage</option>
            </select></label>
            <label>Roll for<select id="diceActor"><option value="">Campaign roll</option>${characters.map(character => `<option value="${e(character.id)}" ${this.actorId === character.id ? "selected" : ""}>${e(character.name)}</option>`).join("")}</select></label>
          </div>
          <label>Label <span class="hint">Optional, such as Stealth or Greataxe damage</span><input id="diceLabel" maxlength="120" value="${e(this.label)}"></label>
          <p class="subtle dice-help">Enter bonuses in the formula; the character picker labels the result. Advantage/disadvantage applies to one positive 1d20 term; other dice, such as Bless, roll once. Choose Normal when advantage and disadvantage cancel.</p>
          <button class="primary" type="submit">Roll and save</button>
        </fieldset></form>
        <div class="section-heading dice-history-heading"><h2>Roll history</h2><span class="subtle">Latest 50 rolls</span></div>
        <ol class="dice-history">${this.rolls.length ? this.rolls.map(roll => `
          <li class="dice-result">
            <div class="dice-result-heading"><div><strong>${e(describeRoll(roll))}</strong>
              <p class="subtle">${e([roll.actorName, roll.label].filter(Boolean).join(" · ") || "Campaign roll")}</p></div>
              <output aria-label="Roll total">${e(roll.total)}</output></div>
            <ul class="dice-breakdown">${roll.terms.map(term => `<li>${e(describeTerm(term))}</li>`).join("")}</ul>
            <time datetime="${e(roll.createdAt)}">${e(new Date(roll.createdAt).toLocaleString())}</time>
          </li>`).join("") : '<li class="empty-note">Choose a die or enter a formula, then roll. Saved results appear here.</li>'}</ol>
      </section>`;
  }
  bind(root: HTMLElement, context: Context) {
    this.detach();
    const canvas=root.querySelector<HTMLCanvasElement>("#diceAnimation");if(canvas&&this.visualRoll){this.cleanup=mountDiceAnimation(canvas,this.visualRoll,{animate:this.enabled&&this.animateNext,color:this.color,onSummary:message=>{const summary=root.querySelector("#diceVisualSummary");if(summary&&summary.textContent!==message)summary.textContent=message;}});this.animateNext=false;}
    root.querySelector<HTMLInputElement>("#diceAnimate")?.addEventListener("change",event=>{this.enabled=(event.target as HTMLInputElement).checked;this.preferences();context.render();});
    root.querySelector<HTMLInputElement>("#diceColor")?.addEventListener("change",event=>{this.color=(event.target as HTMLInputElement).value;this.preferences();context.render();});
    root.querySelector("#diceReplay")?.addEventListener("click",()=>{if(!this.visualRoll||context.isBusy())return;this.animateNext=true;context.render();});
    const form = root.querySelector<HTMLFormElement>("#diceForm");
    if (!form) return;
    const read = () => {
      this.expression = root.querySelector<HTMLInputElement>("#diceExpression")!.value;
      this.mode = root.querySelector<HTMLSelectElement>("#diceMode")!.value as DiceMode;
      this.actorId = root.querySelector<HTMLSelectElement>("#diceActor")!.value;
      this.label = root.querySelector<HTMLInputElement>("#diceLabel")!.value;
    };
    form.addEventListener("input", read);
    form.addEventListener("change", read);
    root.querySelectorAll<HTMLButtonElement>("[data-die-preset]").forEach(button => button.addEventListener("click", () => {
      if (context.isBusy()) return;
      read();
      this.expression = "1d" + button.dataset.diePreset!;
      const input = root.querySelector<HTMLInputElement>("#diceExpression")!;
      input.value = this.expression; input.focus();
    }));
    form.addEventListener("submit", event => {
      event.preventDefault(); read();
      const request = { expression: this.expression, mode: this.mode, characterId: this.actorId || null, label: this.label };
      void context.action(async () => {
        const result = await invoke<Saved<DiceRoll>>("roll_dice", { campaignId: context.campaignId, request });
        this.rolls = [result.value, ...this.rolls].slice(0,50);
        this.visualRoll=structuredClone(result.value);this.animateNext=true;
        await context.onSaved(result.value, result.warning);
      });
    });
  }
}
