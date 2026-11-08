export type CompendiumSection="documents"|"entries";
export function compendiumWorkspace(section:CompendiumSection,documents:string,entries:string):string{
  return `<section aria-label="Compendium"><div class="compendium-sections" aria-label="Compendium sections"><button type="button" data-compendium-section="documents" aria-pressed="${section==="documents"}">Campaign documents & DM library</button><button type="button" data-compendium-section="entries" aria-pressed="${section==="entries"}">Reference entries</button></div>${section==="documents"?documents:entries}</section>`;
}
