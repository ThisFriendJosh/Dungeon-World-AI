export type SourceDraft={title:string;text:string;sourceRef:string;edition:string;gmOnly:boolean};
export type PreparedImport={source:SourceDraft;method:string;warnings:string[];collection?:{name:string;kind:string;entries:unknown[]}};
const editions=["homebrew","unassigned","5e-2014","5e-2024","AD&D-2e",""];
const datasets=["backgrounds","classes","items","monsters","species","spells"];
export const titleFor=(name:string)=>name.replace(/\.[^.]+$/,"").slice(0,120)||"Imported source";
export function htmlText(text:string):string{
  // Strip markup as text; never create a browsing document or load its resources.
  const entities:Record<string,string>={amp:"&",lt:"<",gt:">",quot:'"',apos:"'",nbsp:" "};
  return text.replace(/<!--[\s\S]*?-->/g,"").replace(/<(script|style|template)\b[^>]*>[\s\S]*?<\/\1\s*>/gi,"")
    .replace(/<\/?(?:p|div|h[1-6]|tr|li|br|section|article)\b[^>]*>/gi,"\n").replace(/<[^>]*>/g,"")
    .replace(/&(#x[0-9a-f]+|#\d+|amp|lt|gt|quot|apos|nbsp);/gi,(original,entity:string)=>{
      if(entity[0]!=="#")return entities[entity.toLowerCase()]??original;
      const number=entity[1].toLowerCase()==="x"?parseInt(entity.slice(2),16):parseInt(entity.slice(1),10);
      return number>0&&number<=0x10ffff&&!(number>=0xd800&&number<=0xdfff)?String.fromCodePoint(number):original;
    }).replace(/\n[ \t]*\n[ \t]*\n/g,"\n\n").trim();
}
export function validateSource(value:unknown):SourceDraft{
  if(!value||typeof value!=="object")throw Error("A source must contain a title and text.");
  const source=value as Record<string,unknown>;
  if(typeof source.title!=="string"||!source.title.trim()||source.title.length>120||typeof source.text!=="string"||!source.text.trim())throw Error("Each source needs a title (up to 120 characters) and readable text.");
  if(new TextEncoder().encode(source.text).length>8_000_000)throw Error("A source exceeds the 8 MB text limit. Split it into smaller documents.");
  const edition=source.edition??"unassigned",sourceRef=source.sourceRef??"",gmOnly=source.gmOnly??true;
  if(typeof edition!=="string"||!editions.includes(edition)||typeof sourceRef!=="string"||sourceRef.length>1000||typeof gmOnly!=="boolean")throw Error("Check the source edition, filename/link, and GM-only setting.");
  return {title:source.title.trim(),text:source.text,sourceRef,edition,gmOnly};
}
export function prepareTextFile(name:string,text:string,edition="unassigned",gmOnly=true):PreparedImport[]{
  const extension=name.split('.').pop()?.toLowerCase();
  const base={title:titleFor(name),text,sourceRef:name,edition,gmOnly};
  if(extension==="html"||extension==="htm")base.text=htmlText(text);
  if(extension==="json"){
    let value:unknown;try{value=JSON.parse(text);}catch{throw Error(name+" contains invalid JSON.");}
    if(value&&typeof value==="object"&&!Array.isArray(value)&&"sources" in value){
      const pack=value as {schemaVersion?:unknown;sources:unknown;notIndexed?:unknown};
      if(pack.schemaVersion!==1||!Array.isArray(pack.sources)||!pack.sources.length||pack.sources.length>200)throw Error("A campaign source pack needs schemaVersion 1 and 1-200 sources.");
      const warnings=Array.isArray(pack.notIndexed)?pack.notIndexed.map(item=>{const entry=item as {source?:unknown;reason?:unknown};return String(entry?.source??"Source")+": "+String(entry?.reason??"Not indexed");}):[];
      return pack.sources.map((source,index)=>({source:validateSource(source),method:"Campaign source pack",warnings:index===0?warnings:[]}));
    }
    const kind=titleFor(name).toLowerCase();
    if(datasets.includes(kind)&&Array.isArray(value)){
      if(!value.length)throw Error(name+" contains an empty reference collection.");
      return [{source:{...base,text:JSON.stringify(value,null,2)},method:"Structured reference collection",warnings:[],collection:{name,kind,entries:value}}];
    }
    base.text=JSON.stringify(value,null,2);
    return [{source:validateSource(base),method:"JSON reference text",warnings:["JSON is indexed as reference text. It does not create characters or change game statistics."]}];
  }
  if(!["txt","md","markdown","csv","html","htm"].includes(extension??""))throw Error("Choose PDF, DOCX, TXT, Markdown, CSV, HTML or JSON. Save older .doc files as .docx first.");
  return [{source:validateSource(base),method:extension==="csv"?"CSV reference text":"Document text",warnings:[]}];
}
