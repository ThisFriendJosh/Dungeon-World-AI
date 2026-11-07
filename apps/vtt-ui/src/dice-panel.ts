import { invoke } from "@tauri-apps/api/core";
import type { CharacterSummary, Saved } from "./types";
import type { DiceMode, DiceRoll } from "./dice-types";
import { describeRoll, describeTerm } from "./dice-format";
import { escapeHtml as e } from "./html";

type Context = {
  campaignId: string;
  action(work: () => Promise<void>): Promise<void>;
  onSaved(roll: DiceRoll, warning: string | null): Promise<void>;
  isBusy(): boolean;
};
export class DicePanel {
  private rolls: DiceRoll[] = [];
  private expression = "1d20";
  private mode: DiceMode = "normal";
  private actorId = "";
  private label = "";
  private campaignId = "";
  load(campaignId: string, rolls: DiceRoll[]) {
    if (this.campaignId !== campaignId) { this.actorId = ""; this.label = ""; }
    this.campaignId = campaignId;
    this.rolls = structuredClone(rolls);
  }
  html(characters: CharacterSummary[]): string {
    if (!characters.some(character => character.id === this.actorId)) this.actorId = "";
    return `
      <section class="dice-panel">
        <p class="eyebrow">Open rolls</p><h2>Dice tray</h2>
        <p class="subtle">Roll locally and keep the result in this campaign's history. Each roll shows every die face.</p>
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
        await context.onSaved(result.value, result.warning);
      });
    });
  }
}
