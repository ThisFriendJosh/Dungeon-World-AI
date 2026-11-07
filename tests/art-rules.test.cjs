const {test}=require("node:test");
const assert=require("node:assert/strict"),fs=require("node:fs"),path=require("node:path"),vm=require("node:vm");
const {createRequire}=require("node:module"),uiRequire=createRequire(path.resolve("apps/vtt-ui/package.json"));
const catalog=JSON.parse(fs.readFileSync("packages/shared/assets/catalog.json","utf8"));
test("changing campaigns clears a previously opened compendium before rendering",()=>{
 const {Compendium}=bundle("compendium.ts"),content=new Compendium();
 content.selectCampaign("campaign-a");
 content.detail={kind:"items",sourceEdition:"unassigned",entry:{name:"Private campaign relic",description:"A secret hook",properties:{},book:"Campaign A",publisher:"Table"}};
 assert.ok(content.html().includes("Private campaign relic"));
 content.selectCampaign("campaign-b");
 assert.ok(!content.html().includes("Private campaign relic"));assert.ok(!content.html().includes("A secret hook"));
});
function bundle(file){const result=uiRequire("esbuild").buildSync({entryPoints:["apps/vtt-ui/src/"+file],bundle:true,platform:"node",format:"cjs",write:false,alias:{"@tauri-apps/api/core":path.resolve("tests/tauri-render-stub.cjs")}});const fixture={exports:{}};vm.runInNewContext(result.outputFiles[0].text,{module:fixture,exports:fixture.exports,crypto:globalThis.crypto,structuredClone,console});return fixture.exports;}
test("every supplied source maps to local media and presentation crops stay in bounds",()=>{
 assert.equal(catalog.sources.length,62);assert.equal(new Set(catalog.assets.map(a=>a.id)).size,catalog.assets.length);
 for(const asset of catalog.assets){
  assert.match(asset.path,/^\/art\/art-[a-f0-9]{20}\.webp$/);assert.ok(fs.existsSync("apps/vtt-ui/public"+asset.path));
  assert.equal(asset.mechanics,"reference-only");
  assert.equal(asset.sourceSha256,catalog.sources[asset.sourceIndex].sha256);
  if(asset.crop){const[x,y,w,h]=asset.crop;assert.ok(x>=0&&y>=0&&w>0&&h>0&&x+w<=asset.width&&y+h<=asset.height);}
 }
 assert.equal(new Set(catalog.assets.map(a=>a.path)).size,61);
 for(const source of catalog.sources)assert.ok(catalog.assets.some(a=>a.id===source.assetId));
 const declared=fs.readFileSync("apps/vtt-ui/src/bundled-art-data.ts","utf8").replace(/\r\n/g,"\n").replace(/^.*\nexport const bundledArt = /,"").replace(/ as const;\n$/,"");
 assert.deepEqual(JSON.parse(declared),catalog.assets);
});
test("secret floor plans are reference views and cannot become backgrounds through the art picker",()=>{
 const {SceneEditor}=bundle("tabletop.ts"),editor=new SceneEditor();
 const source=catalog.sources.find(s=>s.index===9),art=catalog.assets.find(a=>a.id===source.assetId);
 assert.equal(art.role,"reference");assert.equal(editor.useBundledBackground(art.id),false);assert.equal(editor.dirty,false);
 const {ArtLibrary}=bundle("art-library.ts"),html=new ArtLibrary().html();
 assert.ok(html.includes('data-view-art="'+art.id+'"'));assert.ok(!html.includes('data-use-background="'+art.id+'"'));
});
test("placing a portrait uses a new piece without assigning illustrated statistics",()=>{
 const {SceneEditor}=bundle("tabletop.ts"),editor=new SceneEditor();
 const portrait=catalog.assets.find(a=>a.sourceIndex===49&&a.role==="portrait");assert.ok(editor.addBundledToken(portrait.id));
 const html=editor.html();assert.ok(html.includes(portrait.title));assert.ok(html.includes('data-token-field="hp" type="number" min="0" max="9999" value=""'));
 assert.equal(editor.dirty,true);
});
test("rules and imported reference entries escape supplied descriptions",()=>{
 const {RulesPanel}=bundle("rules-panel.ts"),rules=new RulesPanel();
 rules.load({config:{schemaVersion:1,profile:"5e-2014",houseRules:{flankingAdvantage:false,potionAsBonusAction:true,critMaxOneDie:false}},revision:1,updatedAt:"",sourceVersion:"SRD-5.1"});
 rules.book={version:"5.2.1",toc:[],pages:[{number:5,text:"<script>unsafe</script>"}]};
 const html=rules.html();assert.ok(html.includes("Your campaign uses 2014 rules"));assert.ok(html.includes("&lt;script&gt;unsafe&lt;/script&gt;"));assert.ok(!html.includes("<script>unsafe"));
 const {Compendium}=bundle("compendium.ts"),content=new Compendium();
 content.detail={kind:"spells",sourceEdition:"unassigned",entry:{name:"<img>",description:"<script>unsafe</script>",properties:{"<b>":"<svg onload=bad>"},book:"<i>",publisher:"<u>"}};
 const entry=content.html();assert.ok(!entry.includes("<script>unsafe"));assert.ok(!entry.includes("<svg onload"));assert.ok(entry.includes("&lt;svg onload=bad&gt;"));
});
