import {invoke} from "@tauri-apps/api/core";
import {copyScene,makeScene,makeSceneCard} from "./fari/scene";
import type {FariScene,TabletopState} from "./fari/scene";
import type {CharacterSummary,Saved} from "./types";
import {escapeHtml as e} from "./html";
import {makeBoard,makeToken,positionToken,resizeBoard,movementFeet,clamp,SKETCHES} from "./board-model";
import type {SceneBoard,BoardToken,TokenKind,Point,Drawing,SceneSummary,SceneImage} from "./board-model";
import {getArt,MAP_ART,SCENE_ART,PORTRAIT_ART,artThumbnail} from "./bundled-art";
import {renderBoard} from "./board-render";
import {prepareImage} from "./image-import";
import {newId} from "./uuid";

type Tool="select"|"pan"|"pen"|"rectangle"|"ellipse"|"reveal"|"conceal";
type Context={
 campaignId:string; characters:CharacterSummary[];
 action(work:()=>Promise<void>):Promise<void>; onSaved(warning:string|null,message?:string):Promise<void>;
 onDirty():void; onError(message:string):void; render():void; isBusy():boolean;
};
const kinds:TokenKind[]=["player","npc","monster","object","treasure"];
const tools:Tool[]=["select","pan","pen","rectangle","ellipse","reveal","conceal"];
const labels:Record<Tool,string>={select:"Select / move",pan:"Pan",pen:"Draw",rectangle:"Rectangle",ellipse:"Ellipse",reveal:"Reveal fog",conceal:"Conceal fog"};
const checked=(v:boolean)=>v?"checked":"";
const option=(value:string,text:string,current:string|null)=>`<option value="${e(value)}" ${value===current?"selected":""}>${e(text)}</option>`;
export class SceneEditor {
 private saved:TabletopState={scene:null,revision:0,updatedAt:null};
 private draft:FariScene=makeScene();
 private scenes:SceneSummary[]=[];private images:SceneImage[]=[];
 private sources=new Map<string,string>();private loading=new Set<string>();private failed=new Set<string>();private campaignId="";
 private audience:"public"|"private"="public";
 private selectedId:string|null=null;private tool:Tool="select";private drawColor="#ead5a0";private drawWidth=.09;private brush=2;
 private preview=false;private zoom=.65;private fitPending=true;private scroll={x:0,y:0};private diceOpen=false;private cardsOpen=false;
 private undoStack:FariScene[]=[];private redoStack:FariScene[]=[];private lastMove="";
 focused=false;
 dirty=false;
 private get board():SceneBoard {return this.draft.board??(this.draft.board=makeBoard());}
 private get selected():BoardToken|undefined{return this.board.tokens.find(t=>t.id===this.selectedId);}
 load(value:TabletopState,scenes:SceneSummary[]=[],images:SceneImage[]=[],preserveDraft=false){
   this.scenes=structuredClone(scenes);this.images=structuredClone(images);
   if(preserveDraft&&this.dirty)return;
   const changed=this.saved.scene?.id!==value.scene?.id;
   this.saved=structuredClone(value);this.draft=value.scene?copyScene(value.scene):makeScene();
   this.draft.board??=makeBoard();this.dirty=false;this.undoStack=[];this.redoStack=[];
   if(!this.board.tokens.some(t=>t.id===this.selectedId))this.selectedId=null;
   if(changed){this.fitPending=true;this.lastMove="";this.scroll={x:0,y:0};}
 }
 discard(){this.load(this.saved,this.scenes,this.images);}
 private checkpoint(){this.undoStack.push(copyScene(this.draft));if(this.undoStack.length>30)this.undoStack.shift();this.redoStack=[];}
 private changed(context:Context){this.dirty=true;context.onDirty();}
 private source(id:string):string|null{return getArt(id)?.path??this.sources.get(id)??null;}
 useBundledBackground(id:string){const asset=getArt(id);if(!asset||!["battlemap","scene"].includes(asset.role))return false;this.checkpoint();this.board.background=id;this.board.backgroundFit="contain";this.board.backgroundScale=1;this.board.backgroundX=0;this.board.backgroundY=0;this.board.grid=asset.role==="battlemap"&&!asset.gridPrinted;if(!this.board.tokens.length&&!this.board.drawings.length&&!this.board.fog.length){resizeBoard(this.board,asset.suggestedColumns??30,asset.suggestedRows??Math.round(30*asset.height/asset.width));this.fitPending=true;}this.lastMove="";this.preview=false;this.dirty=true;return true;}
 addBundledToken(id:string){const asset=getArt(id);if(!asset||asset.role!=="portrait"||this.board.tokens.length>=100)return false;this.checkpoint();const token=makeToken(asset.kindHint??"npc",this.board.tokens.length+1);token.name=asset.title.slice(0,120);token.sprite=id;token.x=Math.min(2+this.board.tokens.length%6,this.board.columns-1);token.y=Math.min(2+Math.floor(this.board.tokens.length/6),this.board.rows-1);this.board.tokens.push(token);this.selectedId=token.id;this.preview=false;this.dirty=true;return true;}
 addSavedToken(id:string,name:string){if(!this.images.some(image=>image.id===id)||this.board.tokens.length>=100)return false;this.checkpoint();const token=makeToken("npc",this.board.tokens.length+1);token.name=name.slice(0,120);token.sprite=id;this.board.tokens.push(token);this.selectedId=token.id;this.preview=false;this.dirty=true;return true;}
 html(characters:CharacterSummary[]=[],diceHtml=""):string{
   const b=this.board,t=this.selected;
   const selectedArt=getArt(b.background);
   const imageOptions=(current:string|null)=>option("","No image",current??"")+'<optgroup label="Preloaded portraits">'+PORTRAIT_ART.map(a=>option(a.id,a.title,current)).join("")+'</optgroup><optgroup label="Imported images">'+this.images.map(a=>option(a.id,a.name,current)).join("")+"</optgroup>";
   const cards=this.draft.indexCards[this.audience].slice().sort((a,b)=>Number(b.pinned)-Number(a.pinned));
   return `
    <section class="oak-table">
      <form id="sceneForm">
        <div class="table-title"><div><p class="eyebrow">Your world, at the table</p><h2>${e(this.draft.name)}</h2><p class="subtle">${selectedArt?.role==="scene"?"Illustrated scene · positions are markers":b.columns*5+" × "+b.rows*5+" ft · each square 5 × 5 ft"}${this.dirty?" · Unsaved changes":""}</p></div><button class="primary" type="submit">Save scene</button></div>
        <fieldset><legend class="sr-only">Scene and battle map</legend>
          <div class="scene-library"><label>Saved scenes<select id="savedScene" data-ui><option value="">Choose a saved scene</option>${this.scenes.map(s=>option(s.id,s.name,this.draft.id)).join("")}</select></label><button type="button" id="newBoardScene">New scene</button><label class="scene-name-field">Scene name<input id="sceneName" maxlength="120" required value="${e(this.draft.name)}"></label></div>
          <div class="table-layout">
            <div class="table-map-column">
              <div class="map-tools" role="group" aria-label="Map tools">${tools.map(tool=>`<button type="button" data-tool="${tool}" aria-pressed="${this.tool===tool}" ${this.preview?"disabled":""}>${labels[tool]}</button>`).join("")}</div>
              <div class="map-options"><label class="inline-label">Ink<input id="drawColor" data-ui type="color" value="${this.drawColor}"></label><label class="inline-label">Brush<select id="brushSize" data-ui>${[1,2,4,8].map(n=>option(String(n),n*5+" ft radius",String(this.brush))).join("")}</select></label><button type="button" id="undoBoard" ${this.undoStack.length&&!this.preview?"":"disabled"}>Undo</button><button type="button" id="redoBoard" ${this.redoStack.length&&!this.preview?"":"disabled"}>Redo</button><span class="map-options-spacer"></span><button type="button" id="zoomOut" aria-label="Zoom out">−</button><span class="zoom-label">${Math.round(this.zoom*100)}%</span><button type="button" id="zoomIn" aria-label="Zoom in">+</button><button type="button" id="fitMap">Fit map</button><button type="button" id="focusTable">${this.focused?"Exit focus":"Focus table"}</button><button type="button" id="previewMap" aria-pressed="${this.preview}">${this.preview?"Return to GM":"Player preview"}</button></div>
              <div class="oak-map-frame"><div id="mapViewport" class="map-viewport" data-tool="${this.tool}" aria-label="Scrollable battle map"><div id="battleSurface">${renderBoard(b,this.selectedId,this.preview,id=>this.source(id))}</div></div></div>
              <div class="table-map-footer"><span>${this.preview?"Player preview · hidden tokens omitted":this.tool==="select"?"Drag a token, or select it and use the arrow keys":this.tool==="pan"?"Drag the map to pan":labels[this.tool]+" · drag on the map"}</span><span id="moveDistance">${e(this.lastMove||(selectedArt?.role==="scene"?"Scene markers":"5 ft per square"))}</span></div>
              <div class="map-settings"><label class="inline-label"><input id="boardGrid" type="checkbox" ${checked(b.grid)}>Grid</label><label class="inline-label"><input id="boardSnap" type="checkbox" ${checked(b.snap)}>Snap</label><label>Columns<input id="boardColumns" type="number" min="8" max="100" required value="${b.columns}"></label><label>Rows<input id="boardRows" type="number" min="8" max="100" required value="${b.rows}"></label><label>Grid opacity<input id="gridOpacity" type="range" min="0" max=".8" step=".02" value="${b.gridOpacity}"></label></div>
              <details class="oak-drawer"><summary>Scene notes</summary><textarea id="sceneNotes" rows="3" maxlength="10000" aria-label="Scene notes">${e(this.draft.notes)}</textarea></details>
              <details class="oak-drawer" id="cardsDrawer" ${this.cardsOpen?"open":""}><summary>Index cards & GM notes · ${this.draft.indexCards.public.length+this.draft.indexCards.private.length}</summary>
                <div class="scene-toolbar"><div class="scene-groups" aria-label="Card groups"><button type="button" data-scene-group="public" aria-pressed="${this.audience==="public"}">Table cards (${this.draft.indexCards.public.length})</button><button type="button" data-scene-group="private" aria-pressed="${this.audience==="private"}">GM notes (${this.draft.indexCards.private.length})</button></div><button type="button" id="addSceneCard">Add card</button></div>
                <div class="scene-cards">${cards.map(card=>`<article class="scene-card" data-card-id="${e(card.id)}"><div class="card-heading"><label>Card type<input data-card-field="titleLabel" maxlength="40" required value="${e(card.titleLabel)}"></label><label>Color<input data-card-field="color" type="color" value="${e(card.color)}"></label></div><label>Title<input data-card-field="title" required maxlength="120" value="${e(card.title)}"></label><label>Notes<textarea data-card-field="notes" maxlength="10000" rows="4">${e(card.blocks[0].value)}</textarea></label><div class="card-options"><label><input data-card-field="pinned" type="checkbox" ${checked(card.pinned)}>Pin first</label><label><input data-card-field="playedDuringTurn" type="checkbox" ${checked(card.playedDuringTurn)}>Acted this turn</label><button type="button" data-remove-card="${e(card.id)}">Remove</button></div></article>`).join("")||'<p class="empty-note">Keep scene details and discoveries on cards.</p>'}</div>
              </details>
            </div>
            <aside class="table-inspector">
              <section class="oak-well"><h3>Scene background</h3><p class="subtle">Choose a supplied map or import your artwork.</p><div class="background-presets supplied-presets">${[...MAP_ART.filter(a=>a.title.startsWith("Forest ")).slice(0,2),...MAP_ART.filter(a=>!a.title.startsWith("Forest ")).slice(0,4)].map(p=>`<button type="button" data-bundled-background="${e(p.id)}" class="terrain-thumb supplied-thumb" aria-pressed="${b.background===p.id}" title="${e(p.title)}">${artThumbnail(p)}<span>${e(p.title)}</span></button>`).join("")}</div><div class="background-presets">${SKETCHES.map(p=>`<button type="button" data-background="${p.id}" class="terrain-thumb terrain-${p.id.split(":")[1]}" aria-pressed="${b.background===p.id}" title="${p.description}"><span>${p.name}</span></button>`).join("")}</div><label>Background<select id="boardBackground">${option("","Green felt",b.background??"")}${SKETCHES.some(p=>p.id===b.background)?option(b.background!,SKETCHES.find(p=>p.id===b.background)?.name??"",b.background):""}<optgroup label="Supplied battle maps">${MAP_ART.map(a=>option(a.id,a.title,b.background)).join("")}</optgroup><optgroup label="Illustrated scenes">${SCENE_ART.map(a=>option(a.id,a.title,b.background)).join("")}</optgroup><optgroup label="Imported images">${this.images.map(a=>option(a.id,a.name,b.background)).join("")}</optgroup></select></label><label class="file-button">Import background<input id="backgroundFile" data-ui type="file" accept="image/*"></label><p class="hint">Images stay on this device. Set rows and columns to align an existing map grid.</p><details class="background-alignment"><summary>Align background</summary><label>Image fit<select id="backgroundFit">${option("contain","Fit entire image",b.backgroundFit??"contain")}${option("stretch","Stretch to board",b.backgroundFit??"contain")}</select></label><label>Scale (%)<input id="backgroundScale" type="number" min="25" max="400" value="${Math.round((b.backgroundScale??1)*100)}"></label><div class="inspector-pair"><label>Offset X (cells)<input id="backgroundX" type="number" min="-100" max="100" step=".1" value="${b.backgroundX??0}"></label><label>Offset Y (cells)<input id="backgroundY" type="number" min="-100" max="100" step=".1" value="${b.backgroundY??0}"></label></div></details></section>
              <section class="oak-well"><h3>Pieces on the table <span>${b.tokens.length}/100</span></h3><div class="token-add-row">${kinds.map(kind=>`<button type="button" data-add-token="${kind}">${kind==="npc"?"NPC":kind[0].toUpperCase()+kind.slice(1)}</button>`).join("")}</div>${characters.length?`<label>Place a character<select id="placeCharacter" data-ui><option value="">Choose a character</option>${characters.map(c=>option(c.id,c.name,null)).join("")}</select></label>`:""}<div class="token-roster">${b.tokens.map(token=>`<button type="button" data-select-token="${e(token.id)}" aria-pressed="${token.id===this.selectedId}"><span style="color:${e(token.color)}">●</span> ${e(token.name)}<small>${token.kind} · ${token.x+1}, ${token.y+1}${token.hidden?" · GM":""}</small></button>`).join("")||'<p class="empty-note">Add a player, creature, object, or treasure.</p>'}</div></section>
              ${t?`<section class="oak-well token-inspector"><h3>Selected piece</h3><label>Name<input data-token-field="name" required maxlength="120" value="${e(t.name)}"></label><div class="inspector-pair"><label>Type<select data-token-field="kind">${kinds.map(k=>option(k,k==="npc"?"NPC":k,t.kind)).join("")}</select></label><label>Color<input data-token-field="color" type="color" value="${e(t.color)}"></label></div>
              <div class="inspector-pair"><label>Column<input ${t.locked?"disabled":""} data-token-field="x" type="number" min="1" max="${b.columns-t.size+1}" step="${b.snap?1:.2}" value="${t.x+1}"></label><label>Row<input ${t.locked?"disabled":""} data-token-field="y" type="number" min="1" max="${b.rows-t.size+1}" step="${b.snap?1:.2}" value="${t.y+1}"></label></div>
              <label>Footprint<select data-token-field="size">${[1,2,3,4,6,8].map(n=>option(String(n),n*5+" × "+n*5+" ft",String(t.size))).join("")}</select></label>
              <div class="inspector-pair"><label>HP<input data-token-field="hp" type="number" min="0" max="9999" value="${t.hp??""}"></label><label>Maximum HP<input data-token-field="maxHp" type="number" min="1" max="9999" value="${t.maxHp??""}"></label></div><p class="hint">Token HP is tracked here independently of the character sheet.</p>
              <label>Sprite image<select data-token-field="sprite">${imageOptions(t.sprite)}</select></label><label class="file-button">Import sprite<input id="spriteFile" data-ui type="file" accept="image/*"></label><label>Rotation<input data-token-field="rotation" type="range" min="0" max="360" step="15" value="${t.rotation}"></label>
              <label>States <span class="hint">One per line, such as Prone, Hidden, Locked, or Claimed</span><textarea data-token-field="states" rows="2" maxlength="1296">${e(t.states.join("\n"))}</textarea></label><label>Notes<textarea data-token-field="notes" rows="4" maxlength="10000">${e(t.notes)}</textarea></label>
              <div class="inspector-pair"><label>Bright light (ft)<input data-token-field="brightLight" type="number" min="0" max="300" step="5" value="${t.brightLight}"></label><label>Dim edge (ft)<input data-token-field="dimLight" type="number" min="0" max="300" step="5" value="${t.dimLight}"></label></div><p class="hint">Dim edge is the total radius from the source.</p>
              <label class="inline-label"><input data-token-field="hidden" type="checkbox" ${checked(t.hidden)}>GM only</label><label class="inline-label"><input data-token-field="locked" type="checkbox" ${checked(t.locked)}>Lock position</label>
              <div class="token-add-row"><button type="button" id="duplicateToken">Duplicate</button><button type="button" id="removeToken">Remove piece</button></div></section>`:""}
              <section class="oak-well"><h3>Fog & lighting</h3><label class="inline-label"><input id="fogEnabled" type="checkbox" ${checked(b.fogEnabled)}>Fog of war</label><div class="token-add-row"><button type="button" id="coverMap">Cover all</button><button type="button" id="revealMap">Reveal all</button></div><label>Ambient light<select id="ambientLight">${option("1","Daylight",String(b.ambient))}${option("0.45","Dim light",String(b.ambient))}${option("0","Darkness",String(b.ambient))}</select></label><p class="hint">Use Reveal / Conceal to paint fog. Pieces with light radii illuminate the map. Player preview shows full fog and omits GM-only pieces. Light passes through walls.</p></section>
            </aside>
          </div>
        </fieldset>
        <p class="subtle scene-save-note">${this.saved.updatedAt?"Last saved "+e(new Date(this.saved.updatedAt).toLocaleString()):"New scene · Save scene to keep this location."}</p>
      </form>
      <details class="oak-dice-tray" id="tableDice" ${this.diceOpen?"open":""}><summary><span>Dice tray</span><small>Open rolls · saved with this campaign</small></summary>${diceHtml}</details>
    </section>`;
 }
 bind(root:HTMLElement,context:Context){
   const form=root.querySelector<HTMLFormElement>("#sceneForm");if(!form)return;
   if(this.campaignId!==context.campaignId){this.campaignId=context.campaignId;this.sources.clear();this.loading.clear();this.failed.clear();}
   const viewport=root.querySelector<HTMLElement>("#mapViewport")!;
   viewport.scrollLeft=this.scroll.x;viewport.scrollTop=this.scroll.y;
   const input=(id:string)=>root.querySelector<HTMLInputElement>("#"+id)!;
   const mark=()=>this.changed(context);
   const read=()=>{
     this.draft.name=input("sceneName").value;this.draft.notes=root.querySelector<HTMLTextAreaElement>("#sceneNotes")!.value;
     for(const card of this.draft.indexCards[this.audience]){
       const article=root.querySelector<HTMLElement>('[data-card-id="'+card.id+'"]');if(!article)continue;
       const field=(key:string)=>article.querySelector<HTMLInputElement>('[data-card-field="'+key+'"]')!;
       card.title=field("title").value;card.titleLabel=field("titleLabel").value;card.color=field("color").value;
       card.pinned=field("pinned").checked;card.playedDuringTurn=field("playedDuringTurn").checked;
       card.blocks[0].value=article.querySelector<HTMLTextAreaElement>('[data-card-field="notes"]')!.value;
     }
     const token=this.selected;if(token){
       for(const field of root.querySelectorAll<HTMLInputElement|HTMLSelectElement|HTMLTextAreaElement>("[data-token-field]")){
         const key=field.dataset.tokenField!;const value=field.value;
         if(key==="name"||key==="notes"||key==="color")token[key]=value;
         else if(key==="kind")token.kind=value as TokenKind;
         else if(key==="sprite")token.sprite=value||null;
         else if(key==="states")token.states=value.split("\n").map(s=>s.trim()).filter(Boolean);
         else if(key==="hp"||key==="maxHp")token[key]=value===""?null:Number(value);
         else if(key==="hidden"||key==="locked")token[key]=(field as HTMLInputElement).checked;
         else if(["x","y","size","rotation","brightLight","dimLight"].includes(key)&&value!==""&&Number.isFinite(Number(value))){
           const n=Number(value);if(key==="x"||key==="y")token[key]=n-1;
           else if(key==="size"||key==="rotation"||key==="brightLight"||key==="dimLight")token[key]=n;
         }
       }
       Object.assign(token,positionToken(this.board,token,token.x,token.y));
     }
   };
   const paint=(live:Drawing|null=null)=>{
     const surface=root.querySelector<HTMLElement>("#battleSurface");if(!surface)return;
     surface.innerHTML=renderBoard(this.board,this.selectedId,this.preview,id=>this.source(id),live);
     const svg=root.querySelector<SVGSVGElement>("#battleSvg")!;svg.style.width=this.board.columns*40*this.zoom+"px";svg.style.height=this.board.rows*40*this.zoom+"px";
   };
   const rerender=()=>{this.scroll={x:viewport.scrollLeft,y:viewport.scrollTop};context.render();};
   const mutate=(work:()=>void)=>{if(context.isBusy())return;read();this.checkpoint();work();mark();rerender();};
   const click=(selector:string,handler:()=>void)=>root.querySelector(selector)?.addEventListener("click",handler);
   form.addEventListener("focusin",event=>{const el=event.target as HTMLElement;if(el.matches('input:not([data-ui]),textarea,select:not([data-ui])'))this.checkpoint();});
   form.addEventListener("input",event=>{if(context.isBusy())return;const target=event.target as HTMLInputElement;if(target.hasAttribute("data-ui"))return;read();mark();if(target.dataset.tokenField||target.id==="gridOpacity")paint();});
   form.addEventListener("submit",event=>{event.preventDefault();read();const scene=copyScene(this.draft);scene.name=scene.name.trim();scene.lastUpdated=Math.floor(Date.now()/1000);void context.action(async()=>{
     const result=await invoke<Saved<TabletopState>>("save_tabletop",{campaignId:context.campaignId,scene,expectedRevision:this.saved.revision});
     this.load(result.value,this.scenes,this.images);await context.onSaved(result.warning);
   });});
   for(const button of root.querySelectorAll<HTMLButtonElement>("[data-tool]"))if(button.tagName==="BUTTON")button.addEventListener("click",()=>{if(context.isBusy())return;read();this.tool=button.dataset.tool as Tool;rerender();});
   for(const button of root.querySelectorAll<HTMLButtonElement>("[data-background]"))button.addEventListener("click",()=>mutate(()=>{this.board.background=button.dataset.background!;}));
   for(const button of root.querySelectorAll<HTMLButtonElement>("[data-bundled-background]"))button.addEventListener("click",()=>{if(context.isBusy())return;read();this.useBundledBackground(button.dataset.bundledBackground!);mark();rerender();});
   root.querySelector("#boardBackground")?.addEventListener("change",()=>{const id=input("boardBackground").value;if(getArt(id)){read();this.useBundledBackground(id);mark();rerender();}else mutate(()=>{this.board.background=id||null;});});
   for(const id of ["backgroundFit","backgroundScale","backgroundX","backgroundY"])root.querySelector("#"+id)?.addEventListener("change",()=>mutate(()=>{this.board.backgroundFit=input("backgroundFit").value as "contain"|"stretch";this.board.backgroundScale=clamp(Number(input("backgroundScale").value)/100,.25,4);this.board.backgroundX=clamp(Number(input("backgroundX").value),-100,100);this.board.backgroundY=clamp(Number(input("backgroundY").value),-100,100);}));
   for(const key of ["Grid","Snap"])input("board"+key).addEventListener("change",()=>mutate(()=>{if(key==="Grid")this.board.grid=input("boardGrid").checked;else this.board.snap=input("boardSnap").checked;}));
   input("gridOpacity").addEventListener("input",()=>{this.board.gridOpacity=Number(input("gridOpacity").value);paint();});
   for(const id of ["boardColumns","boardRows"])input(id).addEventListener("change",()=>mutate(()=>{resizeBoard(this.board,Number(input("boardColumns").value)||30,Number(input("boardRows").value)||20);this.fitPending=true;}));
   input("drawColor").addEventListener("input",()=>{this.drawColor=input("drawColor").value;});
   root.querySelector("#brushSize")?.addEventListener("change",()=>{this.brush=Number(input("brushSize").value);});
   input("fogEnabled").addEventListener("change",()=>mutate(()=>{this.board.fogEnabled=input("fogEnabled").checked;}));
   click("#coverMap",()=>mutate(()=>{this.board.fogEnabled=true;this.board.fog=[];}));
   click("#revealMap",()=>mutate(()=>{this.board.fogEnabled=false;}));
   root.querySelector("#ambientLight")?.addEventListener("change",()=>mutate(()=>{this.board.ambient=Number(input("ambientLight").value);}));
   const zoom=(value:number)=>{this.zoom=clamp(value,.15,3);paint();const label=root.querySelector(".zoom-label");if(label)label.textContent=Math.round(this.zoom*100)+"%";};
   click("#zoomIn",()=>zoom(this.zoom*1.25));click("#zoomOut",()=>zoom(this.zoom/1.25));
   const fit=()=>{zoom(Math.min((viewport.clientWidth-8)/(this.board.columns*40),(viewport.clientHeight-8)/(this.board.rows*40)));viewport.scrollTo(0,0);this.scroll={x:0,y:0};};
   click("#fitMap",fit);click("#previewMap",()=>{if(context.isBusy())return;read();this.preview=!this.preview;rerender();});
   const undo=(redo=false)=>{if(context.isBusy()||this.preview)return;read();const from=redo?this.redoStack:this.undoStack,to=redo?this.undoStack:this.redoStack;const scene=from.pop();if(!scene)return;to.push(copyScene(this.draft));this.draft=scene;if(!this.selected)this.selectedId=null;mark();rerender();};
   click("#undoBoard",()=>undo());click("#redoBoard",()=>undo(true));
   click("#newBoardScene",()=>{if(context.isBusy())return;if(this.dirty&&!window.confirm("Discard unsaved changes and start a new scene?"))return;this.draft=makeScene();this.draft.name="New scene";this.selectedId=null;this.undoStack=[];this.redoStack=[];this.fitPending=true;mark();rerender();});
   root.querySelector("#savedScene")?.addEventListener("change",()=>{const sceneId=input("savedScene").value;if(!sceneId||sceneId===this.draft.id)return;
     if(this.dirty&&!window.confirm("Discard unsaved changes and open this saved scene?")){input("savedScene").value=this.saved.scene?.id??"";return;}
     void context.action(async()=>{const result=await invoke<Saved<TabletopState>>("select_tabletop_scene",{campaignId:context.campaignId,sceneId,expectedRevision:this.saved.revision});this.load(result.value,this.scenes,this.images);await context.onSaved(result.warning,"Scene selected.");});
   });
   const add=(kind:TokenKind,character?:CharacterSummary)=>mutate(()=>{
     if(this.board.tokens.length>=100){context.onError("A scene supports up to 100 pieces.");return;}
     const token=makeToken(kind,this.board.tokens.length+1);token.x=Math.min(2+this.board.tokens.length%6,this.board.columns-1);token.y=Math.min(2+Math.floor(this.board.tokens.length/6),this.board.rows-1);
     if(character){token.name=character.name;token.characterId=character.id;try{const stats=JSON.parse(character.statsJson);token.hp=typeof stats.hp==="number"?stats.hp:null;token.maxHp=typeof stats.maxHp==="number"?stats.maxHp:null;}catch{/* A manually filled token remains usable. */}}
     this.board.tokens.push(token);this.selectedId=token.id;this.tool="select";
   });
   for(const button of root.querySelectorAll<HTMLButtonElement>("[data-add-token]"))button.addEventListener("click",()=>add(button.dataset.addToken as TokenKind));
   root.querySelector("#placeCharacter")?.addEventListener("change",()=>{const character=context.characters.find(c=>c.id===input("placeCharacter").value);if(character)add("player",character);});
   for(const button of root.querySelectorAll<HTMLButtonElement>("[data-select-token]"))button.addEventListener("click",()=>{if(context.isBusy())return;read();this.selectedId=button.dataset.selectToken!;this.tool="select";rerender();});
   click("#removeToken",()=>mutate(()=>{this.board.tokens=this.board.tokens.filter(t=>t.id!==this.selectedId);this.selectedId=null;}));
   click("#duplicateToken",()=>mutate(()=>{const original=this.selected;if(!original||this.board.tokens.length>=100)return;const copy=structuredClone(original);copy.id=newId();copy.name=(copy.name+" copy").slice(0,120);Object.assign(copy,positionToken(this.board,copy,copy.x+1,copy.y+1));this.board.tokens.push(copy);this.selectedId=copy.id;}));
   for(const field of root.querySelectorAll<HTMLInputElement>("[data-token-field]"))field.addEventListener("change",()=>{read();if(field.dataset.tokenField==="size"||field.dataset.tokenField==="locked"){rerender();return;}paint();void this.loadImages(context,paint);});
   click("#focusTable",()=>{this.focused=!this.focused;this.fitPending=true;rerender();});
   const importImage=(selector:string,sprite:boolean)=>root.querySelector<HTMLInputElement>(selector)?.addEventListener("change",event=>{
     const file=(event.target as HTMLInputElement).files?.[0];if(!file||context.isBusy())return;read();
     void context.action(async()=>{const request=await prepareImage(file);const result=await invoke<Saved<SceneImage>>("import_scene_image",{campaignId:context.campaignId,request});
       this.images.unshift(result.value);this.sources.set(result.value.id,request.dataUrl);this.checkpoint();if(sprite&&this.selected)this.selected.sprite=result.value.id;else {this.board.background=result.value.id;this.board.backgroundFit="contain";this.board.backgroundScale=1;this.board.backgroundX=0;this.board.backgroundY=0;}mark();if(result.warning)context.onError(result.warning);rerender();
     });
   });
   importImage("#backgroundFile",false);importImage("#spriteFile",true);
   for(const button of root.querySelectorAll<HTMLButtonElement>("[data-scene-group]"))button.addEventListener("click",()=>{if(context.isBusy())return;read();this.audience=button.dataset.sceneGroup as "public"|"private";rerender();});
   for(const field of root.querySelectorAll("[data-card-field=\"pinned\"]"))field.addEventListener("change",()=>{read();rerender();});
   click("#addSceneCard",()=>mutate(()=>{if(this.draft.indexCards.public.length+this.draft.indexCards.private.length>=50){context.onError("A scene supports up to 50 cards.");return;}const card=makeSceneCard();card.title="New note";this.draft.indexCards[this.audience].push(card);this.cardsOpen=true;}));
   for(const button of root.querySelectorAll<HTMLButtonElement>("[data-remove-card]"))button.addEventListener("click",()=>mutate(()=>{this.draft.indexCards[this.audience]=this.draft.indexCards[this.audience].filter(c=>c.id!==button.dataset.removeCard);}));
   root.querySelector<HTMLDetailsElement>("#cardsDrawer")?.addEventListener("toggle",event=>{this.cardsOpen=(event.target as HTMLDetailsElement).open;});
   root.querySelector<HTMLDetailsElement>("#tableDice")?.addEventListener("toggle",event=>{this.diceOpen=(event.target as HTMLDetailsElement).open;});
   viewport.addEventListener("scroll",()=>{this.scroll={x:viewport.scrollLeft,y:viewport.scrollTop};});
   type Gesture={start:Point;tokenId?:string;origin?:Point;drawing?:Drawing;pan?:{x:number;y:number;left:number;top:number};lastFog?:Point};
   let gesture:Gesture|null=null;
   const point=(event:PointerEvent):Point=>{const svg=root.querySelector<SVGSVGElement>("#battleSvg")!;const matrix=svg.getScreenCTM();if(!matrix)return{x:0,y:0};const p=svg.createSVGPoint();p.x=event.clientX;p.y=event.clientY;const world=p.matrixTransform(matrix.inverse());return{x:clamp(world.x,0,this.board.columns),y:clamp(world.y,0,this.board.rows)};};
   const fog=(p:Point)=>{if(this.board.fog.length>=500){context.onError("Fog brush limit reached. Undo a stroke or use Cover all to reset the mask.");return;}
     this.board.fogEnabled=true;this.board.fog.push({id:newId(),...p,radius:this.brush,reveal:this.tool==="reveal"});mark();
   };
   viewport.addEventListener("pointerdown",event=>{
     if(event.button!==0||context.isBusy())return;
     if(this.preview&&this.tool!=="pan"){gesture={start:point(event),pan:{x:event.clientX,y:event.clientY,left:viewport.scrollLeft,top:viewport.scrollTop}};viewport.setPointerCapture(event.pointerId);return;}
     read();const p=point(event);const tokenId=(event.target as Element).closest("[data-token]")?.getAttribute("data-token");
     if(this.tool==="pan"){gesture={start:p,pan:{x:event.clientX,y:event.clientY,left:viewport.scrollLeft,top:viewport.scrollTop}};}
     else if(this.tool==="select"){
       this.selectedId=tokenId??null;const token=this.selected;gesture={start:p};
       if(token&&!token.locked){this.checkpoint();gesture.tokenId=token.id;gesture.origin={x:token.x,y:token.y};}paint();
     }else{this.checkpoint();gesture={start:p};
       if(this.tool==="reveal"||this.tool==="conceal"){fog(p);gesture.lastFog=p;paint();}
       else if(this.board.drawings.length>=200){context.onError("A scene supports up to 200 drawings.");gesture=null;return;}
   else gesture.drawing={id:newId(),kind:this.tool as Drawing["kind"],points:[p,p],color:this.drawColor,width:this.drawWidth};
     }
     viewport.setPointerCapture(event.pointerId);event.preventDefault();
   });
   viewport.addEventListener("pointermove",event=>{
     if(!gesture)return;const p=point(event);
     if(gesture.pan){viewport.scrollLeft=gesture.pan.left-(event.clientX-gesture.pan.x);viewport.scrollTop=gesture.pan.top-(event.clientY-gesture.pan.y);return;}
     if(gesture.tokenId&&gesture.origin){const token=this.board.tokens.find(t=>t.id===gesture!.tokenId);if(token){Object.assign(token,positionToken(this.board,token,gesture.origin.x+p.x-gesture.start.x,gesture.origin.y+p.y-gesture.start.y));this.lastMove=getArt(this.board.background)?.role==="scene"?"Marker repositioned":"Moved "+Number(movementFeet(gesture.origin,token).toFixed(1))+" ft";const label=root.querySelector("#moveDistance");if(label)label.textContent=this.lastMove;mark();paint();}}
     else if(gesture.drawing){const d=gesture.drawing;if(d.kind==="pen"){const last=d.points[d.points.length-1];if(d.points.length<2000&&Math.hypot(p.x-last.x,p.y-last.y)>.03)d.points.push(p);}else d.points[1]=p;paint(d);}
     else if(gesture.lastFog&&Math.hypot(p.x-gesture.lastFog.x,p.y-gesture.lastFog.y)>this.brush*.35){fog(p);gesture.lastFog=p;paint();}
   });
   const finish=(event:PointerEvent)=>{if(!gesture)return;const current=gesture;gesture=null;if(viewport.hasPointerCapture(event.pointerId))viewport.releasePointerCapture(event.pointerId);
     if(current.drawing){this.board.drawings.push(current.drawing);mark();}
     if(!current.pan){rerender();root.querySelector<SVGSVGElement>("#battleSvg")?.focus({preventScroll:true});}
   };
   viewport.addEventListener("pointerup",finish);viewport.addEventListener("pointercancel",finish);
   viewport.addEventListener("keydown",event=>{
     if(context.isBusy()||this.preview)return;
     if((event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==="z"){event.preventDefault();undo(event.shiftKey);return;}
     const d:Record<string,Point>={ArrowLeft:{x:-1,y:0},ArrowRight:{x:1,y:0},ArrowUp:{x:0,y:-1},ArrowDown:{x:0,y:1}};
     const token=this.selected;if(!token||token.locked||!d[event.key])return;event.preventDefault();this.checkpoint();const from={x:token.x,y:token.y};const delta=d[event.key],step=this.board.snap?1:.2;
     Object.assign(token,positionToken(this.board,token,token.x+delta.x*step,token.y+delta.y*step));this.lastMove=getArt(this.board.background)?.role==="scene"?"Marker repositioned":"Moved "+Number(movementFeet(from,token).toFixed(1))+" ft";mark();rerender();root.querySelector<SVGSVGElement>("#battleSvg")?.focus({preventScroll:true});
   });
   if(this.fitPending){this.fitPending=false;fit();}else paint();
   void this.loadImages(context,()=>{if(!root.querySelector("#sceneForm"))return;read();paint();});
 }
 private async loadImages(context:Context,paint:()=>void){
   const campaignId=context.campaignId;
   const ids=new Set([this.board.background,...this.board.tokens.map(t=>t.sprite)].filter((v):v is string=>!!v&&!v.startsWith("builtin:")));
   for(const id of ids){if(this.sources.has(id)||this.loading.has(id)||this.failed.has(id))continue;this.loading.add(id);
     try{const url=await invoke<string>("load_scene_image",{campaignId,imageId:id});if(this.campaignId!==campaignId)continue;if(!url.startsWith("data:image/png;base64,"))throw new Error("Unsupported stored image.");this.sources.set(id,url);paint();}
     catch(error){if(this.campaignId===campaignId){this.failed.add(id);context.onError("Image could not be loaded: "+String(error));}}
     finally{this.loading.delete(id);}
   }
 }
}
