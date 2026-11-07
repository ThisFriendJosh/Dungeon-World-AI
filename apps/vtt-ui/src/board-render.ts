import {getArt,artImage} from "./bundled-art";
import {escapeHtml as e} from "./html";
import type {SceneBoard, BoardToken, Drawing} from "./board-model";
const initials=(name:string)=>name.split(/\s+/).filter(Boolean).slice(0,2).map(s=>s[0]).join("").toUpperCase();
const color=(value:string)=>/^#[a-f0-9]{6}$/i.test(value)?value:"#d6b973";
function terrain(board:SceneBoard):string {
  const w=board.columns,h=board.rows;
  if(board.background==="builtin:sewer")return `<rect width="${w}" height="${h}" fill="#57534b"/><path d="M${w*.45},0V${h}H${w*.55}V0Z" fill="#2d5149"/><path d="M1,1H${w*.38}V${h*.42}H1ZM${w*.62},1H${w-1}V${h*.42}H${w*.62}ZM1,${h*.58}H${w*.38}V${h-1}H1ZM${w*.62},${h*.58}H${w-1}V${h-1}H${w*.62}Z" fill="#262b29" stroke="#a59b7e" stroke-width=".2"/>`;
  if(board.background==="builtin:pond")return `<rect width="${w}" height="${h}" fill="#52634a"/><path d="M0,${h*.7}Q${w*.3},${h*.6} ${w*.38},${h*.4}T${w},${h*.3}" fill="none" stroke="#ab9270" stroke-width="2"/><ellipse cx="${w*.67}" cy="${h*.65}" rx="${w*.2}" ry="${h*.26}" fill="#325b63" stroke="#839579" stroke-width=".4"/><path d="M${w*.45},${h*.6}L${w*.84},${h*.55}" stroke="#574333" stroke-width=".9"/>`;
  if(board.background==="builtin:forest"){
    const trees=Array.from({length:28},(_,i)=>{const x=(i*7.71)%w,y=(i*3.17)%h;if(x>w*.25&&x<w*.75&&y>h*.2&&y<h*.8)return "";return `<circle cx="${x}" cy="${y}" r="${.8+(i%3)*.2}" fill="${i%2?"#233c30":"#314c34"}" stroke="#6b7d4f" stroke-width=".08"/>`;}).join("");
    return `<rect width="${w}" height="${h}" fill="#5d6347"/><path d="M0,${h*.76}Q${w*.4},${h*.8} ${w*.5},${h*.5}T${w},${h*.2}" fill="none" stroke="#b29873" stroke-width="1.7"/>${trees}`;
  }
  return `<rect width="${w}" height="${h}" fill="#17382e"/><rect width="${w}" height="${h}" fill="url(#felt-grain)" opacity=".2"/>`;
}
export function renderDrawing(d:Drawing):string {
  const a=d.points[0],b=d.points[d.points.length-1];
  const style=`fill="none" stroke="${color(d.color)}" stroke-width="${d.width}" stroke-linecap="round" stroke-linejoin="round"`;
  if(d.kind==="rectangle")return `<rect x="${Math.min(a.x,b.x)}" y="${Math.min(a.y,b.y)}" width="${Math.abs(b.x-a.x)}" height="${Math.abs(b.y-a.y)}" ${style}/>`;
  if(d.kind==="ellipse")return `<ellipse cx="${(a.x+b.x)/2}" cy="${(a.y+b.y)/2}" rx="${Math.abs(b.x-a.x)/2}" ry="${Math.abs(b.y-a.y)/2}" ${style}/>`;
  return `<polyline points="${d.points.map(p=>p.x+","+p.y).join(" ")}" ${style}/>`;
}
export function renderBoard(board:SceneBoard,selected:string|null,preview:boolean,image:(id:string)=>string|null,live:Drawing|null=null):string {
  const w=board.columns,h=board.rows;
  const tokens=board.tokens.filter(t=>!preview||!t.hidden);
  const lights=tokens.filter(t=>t.dimLight>0);
  const background=board.background?image(board.background):null;
  const backgroundArt=getArt(board.background);
  const token=(t:BoardToken)=>{
    const c=color(t.color),r=t.size/2,cx=t.x+r,cy=t.y+r,src=t.sprite?image(t.sprite):null,spriteArt=getArt(t.sprite),clip="token-"+t.id;
    const hp=t.hp!==null?`<text x="0" y="${r+.32}" text-anchor="middle" font-size=".24" fill="#f1e9cd" paint-order="stroke" stroke="#151713" stroke-width=".09">${t.hp}${t.maxHp!==null?"/"+t.maxHp:""} HP</text>`:"";
    return `<g data-token="${e(t.id)}" transform="translate(${cx} ${cy})" opacity="${t.hidden?.45:1}" class="map-token">
      <title>${e(t.name)}${t.states.length?": "+e(t.states.join(", ")):""}${t.hidden?" (GM only)":""}</title>
      <defs><clipPath id="${clip}"><circle r="${r*.83}"/></clipPath></defs>
      <circle r="${r*.95}" fill="#121714" stroke="${!preview&&selected===t.id?"#fff5cf":c}" stroke-width="${!preview&&selected===t.id?.11:.065}"/>
      ${src?`<g clip-path="url(#${clip})" transform="rotate(${t.rotation})">${spriteArt?artImage(spriteArt,-r*.83,-r*.83,r*1.66,r*1.66):`<image href="${e(src)}" x="${-r*.83}" y="${-r*.83}" width="${r*1.66}" height="${r*1.66}" preserveAspectRatio="xMidYMid slice"/>`}</g>`:`<circle r="${r*.8}" fill="${c}" opacity=".18"/><text y=".12" text-anchor="middle" font-family="Georgia,serif" font-weight="bold" font-size="${Math.min(r*.75,.5)}" fill="${c}">${e(initials(t.name))}</text>`}
      ${t.states.length?`<circle cx="${r*.7}" cy="${-r*.7}" r=".17" fill="#c87b5d" stroke="#f2dbaf" stroke-width=".035"/><text x="${r*.7}" y="${-r*.7+.065}" text-anchor="middle" font-size=".19" fill="#fff">${t.states.length}</text>`:""}
      ${t.locked&&!preview?`<rect x="${-r*.8}" y="${r*.5}" width=".22" height=".2" rx=".03" fill="#ceb578"/>`:""}${hp}
      <text x="0" y="${-r-.18}" text-anchor="middle" font-size=".27" font-family="Segoe UI,sans-serif" fill="#f5edd7" paint-order="stroke" stroke="#151713" stroke-width=".095">${e(t.name.slice(0,35))}</text>
    </g>`;
  };
  return `<svg id="battleSvg" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" role="img" aria-label="${backgroundArt?.role==="scene"?"Illustrated scene":"Battle map, "+w+" by "+h+" squares, each square 5 feet"}" tabindex="0">
    <defs>
      <pattern id="table-grid" width="1" height="1" patternUnits="userSpaceOnUse"><path d="M1 0H0V1" fill="none" stroke="#f6ebcd" stroke-width=".018"/></pattern>
      <pattern id="felt-grain" width=".3" height=".3" patternUnits="userSpaceOnUse"><path d="M0 0L.3 .3M.3 0L0 .3" stroke="#d6c496" stroke-width=".02"/></pattern>
      ${lights.map(t=>`<radialGradient id="light-${t.id}"><stop offset="${Math.min(t.brightLight/t.dimLight,.99)}" stop-color="black"/><stop offset="1" stop-color="white"/></radialGradient>`).join("")}
      <mask id="darkness-mask"><rect width="${w}" height="${h}" fill="white"/>${lights.map(t=>`<circle cx="${t.x+t.size/2}" cy="${t.y+t.size/2}" r="${t.dimLight/5}" fill="url(#light-${t.id})" style="mix-blend-mode:darken"/>`).join("")}</mask>
      <mask id="fog-mask"><rect width="${w}" height="${h}" fill="white"/>${board.fog.map(m=>`<circle cx="${m.x}" cy="${m.y}" r="${m.radius}" fill="${m.reveal?"black":"white"}"/>`).join("")}</mask>
    </defs>
    ${background?`<rect width="${w}" height="${h}" fill="#171b18"/><image href="${e(background)}" x="${board.backgroundX??0}" y="${board.backgroundY??0}" width="${w*(board.backgroundScale??1)}" height="${h*(board.backgroundScale??1)}" preserveAspectRatio="${board.backgroundFit==="stretch"?"none":"xMidYMid meet"}"/>`:terrain(board)}
    ${board.grid?`<rect width="${w}" height="${h}" fill="url(#table-grid)" opacity="${board.gridOpacity}" pointer-events="none"/>`:""}
    <g pointer-events="none">${board.drawings.map(renderDrawing).join("")}${live?renderDrawing(live):""}</g>
    ${tokens.map(token).join("")}
    ${board.ambient<1?`<rect width="${w}" height="${h}" fill="#060b16" opacity="${1-board.ambient}" mask="url(#darkness-mask)" pointer-events="none"/>`:""}
    ${board.fogEnabled?`<rect width="${w}" height="${h}" fill="#090c0b" opacity="${preview?1:.62}" mask="url(#fog-mask)" pointer-events="none"/>`:""}
  </svg>`;
}
