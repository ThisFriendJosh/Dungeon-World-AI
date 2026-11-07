const {test}=require("node:test");
const assert=require("node:assert/strict"),path=require("node:path"),vm=require("node:vm");
const {createRequire}=require("node:module");
const uiRequire=createRequire(path.resolve("apps/vtt-ui/package.json"));
const esbuild=uiRequire("esbuild");
const result=esbuild.buildSync({entryPoints:["apps/vtt-ui/src/tabletop.ts"],bundle:true,platform:"node",format:"cjs",write:false,alias:{"@tauri-apps/api/core":path.resolve("tests/tauri-render-stub.cjs")}});
const moduleFixture={exports:{}};
vm.runInNewContext(result.outputFiles[0].text,{module:moduleFixture,exports:moduleFixture.exports,crypto:globalThis.crypto,structuredClone,console});
const {SceneEditor}=moduleFixture.exports;
const {makeScene,makeSceneCard}=require("../.cache/adapter-tests/fari/scene.js");
const {makeToken}=require("../.cache/adapter-tests/board-model.js");
test("legacy card-only scenes gain a map without mutating their saved data",()=>{
 const scene=makeScene();delete scene.board;
 const card=makeSceneCard();card.title="Gatekeeper";card.blocks[0].value="The old secret";scene.indexCards.private.push(card);
 const before=JSON.stringify(scene),editor=new SceneEditor();editor.load({scene,revision:4,updatedAt:null});
 const html=editor.html();
 assert.ok(html.includes("each square 5 feet"));assert.ok(html.includes("GM notes (1)"));
 assert.equal(JSON.stringify(scene),before);assert.equal(editor.dirty,false);
});
test("table inspector escapes token names, state, notes, and imported image names",()=>{
 const scene=makeScene(),token=makeToken("npc",1);token.name='<script>alert("name")</script>';token.notes="<b>secret</b>";scene.board.tokens.push(token);
 const editor=new SceneEditor();editor.load({scene,revision:1,updatedAt:null},[],[{id:"fixture",name:'<img onerror="bad">',width:1,height:1}]);
 const html=editor.html();
 assert.ok(!html.includes("<script>"));assert.ok(!html.includes("<img onerror"));
 assert.ok(html.includes("&lt;script&gt;"));assert.ok(html.includes("&lt;img onerror"));
});
test("embedded dice has its own form outside the scene save form",()=>{
 const editor=new SceneEditor(),html=editor.html([], '<section><form id="diceForm"><button>Roll</button></form></section>');
 const sceneFormEnd=html.indexOf("</form>"),diceForm=html.indexOf('id="diceForm"');
 assert.ok(sceneFormEnd>html.indexOf('id="sceneForm"'));assert.ok(diceForm>sceneFormEnd);
 assert.ok(html.includes('id="backgroundFile"'));assert.ok(html.includes("Player preview"));
});
