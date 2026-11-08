import {invoke} from "@tauri-apps/api/core";
import {escapeHtml as e} from "./html";
import type {Saved} from "./types";
export type TokenSpec={name:string;label:string;color:string;shape:"circle"|"square"|"hex";symbol:"shield"|"sword"|"skull"|"star"|"initials"};
export type GeneratedAsset={id:string;spec:TokenSpec;createdAt:string};
type StoredAsset={id:string;svg:string;pngDataUrl:string};
type Context={campaignId:string;action(work:()=>Promise<void>):Promise<void>;render():void;onSaved(warning:string|null):Promise<void>;onPlace(id:string,name:string):void;onError(message:string):void};
const uri=(svg:string)=>"data:image/svg+xml;charset=utf-8,"+encodeURIComponent(svg);
const filename=(name:string)=>name.replace(/[^a-z0-9_-]+/gi,"_").slice(0,80)||"token";
export async function pngFromSvg(svg:string):Promise<string>{
  const image=new Image();image.decoding="async";
  const objectUrl=URL.createObjectURL(new Blob([svg],{type:"image/svg+xml"}));
  try{await new Promise<void>((resolve,reject)=>{image.onload=()=>resolve();image.onerror=()=>reject(Error("Could not render the token preview."));image.src=objectUrl;});
    const canvas=document.createElement("canvas");canvas.width=512;canvas.height=512;const context=canvas.getContext("2d");if(!context)throw Error("PNG rendering is unavailable.");context.drawImage(image,0,0,512,512);return canvas.toDataURL("image/png");
  }finally{URL.revokeObjectURL(objectUrl);}
}
export class AssetWorkshop{
  private campaignId="";private assets:GeneratedAsset[]=[];private selected:StoredAsset|null=null;private preview="";private open=false;
  private spec:TokenSpec={name:"New token",label:"NPC",color:"#385547",shape:"circle",symbol:"shield"};
  load(campaignId:string,assets:GeneratedAsset[]){if(campaignId!==this.campaignId){this.selected=null;this.preview="";}this.campaignId=campaignId;this.assets=assets;}
  html():string{return `<section class="asset-workshop"><details id="assetWorkshopDetails" ${this.open?"open":""}><summary>Token workshop · create and save PNG + SVG</summary><p class="subtle">Create a transparent 512×512 token locally. Save its PNG, SVG, and editable recipe with this campaign; export copies or place it on the table.</p>
    <form id="tokenAssetForm"><fieldset><div class="asset-editor"><div><label>Asset name<input id="assetName" required maxlength="120" value="${e(this.spec.name)}"></label><label>Short label<input id="assetLabel" maxlength="18" value="${e(this.spec.label)}"></label>
      <div class="two-columns"><label>Color<input id="assetColor" type="color" value="${e(this.spec.color)}"></label><label>Frame<select id="assetShape">${(["circle","square","hex"] as const).map(value=>`<option ${value===this.spec.shape?"selected":""}>${value}</option>`).join("")}</select></label></div>
      <label>Symbol<select id="assetSymbol">${(["shield","sword","skull","star","initials"] as const).map(value=>`<option ${value===this.spec.symbol?"selected":""}>${value}</option>`).join("")}</select></label><div class="asset-actions"><button type="button" id="assetPreview">Preview token</button><button type="submit" class="primary">Save PNG + SVG</button></div></div>
      <div class="asset-preview">${this.preview?`<img src="${e(uri(this.preview))}" alt="Generated token preview">`:'<p class="subtle">Choose a frame and symbol, then preview your token.</p>'}</div></div></fieldset></form>
    <h3>Generated assets (${this.assets.length})</h3><div class="saved-assets">${this.assets.map(asset=>`<article><strong>${e(asset.spec.name)}</strong><p class="subtle">${e(asset.spec.symbol)} · ${e(asset.spec.shape)}</p><div class="asset-actions"><button type="button" data-open-asset="${e(asset.id)}">Open / export</button><button type="button" data-place-asset="${e(asset.id)}">Place token</button></div></article>`).join("")||'<p class="subtle">Your saved tokens will appear here.</p>'}</div>
    ${this.selected?`<div class="asset-export"><img src="${e(this.selected.pngDataUrl)}" alt="Saved token"><div class="asset-actions"><a href="${e(this.selected.pngDataUrl)}" download="${e(filename(this.assets.find(a=>a.id===this.selected?.id)?.spec.name??"token"))}.png">Download PNG</a><a href="${e(uri(this.selected.svg))}" download="${e(filename(this.assets.find(a=>a.id===this.selected?.id)?.spec.name??"token"))}.svg">Download SVG</a></div></div>`:""}</details></section>`;}
  bind(root:HTMLElement,context:Context){
    const details=root.querySelector<HTMLDetailsElement>("#assetWorkshopDetails");details?.addEventListener("toggle",()=>{this.open=details.open;});
    const read=()=>{this.spec={name:root.querySelector<HTMLInputElement>("#assetName")!.value,label:root.querySelector<HTMLInputElement>("#assetLabel")!.value,color:root.querySelector<HTMLInputElement>("#assetColor")!.value,shape:root.querySelector<HTMLSelectElement>("#assetShape")!.value as TokenSpec["shape"],symbol:root.querySelector<HTMLSelectElement>("#assetSymbol")!.value as TokenSpec["symbol"]};};
    root.querySelector("#tokenAssetForm")?.addEventListener("input",read);
    root.querySelector("#assetPreview")?.addEventListener("click",()=>{read();void context.action(async()=>{this.preview=await invoke<string>("preview_token_asset",{spec:this.spec});context.render();});});
    root.querySelector("#tokenAssetForm")?.addEventListener("submit",event=>{event.preventDefault();read();const spec=structuredClone(this.spec);void context.action(async()=>{const svg=await invoke<string>("preview_token_asset",{spec});const pngDataUrl=await pngFromSvg(svg);const result=await invoke<Saved<GeneratedAsset>>("save_token_asset",{campaignId:context.campaignId,request:{spec,pngDataUrl}});this.preview=svg;this.selected={id:result.value.id,svg,pngDataUrl};await context.onSaved(result.warning);});});
    root.querySelectorAll<HTMLButtonElement>("[data-open-asset]").forEach(button=>button.addEventListener("click",()=>{void context.action(async()=>{this.selected=await invoke<StoredAsset>("load_generated_asset",{campaignId:context.campaignId,id:button.dataset.openAsset});const spec=this.assets.find(a=>a.id===this.selected?.id)?.spec;if(spec)this.spec=structuredClone(spec);this.preview=this.selected.svg;context.render();});}));
    root.querySelectorAll<HTMLButtonElement>("[data-place-asset]").forEach(button=>button.addEventListener("click",()=>{const asset=this.assets.find(a=>a.id===button.dataset.placeAsset);if(asset)context.onPlace(asset.id,asset.spec.name);}));
  }
}
