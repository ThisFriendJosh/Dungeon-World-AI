export type ImportedImage = {name:string;dataUrl:string;width:number;height:number};
export async function prepareImage(file:File):Promise<ImportedImage> {
  if(file.size>20*1024*1024)throw new Error("Choose an image smaller than 20 MB.");
  if(file.type.includes("svg")||/\.svgz?$/i.test(file.name))throw new Error("Use PNG, JPEG, WebP, GIF, BMP, or AVIF for map and token artwork.");
  const bitmap=await createImageBitmap(file);
  try {
    if(bitmap.width>16384||bitmap.height>16384)throw new Error("Choose an image up to 16384 pixels per edge.");
    let scale=Math.min(1,2048/Math.max(bitmap.width,bitmap.height));
    const canvas=document.createElement("canvas");
    for(let attempt=0;attempt<5;attempt++){
      canvas.width=Math.max(1,Math.round(bitmap.width*scale));canvas.height=Math.max(1,Math.round(bitmap.height*scale));
      const context=canvas.getContext("2d");if(!context)throw new Error("Image conversion is unavailable.");
      context.drawImage(bitmap,0,0,canvas.width,canvas.height);
      const dataUrl=canvas.toDataURL("image/png");
      if(dataUrl.length<=5_590_000)return {name:file.name.slice(0,120)||"Imported image",dataUrl,width:canvas.width,height:canvas.height};
      scale*=.75;
    }
    throw new Error("The image is still too large after conversion.");
  }finally{bitmap.close();}
}
