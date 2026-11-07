const {test}=require("node:test");
const assert=require("node:assert/strict");
const {makeBoard,makeToken,positionToken,resizeBoard,movementFeet,fogRevealed}=require("../.cache/adapter-tests/board-model.js");
const {renderBoard}=require("../.cache/adapter-tests/board-render.js");
test("five-foot movement snaps and keeps large tokens fully on the map",()=>{
 const b=makeBoard(),t=makeToken("monster",1);t.size=3;
 assert.deepEqual(positionToken(b,t,29.8,-1),{x:27,y:0});
 assert.equal(movementFeet({x:2,y:3},{x:5,y:6}),15);
 b.snap=false;assert.deepEqual(positionToken(b,t,2.2,4.8),{x:2.2,y:4.8});
});
test("resizing keeps pieces, fog, and drawings inside the board",()=>{
 const b=makeBoard(),t=makeToken("player",1);t.x=28;t.y=18;t.size=2;b.tokens.push(t);
 b.fog.push({id:"fog",x:29,y:19,radius:2,reveal:true});
 b.drawings.push({id:"stroke",kind:"pen",color:"#abcdef",width:.1,points:[{x:25,y:19},{x:30,y:20}]});
 resizeBoard(b,8,8);assert.deepEqual({x:t.x,y:t.y},{x:6,y:6});assert.equal(b.fog[0].x,8);
 assert.equal(b.drawings[0].points[1].y,8);
});
test("later conceal strokes cover earlier reveals without changing stored strokes",()=>{
 const b=makeBoard();b.fogEnabled=true;
 b.fog.push({id:"a",x:3,y:3,radius:2,reveal:true},{id:"b",x:3,y:3,radius:.5,reveal:false});
 assert.equal(fogRevealed(b,{x:3,y:3}),false);assert.equal(fogRevealed(b,{x:4,y:3}),true);
 assert.equal(fogRevealed(b,{x:9,y:9}),false);
 b.fogEnabled=false;assert.equal(fogRevealed(b,{x:9,y:9}),true);assert.equal(b.fog.length,2);
});
test("player map preview omits hidden sprites and escapes public labels",()=>{
 const b=makeBoard(),hidden=makeToken("monster",1),publicToken=makeToken("npc",2);
 hidden.name="Secret guardian";hidden.hidden=true;
 publicToken.name='<img src=x onerror="attack()">';publicToken.notes="Private notes are not canvas labels";
 b.tokens.push(hidden,publicToken);
 const player=renderBoard(b,null,true,()=>null);
 assert.ok(!player.includes("Secret guardian"));assert.ok(!player.includes("Private notes"));
 assert.ok(!player.includes("<img"));assert.ok(player.includes("&lt;img"));
 assert.ok(renderBoard(b,hidden.id,false,()=>null).includes("Secret guardian"));
});
test("new boards have independent collections and the required five-foot scale",()=>{
 const a=makeBoard(),b=makeBoard();a.tokens.push(makeToken("treasure",1));
 assert.equal(b.tokens.length,0);assert.equal(a.feetPerCell,5);
 assert.equal(a.ambient,1);assert.equal(a.fogEnabled,false);
});
