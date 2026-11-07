export type Point = { x: number; y: number };
export type TokenKind = "player" | "npc" | "monster" | "object" | "treasure";
export type BoardToken = Point & {
  id: string; name: string; kind: TokenKind; size: number; color: string;
  sprite: string | null; characterId: string | null; notes: string; states: string[];
  hp: number | null; maxHp: number | null; hidden: boolean; locked: boolean;
  rotation: number; brightLight: number; dimLight: number;
};
export type Drawing = { id: string; kind: "pen" | "rectangle" | "ellipse"; points: Point[]; color: string; width: number };
export type FogMark = Point & { id: string; radius: number; reveal: boolean };
export type SceneBoard = {
  columns: number; rows: number; feetPerCell: 5; grid: boolean; snap: boolean; gridOpacity: number;
  background: string | null; backgroundFit: "contain"|"stretch"; backgroundScale:number; backgroundX:number; backgroundY:number; tokens: BoardToken[]; drawings: Drawing[];
  fogEnabled: boolean; fog: FogMark[]; ambient: number;
};
export type SceneSummary = { id: string; name: string; background: string | null; updatedAt: string };
export type SceneImage = { id: string; name: string; width: number; height: number };
export const makeBoard = (): SceneBoard => ({
  columns: 30, rows: 20, feetPerCell: 5, grid: true, snap: true, gridOpacity: .22,
  background: null, backgroundFit:"contain",backgroundScale:1,backgroundX:0,backgroundY:0, tokens: [], drawings: [], fogEnabled: false, fog: [], ambient: 1
});
const colors: Record<TokenKind,string> = {player:"#d6b973",npc:"#60c4b2",monster:"#df796b",object:"#83aaca",treasure:"#edd17c"};
export function makeToken(kind: TokenKind, index: number): BoardToken {
  return {id:crypto.randomUUID(),name:({player:"Adventurer",npc:"NPC",monster:"Monster",object:"Object",treasure:"Treasure"}[kind])+" "+index,
    kind,x:2,y:2,size:1,color:colors[kind],sprite:null,characterId:null,notes:"",states:[],hp:null,maxHp:null,
    hidden:false,locked:false,rotation:0,brightLight:0,dimLight:0};
}
export const clamp = (n:number, low:number, high:number) => Math.max(low,Math.min(high,n));
export function positionToken(board: SceneBoard, token: BoardToken, x: number, y: number): Point {
  return {x:clamp(board.snap?Math.round(x):x,0,board.columns-token.size),
    y:clamp(board.snap?Math.round(y):y,0,board.rows-token.size)};
}
export function resizeBoard(board: SceneBoard, columns:number,rows:number) {
  board.columns=clamp(Math.round(columns),8,100);board.rows=clamp(Math.round(rows),8,100);
  for(const token of board.tokens)Object.assign(token,positionToken(board,token,token.x,token.y));
  for(const drawing of board.drawings)for(const p of drawing.points){p.x=clamp(p.x,0,board.columns);p.y=clamp(p.y,0,board.rows);}
  for(const p of board.fog){p.x=clamp(p.x,0,board.columns);p.y=clamp(p.y,0,board.rows);}
}
export const movementFeet = (a:Point,b:Point) => Math.max(Math.abs(b.x-a.x),Math.abs(b.y-a.y))*5;
export function fogRevealed(board:SceneBoard,p:Point):boolean {
  let revealed=!board.fogEnabled;
  if(board.fogEnabled)for(const mark of board.fog){if(Math.hypot(p.x-mark.x,p.y-mark.y)<=mark.radius)revealed=mark.reveal;}
  return revealed;
}
export const SKETCHES = [
  {id:"builtin:pond",name:"Pondside sketch",description:"Water, a footbridge, and a woodland path"},
  {id:"builtin:forest",name:"Forest sketch",description:"A clearing surrounded by trees"},
  {id:"builtin:sewer",name:"Dungeon sketch",description:"Stone chambers and a central channel"}
] as const;
