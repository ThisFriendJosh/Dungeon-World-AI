const { test } = require("node:test");
const assert = require("node:assert/strict");
const { makeScene, makeSceneCard, copyScene } = require("../.cache/adapter-tests/fari/scene.js");
const { escapeHtml } = require("../.cache/adapter-tests/html.js");

test("editing a draft cannot change saved public or GM notes", () => {
  const saved = makeScene();
  saved.indexCards.public.push(makeSceneCard());
  saved.indexCards.private.push(makeSceneCard());
  saved.indexCards.public[0].blocks[0].value = "Public description";
  saved.indexCards.private[0].blocks[0].value = "Hidden clue";
  const draft = copyScene(saved);
  draft.indexCards.public[0].blocks[0].value = "Unsaved edit";
  draft.indexCards.private[0].blocks[0].value = "Unsaved GM edit";
  draft.indexCards.public.splice(0, 1);
  assert.equal(saved.indexCards.public.length, 1);
  assert.equal(saved.indexCards.public[0].blocks[0].value, "Public description");
  assert.equal(saved.indexCards.private[0].blocks[0].value, "Hidden clue");
});
test("independent scenes never share card collections or identifiers", () => {
  const a = makeScene(), b = makeScene();
  const card = makeSceneCard();
  a.indexCards.public.push(card);
  assert.equal(b.indexCards.public.length, 0);
  assert.notEqual(a.id, b.id);
  assert.notEqual(card.id, card.blocks[0].id);
});
test("card text is escaped before entering the HTML template", () => {
  const input = '<img src=x onerror="window.injected=true"> & <script>alert(1)</script>';
  const result = escapeHtml(input);
  assert.ok(!result.includes("<"));
  assert.ok(!result.includes(">"));
  assert.ok(!result.includes('"'));
  assert.ok(result.includes("&lt;img"));
  assert.ok(result.includes("&amp;"));
});
