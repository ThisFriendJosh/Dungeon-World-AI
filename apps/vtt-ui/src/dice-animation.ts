import type {DiceRoll} from "./dice-types";
import {add,sub,scale,dot,cross,unit,center,dieMesh,rotate,axisAngle,multiply,faceOrientation,slerp} from "./dice-geometry";
import type {Vec3,Quat,DieMesh} from "./dice-geometry";
export type VisualDie={sides:number;value:number;labels:string[];faceIndex:number;kept:boolean;sign:number;label:string;percentile:boolean};
export function visualDice(roll:DiceRoll):VisualDie[]{
 const result:VisualDie[]=[];
 for(const term of roll.terms){if(term.kind!=="dice")continue;term.rolls.forEach((value,index)=>{const kept=term.kept[index]!==false,prefix=term.sign<0?"−":"";
   if(term.sides===100){const tens=value===100?0:Math.floor(value/10),ones=value%10;result.push({sides:10,value:tens*10,labels:Array.from({length:10},(_,i)=>String(i*10).padStart(2,"0")),faceIndex:tens,kept,sign:term.sign,label:prefix+"d100 · tens",percentile:true},{sides:10,value:ones,labels:Array.from({length:10},(_,i)=>String(i)),faceIndex:ones,kept,sign:term.sign,label:prefix+"d100 · units",percentile:true});}
   else result.push({sides:term.sides,value,labels:Array.from({length:Math.min(term.sides,20)},(_,i)=>String(i+1)),faceIndex:value-1,kept,sign:term.sign,label:prefix+"d"+term.sides,percentile:false});
 });}return result;
}
function hash(value:string):number{let state=2166136261;for(const ch of value){state^=ch.charCodeAt(0);state=Math.imul(state,16777619);}return state>>>0;}
const clamp=(n:number)=>Math.min(1,Math.max(0,n));
export function mountDiceAnimation(canvas:HTMLCanvasElement,roll:DiceRoll,options:{animate:boolean;color:string;onSummary(message:string):void}):()=>void{
 const context=canvas.getContext("2d");if(!context){options.onSummary("Saved results are available in the roll history.");return ()=>{};}
 const all=visualDice(roll),reduced=window.matchMedia("(prefers-reduced-motion: reduce)").matches,animate=options.animate&&!reduced;
 const rgb=/^#[a-f0-9]{6}$/i.test(options.color)?[1,3,5].map(i=>parseInt(options.color.slice(i,i+2),16)):[200,164,97];let width=800,height=300,frame=0,disposed=false,last=animate?0:1;const start=performance.now();
 const project=(p:Vec3,x:number,y:number,size:number)=>{const perspective=4/(4-p[2]);return [x+p[0]*size*perspective,y-p[1]*size*perspective];};
 const spin=(seed:number,t:number):Quat=>multiply(axisAngle([0,0,1],t*(8+seed%4)),multiply(axisAngle([0,1,0],t*(13+(seed>>>5)%7)),axisAngle([1,0,0],t*(9+(seed>>>10)%6))));
 const drawMesh=(mesh:DieMesh,die:VisualDie,q:Quat,x:number,y:number,size:number,settled:boolean)=>{
  const vertices=mesh.vertices.map(p=>rotate(q,p)),faces=mesh.faces.map((face,index)=>({face,index,c:center(face.map(i=>vertices[i])),n:rotate(q,mesh.normals[index])})).filter(f=>dot(f.n,sub([0,0,5],f.c))>0).sort((a,b)=>a.c[2]-b.c[2]);
  for(const {face,index,c,n} of faces){const light=.48+.55*Math.max(0,dot(n,unit([-.4,.7,1])));context.beginPath();face.forEach((v,i)=>{const p=project(vertices[v],x,y,size);if(i)context.lineTo(p[0],p[1]);else context.moveTo(p[0],p[1]);});context.closePath();context.fillStyle=`rgb(${rgb.map(v=>Math.min(255,Math.round(v*light))).join(",")})`;context.fill();context.lineWidth=1.1;context.strokeStyle=die.kept?"#f2dba699":"#c4ccc677";context.stroke();
   const u=unit(sub(vertices[face[0]],c)),v=unit(cross(n,u)),inner=Math.min(...face.map((vertex,i)=>{const edge=sub(vertices[face[(i+1)%face.length]],vertices[vertex]);return Math.hypot(...cross(edge,sub(c,vertices[vertex])))/Math.hypot(...edge);}));
   const font=inner*1.22,pc=project(c,x,y,size),pu=project(add(c,scale(u,font)),x,y,size),pv=project(sub(c,scale(v,font)),x,y,size);
   context.save();context.transform((pu[0]-pc[0])/100,(pu[1]-pc[1])/100,(pv[0]-pc[0])/100,(pv[1]-pc[1])/100,pc[0],pc[1]);context.font="bold 100px Georgia,serif";context.fillStyle=die.kept?"#191b15":"#2a302c";context.textAlign="center";context.textBaseline="middle";context.fillText(die.labels[index]??String(index+1),0,0,150);context.restore();
  }
  context.save();context.font="12px system-ui,sans-serif";context.textAlign="center";context.fillStyle=die.kept?"#efdeb7":"#adb5ad";context.fillText(die.label,x,y+size+21);if(settled&&!die.kept){context.font="11px system-ui,sans-serif";context.fillText("Discarded",x,y+size+36);}context.restore();
 };
 const draw=(progress:number)=>{
  if(disposed)return;last=progress;context.clearRect(0,0,width,height);const gradient=context.createRadialGradient(width*.5,height*.35,5,width*.5,height*.5,Math.max(width,height));gradient.addColorStop(0,"#294133");gradient.addColorStop(1,"#0c1813");context.fillStyle=gradient;context.fillRect(0,0,width,height);
  const limit=width<500?8:24,dice=all.slice(0,limit);options.onSummary((all.length>dice.length?`Showing ${dice.length} of ${all.length} dice. `:"")+"Saved total: "+roll.total+(roll.mode!=="normal"?" · "+roll.mode:"")+". All faces remain in the roll history.");
  const cols=Math.max(1,Math.ceil(Math.sqrt(dice.length*width/height))),rows=Math.max(1,Math.ceil(dice.length/cols)),cellWidth=width/cols,cellHeight=height/rows,size=Math.max(15,Math.min(62,cellWidth*.31,(cellHeight-38)*.36));
  dice.forEach((die,index)=>{const seed=hash(roll.id+":"+index),t=animate?clamp(progress*1.18-index%5*.035):1,x=cellWidth*(index%cols+.5)+Math.sin(t*11+seed%9)*(1-t)*Math.min(35,cellWidth*.16),baseY=cellHeight*(Math.floor(index/cols)+.42),bounce=Math.abs(Math.sin(t*Math.PI*4.5))*(1-t)*Math.min(55,cellHeight*.2),y=baseY-bounce;
    context.save();context.globalAlpha=die.kept?1:.46;context.fillStyle="#050c08aa";context.beginPath();context.ellipse(x,baseY+size*.68,size*.75*(1-bounce/160),size*.16,0,0,Math.PI*2);context.fill();const mesh=dieMesh(die.sides);
    if(mesh&&die.faceIndex>=0&&die.faceIndex<mesh.faces.length){const target=faceOrientation(mesh.normals[die.faceIndex],((seed%17)-8)*.025),begin=spin(seed,.68),q=t<.68?spin(seed,t):slerp(begin,target,1-Math.pow(1-clamp((t-.68)/.32),3));drawMesh(mesh,die,q,x,y,size,t>=.995);}
    else{context.fillStyle="#b5955e";context.beginPath();context.roundRect(x-size,y-size,size*2,size*2,9);context.fill();context.fillStyle="#172118";context.font=`bold ${Math.max(14,size*.7)}px Georgia,serif`;context.textAlign="center";context.textBaseline="middle";context.fillText(String(die.value),x,y);context.fillStyle="#e6d4ad";context.font="12px system-ui";context.fillText(die.label,x,y+size+20);}
    context.restore();
  });
 };
 const resize=()=>{const rect=canvas.getBoundingClientRect(),ratio=Math.min(2,window.devicePixelRatio||1);width=Math.max(200,rect.width);height=width<500?280:320;canvas.style.height=height+"px";canvas.width=Math.round(width*ratio);canvas.height=Math.round(height*ratio);context.setTransform(ratio,0,0,ratio,0,0);draw(last);};
 resize();const observer=new ResizeObserver(resize);observer.observe(canvas);
 const step=(now:number)=>{const progress=clamp((now-start)/1150);draw(progress);if(progress<1&&!disposed)frame=requestAnimationFrame(step);};if(animate)frame=requestAnimationFrame(step);
 return ()=>{disposed=true;cancelAnimationFrame(frame);observer.disconnect();};
}
