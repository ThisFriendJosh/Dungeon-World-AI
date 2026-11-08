import {escapeHtml as e} from "./html";

export type CalculationRecord={id:string;kind:"arithmetic"|"rules"|"damage";characterId:string|null;inputs:Record<string,unknown>;result:unknown;previewOnly:true;eventSeq:number;createdAt:string};

export function calculationHistory(records:CalculationRecord[],more:boolean):string{
 return `<section class="tools-card calculation-history" aria-label="Saved calculations"><h3>Saved calculations</h3><p class="hint">These results are available to the Dungeon Master. Checks and damage are previews; apply changes to the character sheet separately.</p>${records.length?`<ol>${records.map(record=>{
  const result=record.result as Record<string,unknown>;
  const summary=record.kind==="arithmetic"?`${String(record.inputs.expression)} = ${record.result}`:record.kind==="damage"?`HP ${result.hpAfter} · temporary HP ${result.tempHpAfter}`:`Ability modifier ${result.abilityModifier} · spell save DC ${result.spellSaveDc}`;
  return `<li><details><summary><strong>${e(record.kind==="damage"?"Damage / healing preview":record.kind==="rules"?"Rules preview":"Arithmetic")}</strong><span>${e(summary)}</span><time datetime="${e(record.createdAt)}">${e(new Date(record.createdAt).toLocaleString())}</time></summary><pre>${e(JSON.stringify({characterId:record.characterId,inputs:record.inputs,result:record.result,previewOnly:record.previewOnly},null,2))}</pre></details></li>`;
 }).join("")}</ol>`:'<p class="subtle">Calculate a result to save it with this campaign.</p>'}${more?'<button type="button" id="olderCalculations">Show older calculations</button>':""}</section>`;
}
