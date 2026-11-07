import {ART,artThumbnail,getArt} from "./bundled-art";
import {escapeHtml as e} from "./html";
type Context={onBackground(id:string):void;onToken(id:string):void;render():void};
export class ArtLibrary {
 private category="All";private query="";private selected:string|null=null;
 html():string {
  const categories=["All",...new Set(ART.map(a=>a.category))];
  const filtered=ART.filter(a=>(this.category==="All"||a.category===this.category)&&[a.title,a.category,a.note].join(" ").toLowerCase().includes(this.query.toLowerCase()));
  const selected=getArt(this.selected);
  return `<section class="art-library"><header class="art-library-heading"><div><p class="eyebrow">Aerathis · illustrated collection</p><h2>Art & codex</h2><p class="subtle">Maps, portraits, scenes, and campaign folios from your supplied artwork.</p></div><label>Find artwork<input id="artSearch" type="search" value="${e(this.query)}" placeholder="Orrek, pond, manor…"></label></header>
   <div class="art-filters" aria-label="Artwork categories">${categories.map(c=>`<button type="button" data-art-category="${e(c)}" aria-pressed="${c===this.category}">${e(c)} <small>${c==="All"?ART.length:ART.filter(a=>a.category===c).length}</small></button>`).join("")}</div>
   <p class="art-reference-note">Illustrated statistics and homebrew effects are reference material. Use the campaign's rules profile and recorded character values for play.</p>
   <div class="art-grid">${filtered.map(asset=>`<article class="art-card"><button type="button" class="art-cover" data-view-art="${e(asset.id)}" aria-label="View ${e(asset.title)}">${artThumbnail(asset)}</button><div class="art-card-content"><p class="eyebrow">${e(asset.category)}</p><h3>${e(asset.title)}</h3><p class="subtle">${e(asset.note)}</p><div class="art-card-actions"><button type="button" data-view-art="${e(asset.id)}">View source</button>${asset.role==="battlemap"||asset.role==="scene"?`<button type="button" data-use-background="${e(asset.id)}">Use background</button>`:""}${asset.role==="portrait"?`<button type="button" data-place-art="${e(asset.id)}">Place token</button>`:""}</div></div></article>`).join("")}</div>
   ${!filtered.length?'<p class="empty-note">No artwork matches this search.</p>':""}
   ${selected?`<dialog id="artViewer" class="art-viewer"><header><div><p class="eyebrow">${e(selected.category)} · original source view</p><h2>${e(selected.title)}</h2></div><button type="button" id="closeArtViewer" aria-label="Close artwork">Close</button></header><p class="subtle">${e(selected.note)}</p><div class="art-viewer-toolbar"><button type="button" id="toggleArtZoom">View at full size</button><span>${selected.width} × ${selected.height}</span></div><div class="art-viewer-image"><img src="${e(selected.path)}" alt="${e(selected.title)}"></div></dialog>`:""}
  </section>`;
 }
 bind(root:HTMLElement,context:Context){
  for(const button of root.querySelectorAll<HTMLButtonElement>("[data-art-category]"))button.addEventListener("click",()=>{this.category=button.dataset.artCategory!;context.render();});
  const search=root.querySelector<HTMLInputElement>("#artSearch");
  search?.addEventListener("input",()=>{const caret=search.selectionStart;this.query=search.value;context.render();const next=root.querySelector<HTMLInputElement>("#artSearch");next?.focus();if(caret!==null)next?.setSelectionRange(caret,caret);});
  for(const button of root.querySelectorAll<HTMLButtonElement>("[data-view-art]"))button.addEventListener("click",()=>{this.selected=button.dataset.viewArt!;context.render();});
  for(const button of root.querySelectorAll<HTMLButtonElement>("[data-use-background]"))button.addEventListener("click",()=>context.onBackground(button.dataset.useBackground!));
  for(const button of root.querySelectorAll<HTMLButtonElement>("[data-place-art]"))button.addEventListener("click",()=>context.onToken(button.dataset.placeArt!));
  const dialog=root.querySelector<HTMLDialogElement>("#artViewer");
  if(dialog){dialog.showModal();root.querySelector("#closeArtViewer")?.addEventListener("click",()=>dialog.close());dialog.addEventListener("close",()=>{this.selected=null;context.render();});
   root.querySelector("#toggleArtZoom")?.addEventListener("click",()=>{const viewer=root.querySelector(".art-viewer-image")!;const full=viewer.classList.toggle("full-size");root.querySelector("#toggleArtZoom")!.textContent=full?"Fit artwork":"View at full size";});
  }
 }
}
