const {test}=require('node:test');
const assert=require('node:assert/strict'),path=require('node:path'),vm=require('node:vm');
const {createRequire}=require('node:module');
const esbuild=createRequire(path.resolve('apps/vtt-ui/package.json'))('esbuild');
function component(file){const bundle=esbuild.buildSync({entryPoints:['apps/vtt-ui/src/'+file],bundle:true,platform:'node',format:'cjs',write:false,alias:{'@tauri-apps/api/core':path.resolve('tests/tauri-render-stub.cjs')}});const fixture={exports:{}};vm.runInNewContext(bundle.outputFiles[0].text,{module:fixture,exports:fixture.exports,TextEncoder,console});return fixture.exports;}
const {prepareTextFile,htmlText}=component('reference-import.ts');
const {replacement}=component('clipboard.ts');
const {compendiumWorkspace}=component('compendium-workspace.ts');
test('general JSON and CSV files become source text with edition and GM visibility',()=>{
  const [source]=prepareTextFile('Aerathis_notes.json',JSON.stringify({npc:'Mara Fen',armorClass:14}),'homebrew',true);
  assert.equal(source.source.sourceRef,'Aerathis_notes.json');assert.ok(source.source.text.includes('Mara Fen'));assert.equal(source.source.edition,'homebrew');assert.equal(source.source.gmOnly,true);assert.equal(source.collection,undefined);
  const [csv]=prepareTextFile('towns.csv','name,population\nMournwell,450','unassigned',false);assert.equal(csv.source.gmOnly,false);assert.ok(csv.source.text.includes('Mournwell,450'));
});
test('source packs retain individual metadata and notices for unreadable source files',()=>{
  const result=prepareTextFile('pack.json',JSON.stringify({schemaVersion:1,sources:[{title:'Enki',text:'The hidden city',edition:'homebrew',gmOnly:true,sourceRef:'city.docx'}],notIndexed:[{source:'scan.pdf',reason:'Needs OCR'}]}));
  assert.equal(result[0].source.sourceRef,'city.docx');assert.ok(result[0].warnings[0].includes('Needs OCR'));
  assert.throws(()=>prepareTextFile('pack.json','{"schemaVersion":1,"sources":[{"title":"Bad"}]}'),/title|text/);
});
test('known dnd-data arrays remain structured collections and arbitrary JSON filenames are accepted',()=>{
  const result=prepareTextFile('monsters.json','[{"name":"Goblin"}]');assert.equal(result[0].collection.kind,'monsters');
  assert.equal(prepareTextFile('my-monsters.json','[{"name":"Goblin"}]')[0].collection,undefined);
  assert.throws(()=>prepareTextFile('broken.json','{bad'),/invalid JSON/);
});
test('HTML is converted as text with no browser document, executable markup or resource fetch',()=>{
  const html='<html><style>secret-style</style><script>bad()</script><h1>Aerathis &amp; Enki</h1><p>AC &#49;&#52;</p><img src="remote"><p>Sunshard</p></html>';
  const text=htmlText(html);assert.ok(text.includes('Aerathis & Enki'));assert.ok(text.includes('AC 14'));assert.ok(!text.includes('bad()'));assert.ok(!text.includes('remote'));assert.ok(!text.includes('secret-style'));
  assert.equal(prepareTextFile('guide.html',html)[0].source.text,text);
});
test('clipboard insertion replaces the selection, preserves surrounding text and respects field limits',()=>{
  const result=replacement('I ask XX about Enki',6,8,'Mara',4000);assert.equal(result.value,'I ask Mara about Enki');assert.equal(result.cursor,10);assert.equal(result.truncated,false);
  const limited=replacement('12345',2,4,'abcdef',6);assert.equal(limited.value,'12abc5');assert.equal(limited.truncated,true);
  assert.equal(replacement('12',2,2,'🛡️',3).value,'12');
});
test('Compendium contains the document library and reference collections as two views',()=>{
  const documents=compendiumWorkspace('documents','<p>Campaign lore</p>','<p>Spells</p>');assert.ok(documents.includes('DM library'));assert.ok(documents.includes('Campaign lore'));assert.ok(!documents.includes('<p>Spells</p>'));
  const entries=compendiumWorkspace('entries','<p>Campaign lore</p>','<p>Spells</p>');assert.ok(entries.includes('<p>Spells</p>'));assert.ok(!entries.includes('<p>Campaign lore</p>'));
});
