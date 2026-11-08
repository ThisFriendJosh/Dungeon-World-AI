export type ImportedImage = {name:string;dataUrl:string;width:number;height:number};
export type ImportedArtwork = ImportedImage & {thumbnailDataUrl:string;sourceSvg:string|null};
export const IMAGE_ACCEPT=".png,.jpg,.jpeg,.webp,.svg,.gif,.bmp,.avif,image/png,image/jpeg,image/webp,image/svg+xml,image/gif,image/bmp,image/avif";
const NS="http://www.w3.org/2000/svg";
const elements=new Set("svg g defs path rect circle ellipse line polyline polygon text tspan title desc clipPath mask linearGradient radialGradient stop use image".split(" "));
const attributes=new Set("xmlns version width height viewBox preserveAspectRatio id d x y x1 y1 x2 y2 dx dy cx cy r rx ry points transform fill fill-rule fill-opacity stroke stroke-width stroke-opacity stroke-linecap stroke-linejoin stroke-dasharray stroke-dashoffset opacity clip-path clip-rule mask font-family font-size font-weight font-style text-anchor dominant-baseline gradientUnits gradientTransform offset stop-color stop-opacity fx fy fr href color paint-order".split(" "));
export function safeSvg(source:string):string{
  const bad=()=>Error("This SVG contains unsupported active or external content. Use a self-contained SVG with shapes, text, gradients, and local references, or export it as PNG.");
  if(source.length>2_000_000||/<!DOCTYPE|<\?/i.test(source.replace(/^\s*<\?xml[^?]*\?>/,"")))throw bad();
  const document=new DOMParser().parseFromString(source,"image/svg+xml");
  if(document.querySelector("parsererror")||document.documentElement.localName!=="svg")throw Error("The SVG could not be read.");
  let images=0,pixels=0;
  const visit=(node:Element,depth:number)=>{
    if(depth>64||!elements.has(node.localName)||node.namespaceURI!==NS)throw bad();
    for(const attr of Array.from(node.attributes)){
      const key=attr.localName,value=attr.value.toLowerCase();
      if(key==="xlink"&&attr.name.startsWith("xmlns:")){node.removeAttribute(attr.name);continue;}
      if(key==="class"){node.removeAttribute(attr.name);continue;}
      if(key==="style"){
        for(const declaration of attr.value.split(";").filter(s=>s.trim())){const split=declaration.indexOf(":");const name=declaration.slice(0,split).trim();const text=declaration.slice(split+1).trim();if(split<0||!["fill","fill-opacity","fill-rule","stroke","stroke-width","stroke-opacity","stroke-linecap","stroke-linejoin","stroke-dasharray","stroke-dashoffset","opacity","font-family","font-size","font-weight","font-style","text-anchor","dominant-baseline","stop-color","stop-opacity","color"].includes(name)||/[\\@<>]/.test(text)||/url\s*\(/i.test(text))throw bad();node.setAttribute(name,text);}
        node.removeAttribute(attr.name);continue;
      }
      if(key==="href"&&node.localName==="image"&&attr.value.startsWith("data:image/png;base64,")){
        const encoded=attr.value.slice(22);if(!/^[a-z0-9+/]+={0,2}$/i.test(encoded)||++images>16)throw bad();let header:string;try{header=atob(encoded.slice(0,44));}catch{throw bad();}if(!header.startsWith("\x89PNG\r\n\x1a\n")||header.slice(12,16)!=="IHDR")throw bad();const size=(offset:number)=>((header.charCodeAt(offset)*16777216)+(header.charCodeAt(offset+1)<<16)+(header.charCodeAt(offset+2)<<8)+header.charCodeAt(offset+3));const w=size(16),h=size(20);pixels+=w*h;if(!w||!h||w>4096||h>4096||pixels>16777216)throw bad();
      }else if(key==="href"&&!attr.value.startsWith("#"))throw bad();
      if(!attributes.has(key)||key.startsWith("on")||/[\\@]/.test(value)||value.includes("javascript:")||(value.includes("url(")&&!/^url\(#[a-z0-9_.:-]+\)$/i.test(value)))throw bad();
      if(attr.prefix&&attr.prefix!=="xmlns"){if(key!=="href")throw bad();node.removeAttribute(attr.name);node.setAttribute("href",attr.value);}
    }
    for(const child of Array.from(node.children))visit(child,depth+1);
  };visit(document.documentElement,0);
  return new XMLSerializer().serializeToString(document.documentElement);
}
async function prepare(file:File,maxEdge:number):Promise<ImportedArtwork>{
  if(file.size>20*1024*1024)throw Error("Choose an image smaller than 20 MB.");
  let sourceSvg:string|null=null,source:CanvasImageSource,width:number,height:number,release:()=>void;
  if(file.type.includes("svg")||/\.svg$/i.test(file.name)){
    sourceSvg=safeSvg(await file.text());
    const url=URL.createObjectURL(new Blob([sourceSvg],{type:"image/svg+xml"}));const image=new Image();
    try{await new Promise<void>((resolve,reject)=>{image.onload=()=>resolve();image.onerror=()=>reject(Error("This SVG could not be rendered. Export it as PNG and try again."));image.src=url;});}catch(error){URL.revokeObjectURL(url);throw error;}
    source=image;width=image.naturalWidth;height=image.naturalHeight;release=()=>URL.revokeObjectURL(url);
  }else{
    let bitmap:ImageBitmap;try{bitmap=await createImageBitmap(file);}catch{throw Error("This image format could not be read. Use PNG, JPEG, WebP, SVG, GIF, BMP, or AVIF.");}
    source=bitmap;width=bitmap.width;height=bitmap.height;release=()=>bitmap.close();
  }
  try{
    if(!width||!height||width>16384||height>16384)throw Error("Choose an image up to 16384 pixels per edge.");
    let scale=Math.min(1,maxEdge/Math.max(width,height));const canvas=document.createElement("canvas");
    for(let attempt=0;attempt<6;attempt++){
      canvas.width=Math.max(1,Math.round(width*scale));canvas.height=Math.max(1,Math.round(height*scale));const context=canvas.getContext("2d");if(!context)throw Error("Image conversion is unavailable.");
      context.drawImage(source,0,0,canvas.width,canvas.height);const dataUrl=canvas.toDataURL("image/png");
      if(dataUrl.length<=5_590_000){
        const thumb=document.createElement("canvas"),size=Math.min(1,256/Math.max(canvas.width,canvas.height));thumb.width=Math.max(1,Math.round(canvas.width*size));thumb.height=Math.max(1,Math.round(canvas.height*size));thumb.getContext("2d")!.drawImage(canvas,0,0,thumb.width,thumb.height);
        return{name:file.name.slice(0,120)||"Imported image",dataUrl,width:canvas.width,height:canvas.height,thumbnailDataUrl:thumb.toDataURL("image/png"),sourceSvg};
      }scale*=.75;
    }throw Error("The image is still too large after conversion.");
  }finally{release();}
}
export async function prepareImage(file:File):Promise<ImportedImage>{const {name,dataUrl,width,height}=await prepare(file,2048);return{name,dataUrl,width,height};}
export async function prepareArtwork(file:File):Promise<ImportedArtwork>{return prepare(file,4096);}
