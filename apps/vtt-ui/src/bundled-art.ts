import {bundledArt} from "./bundled-art-data";
import {escapeHtml as e} from "./html";
export type BundledArt = {
 id:string;title:string;category:string;role:"battlemap"|"scene"|"atlas"|"portrait"|"reference"|"ui";
 path:string;width:number;height:number;sourceSha256:string;sourceIndex:number;note:string;gridPrinted:boolean;
 kindHint?:"player"|"npc"|"monster"|"object"|"treasure";mechanics:string;crop?:readonly number[];suggestedColumns?:number;suggestedRows?:number;
};
export const ART:readonly BundledArt[]=bundledArt;
const byId=new Map(ART.map(a=>[a.id,a]));
export const getArt=(id:string|null|undefined):BundledArt|undefined=>id?byId.get(id):undefined;
export const MAP_ART=ART.filter(a=>a.role==="battlemap");
export const SCENE_ART=ART.filter(a=>a.role==="scene");
export const PORTRAIT_ART=ART.filter(a=>a.role==="portrait");
export const TABLE_ART=ART.find(a=>a.role==="ui");
export function artImage(asset:BundledArt,x:number,y:number,width:number,height:number):string {
 if(asset.crop){return `<svg x="${x}" y="${y}" width="${width}" height="${height}" viewBox="${asset.crop.join(" ")}" preserveAspectRatio="xMidYMid slice" overflow="hidden"><image href="${e(asset.path)}" width="${asset.width}" height="${asset.height}"/></svg>`;}
 return `<image href="${e(asset.path)}" x="${x}" y="${y}" width="${width}" height="${height}" preserveAspectRatio="xMidYMid slice"/>`;
}
export function artThumbnail(asset:BundledArt):string {
 if(asset.crop)return `<svg viewBox="${asset.crop.join(" ")}" role="img" aria-label="${e(asset.title)}" preserveAspectRatio="xMidYMid slice"><image href="${e(asset.path)}" width="${asset.width}" height="${asset.height}"/></svg>`;
 return `<img src="${e(asset.path)}" alt="${e(asset.title)}" loading="lazy" decoding="async">`;
}
